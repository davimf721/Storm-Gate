//! Wine prefix management.
//!
//! Every application gets its own prefix by default so that one game's
//! configuration can never break another. A prefix is only ever deleted or
//! modified by Storm Gate when it contains `stormgate.toml`.

use std::path::{Path, PathBuf};

use stormgate_config::{time, validate_name, Paths, PrefixMetadata, PREFIX_METADATA_FILE};
use stormgate_process::{LaunchSpec, Runner, WineEnv};
use stormgate_runtime::Runtime;

#[derive(Debug, thiserror::Error)]
pub enum PrefixError {
    #[error("invalid prefix name '{0}': {1}")]
    InvalidName(String, String),
    #[error("prefix '{0}' already exists")]
    Exists(String),
    #[error("prefix '{0}' does not exist")]
    NotFound(String),
    #[error("{0} is not managed by Storm Gate (no {PREFIX_METADATA_FILE}); refusing to touch it")]
    Unmanaged(PathBuf),
    #[error("{step} failed with exit code {code}; see {log}")]
    WineFailed {
        step: &'static str,
        code: i32,
        log: String,
    },
    #[error("{0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("{0}")]
    Metadata(String),
}

pub type Result<T> = std::result::Result<T, PrefixError>;

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> PrefixError + '_ {
    move |e| PrefixError::Io(path.to_path_buf(), e)
}

/// A managed prefix on disk.
#[derive(Debug, Clone)]
pub struct Prefix {
    pub path: PathBuf,
    pub metadata: PrefixMetadata,
}

impl Prefix {
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn drive_c(&self) -> PathBuf {
        self.path.join("drive_c")
    }

    /// Whether the files `wineboot` creates are all present.
    pub fn is_initialized(&self) -> bool {
        ["drive_c/windows", "system.reg", "user.reg"]
            .iter()
            .all(|p| self.path.join(p).exists())
    }
}

/// Inspection report for `stormgate prefix inspect`.
#[derive(Debug, Clone)]
pub struct PrefixReport {
    pub prefix: Prefix,
    pub initialized: bool,
    pub size_bytes: u64,
    pub missing: Vec<&'static str>,
}

pub struct PrefixManager<'a> {
    paths: &'a Paths,
    runner: &'a dyn Runner,
}

impl<'a> PrefixManager<'a> {
    pub fn new(paths: &'a Paths, runner: &'a dyn Runner) -> Self {
        Self { paths, runner }
    }

    fn checked_path(&self, name: &str) -> Result<PathBuf> {
        validate_name(name).map_err(|e| PrefixError::InvalidName(name.into(), e))?;
        Ok(self.paths.prefix(name))
    }

    pub fn exists(&self, name: &str) -> bool {
        self.checked_path(name)
            .map(|p| p.join(PREFIX_METADATA_FILE).is_file())
            .unwrap_or(false)
    }

    pub fn open(&self, name: &str) -> Result<Prefix> {
        let path = self.checked_path(name)?;
        if !path.exists() {
            return Err(PrefixError::NotFound(name.into()));
        }
        let meta_file = path.join(PREFIX_METADATA_FILE);
        if !meta_file.is_file() {
            return Err(PrefixError::Unmanaged(path));
        }
        let metadata =
            PrefixMetadata::load(&meta_file).map_err(|e| PrefixError::Metadata(e.to_string()))?;
        Ok(Prefix { path, metadata })
    }

    pub fn list(&self) -> Result<Vec<Prefix>> {
        let dir = self.paths.prefixes();
        let mut out = Vec::new();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(PrefixError::Io(dir, e)),
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Ok(p) = self.open(&name) {
                out.push(p);
            }
        }
        out.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        Ok(out)
    }

    /// Creates and initializes a prefix with `wineboot --init`, then waits
    /// for `wineserver` to exit so the registry is fully flushed.
    ///
    /// On failure the half-created prefix is removed.
    pub fn create(
        &self,
        name: &str,
        runtime: &Runtime,
        appid: Option<u64>,
        log: &Path,
    ) -> Result<Prefix> {
        let path = self.checked_path(name)?;
        if path.exists() {
            return Err(PrefixError::Exists(name.into()));
        }
        std::fs::create_dir_all(&path).map_err(io_err(&path))?;
        let mut metadata = PrefixMetadata::new(name, runtime.version(), time::unix_now());
        metadata.appid = appid;
        let prefix = Prefix { path, metadata };
        let result = self
            .write_metadata(&prefix)
            .and_then(|_| self.wineboot(&prefix, runtime, "--init", log));
        if let Err(e) = result {
            let _ = std::fs::remove_dir_all(&prefix.path);
            return Err(e);
        }
        Ok(prefix)
    }

    /// Re-runs `wineboot --update` with the given runtime and records it.
    pub fn repair(&self, name: &str, runtime: &Runtime, log: &Path) -> Result<Prefix> {
        let mut prefix = self.open(name)?;
        self.wineboot(&prefix, runtime, "--update", log)?;
        prefix.metadata.runtime = runtime.version().to_string();
        self.write_metadata(&prefix)?;
        Ok(prefix)
    }

    /// Deletes a managed prefix.
    pub fn delete(&self, name: &str) -> Result<()> {
        let prefix = self.open(name)?;
        std::fs::remove_dir_all(&prefix.path).map_err(io_err(&prefix.path))
    }

    /// Copies a prefix (symlinks such as `dosdevices/c:` are preserved).
    pub fn clone_prefix(&self, source: &str, dest: &str) -> Result<Prefix> {
        let src = self.open(source)?;
        let dest_path = self.checked_path(dest)?;
        if dest_path.exists() {
            return Err(PrefixError::Exists(dest.into()));
        }
        if let Err(e) = copy_tree(&src.path, &dest_path) {
            let _ = std::fs::remove_dir_all(&dest_path);
            return Err(e);
        }
        let mut metadata = src.metadata.clone();
        metadata.name = dest.into();
        metadata.created_at = time::unix_now();
        metadata.cloned_from = Some(source.into());
        let prefix = Prefix {
            path: dest_path,
            metadata,
        };
        self.write_metadata(&prefix)?;
        Ok(prefix)
    }

    pub fn inspect(&self, name: &str) -> Result<PrefixReport> {
        let prefix = self.open(name)?;
        let missing = ["drive_c/windows", "dosdevices", "system.reg", "user.reg"]
            .into_iter()
            .filter(|p| !prefix.path.join(p).exists())
            .collect();
        Ok(PrefixReport {
            initialized: prefix.is_initialized(),
            size_bytes: dir_size(&prefix.path),
            missing,
            prefix,
        })
    }

    fn write_metadata(&self, prefix: &Prefix) -> Result<()> {
        prefix
            .metadata
            .save(&prefix.path.join(PREFIX_METADATA_FILE))
            .map_err(|e| PrefixError::Metadata(e.to_string()))
    }

    fn wineboot(&self, prefix: &Prefix, runtime: &Runtime, mode: &str, log: &Path) -> Result<()> {
        let env = WineEnv::new(runtime, &prefix.path).build();
        let boot = LaunchSpec::new(runtime.wine())
            .args(["wineboot", mode])
            .env(env.clone())
            .log_file(log);
        let code = self.runner.run(&boot).map_err(io_err(&runtime.wine()))?;
        if code != 0 {
            return Err(PrefixError::WineFailed {
                step: "wineboot",
                code,
                log: log.display().to_string(),
            });
        }
        let wait = LaunchSpec::new(runtime.wineserver())
            .arg("--wait")
            .env(env)
            .log_file(log);
        let code = self
            .runner
            .run(&wait)
            .map_err(io_err(&runtime.wineserver()))?;
        if code != 0 {
            return Err(PrefixError::WineFailed {
                step: "wineserver --wait",
                code,
                log: log.display().to_string(),
            });
        }
        Ok(())
    }
}

fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst).map_err(io_err(dst))?;
    for entry in std::fs::read_dir(src).map_err(io_err(src))? {
        let entry = entry.map_err(io_err(src))?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let kind = entry.file_type().map_err(io_err(&from))?;
        if kind.is_symlink() {
            let target = std::fs::read_link(&from).map_err(io_err(&from))?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &to).map_err(io_err(&to))?;
            #[cfg(not(unix))]
            let _ = target;
        } else if kind.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(io_err(&from))?;
        }
    }
    Ok(())
}

fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(t) if t.is_file() => e.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use stormgate_process::RecordingRunner;

    fn runtime(tmp: &Path) -> Runtime {
        let root = tmp.join("rt");
        std::fs::create_dir_all(root.join("wine/bin")).unwrap();
        std::fs::write(
            root.join("runtime.toml"),
            "[runtime]\nversion = \"0.0.1\"\n",
        )
        .unwrap();
        Runtime::open(&root).unwrap()
    }

    #[test]
    fn create_runs_wineboot_and_writes_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("home"));
        let rt = runtime(tmp.path());
        let runner = RecordingRunner::default();
        let mgr = PrefixManager::new(&paths, &runner);
        let log = tmp.path().join("boot.log");

        let p = mgr.create("test", &rt, Some(42), &log).unwrap();
        assert_eq!(p.metadata.appid, Some(42));
        assert!(p.path.join(PREFIX_METADATA_FILE).is_file());

        let launches = runner.launches.borrow();
        assert_eq!(launches.len(), 2);
        assert_eq!(launches[0].args, vec!["wineboot", "--init"]);
        assert_eq!(launches[0].env["WINEPREFIX"], p.path.display().to_string());
        assert!(launches[1].program.ends_with("wineserver"));
        assert_eq!(launches[1].args, vec!["--wait"]);
        drop(launches);

        assert!(matches!(
            mgr.create("test", &rt, None, &log),
            Err(PrefixError::Exists(_))
        ));
        assert!(matches!(
            mgr.create("../evil", &rt, None, &log),
            Err(PrefixError::InvalidName(..))
        ));
        assert_eq!(mgr.list().unwrap().len(), 1);
    }

    #[test]
    fn failed_wineboot_cleans_up() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("home"));
        let rt = runtime(tmp.path());
        let runner = RecordingRunner::default();
        runner.exit_codes.borrow_mut().push(1);
        let mgr = PrefixManager::new(&paths, &runner);
        let err = mgr
            .create("broken", &rt, None, &tmp.path().join("l"))
            .unwrap_err();
        assert!(matches!(
            err,
            PrefixError::WineFailed {
                step: "wineboot",
                ..
            }
        ));
        assert!(!paths.prefix("broken").exists());
    }

    #[test]
    fn clone_delete_and_unmanaged_guard() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("home"));
        let rt = runtime(tmp.path());
        let runner = RecordingRunner::default();
        let mgr = PrefixManager::new(&paths, &runner);
        let src = mgr
            .create("base", &rt, None, &tmp.path().join("l"))
            .unwrap();
        std::fs::create_dir_all(src.path.join("drive_c/windows")).unwrap();
        std::fs::create_dir_all(src.path.join("dosdevices")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../drive_c", src.path.join("dosdevices/c:")).unwrap();

        let copy = mgr.clone_prefix("base", "copy").unwrap();
        assert_eq!(copy.metadata.cloned_from.as_deref(), Some("base"));
        #[cfg(unix)]
        assert_eq!(
            std::fs::read_link(copy.path.join("dosdevices/c:")).unwrap(),
            Path::new("../drive_c")
        );
        let report = mgr.inspect("copy").unwrap();
        assert!(!report.initialized);
        assert!(report.missing.contains(&"system.reg"));

        mgr.delete("copy").unwrap();
        assert!(!paths.prefix("copy").exists());

        std::fs::create_dir_all(paths.prefix("foreign")).unwrap();
        assert!(matches!(
            mgr.delete("foreign"),
            Err(PrefixError::Unmanaged(_))
        ));
        assert!(paths.prefix("foreign").exists());
    }

    #[test]
    fn repair_updates_runtime_version() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path().join("home"));
        let rt = runtime(tmp.path());
        let runner = RecordingRunner::default();
        let mgr = PrefixManager::new(&paths, &runner);
        mgr.create("p", &rt, None, &tmp.path().join("l")).unwrap();
        let p = mgr.repair("p", &rt, &tmp.path().join("l")).unwrap();
        assert_eq!(p.metadata.runtime, "0.0.1");
        assert_eq!(
            runner.launches.borrow()[2].args,
            vec!["wineboot", "--update"]
        );
    }
}
