//! Per-launch log sessions.
//!
//! ```text
//! logs/2026-10-04/app-1091500-143015/
//! ├── launcher.log   Storm Gate decisions (profile, backend, command line)
//! ├── wine.log       Wine stdout/stderr
//! ├── graphics/      backend logs (DXVK/DXMT/VKD3D) when --debug-graphics
//! └── system.json    host and runtime snapshot
//! ```

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use stormgate_config::{time, validate_name, Paths};

use crate::redact::Redactor;
use crate::system::SystemInfo;

/// Files larger than this are left out of bundles.
const MAX_BUNDLED_FILE: u64 = 64 << 20;

#[derive(Debug, Clone)]
pub struct LogSession {
    dir: PathBuf,
}

impl LogSession {
    /// Creates `logs/<date>/<label>-<time>/`.
    pub fn create(paths: &Paths, label: &str) -> io::Result<Self> {
        validate_name(label).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let now = time::unix_now();
        let day = paths.logs().join(time::date_string(now));
        let base = format!("{label}-{}", time::time_string(now));
        let mut dir = day.join(&base);
        let mut n = 1;
        while dir.exists() {
            n += 1;
            dir = day.join(format!("{base}-{n}"));
        }
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub fn graphics_dir(&self) -> io::Result<PathBuf> {
        let d = self.dir.join("graphics");
        fs::create_dir_all(&d)?;
        Ok(d)
    }

    /// Appends a timestamped line to `launcher.log`.
    pub fn log(&self, line: &str) {
        let path = self.file("launcher.log");
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "[{}] {line}", time::rfc3339(time::unix_now()));
        }
    }

    pub fn write_system(&self, info: &SystemInfo) -> io::Result<()> {
        let json = serde_json::to_string_pretty(info).map_err(io::Error::other)?;
        fs::write(self.file("system.json"), json)
    }
}

fn session_dirs(paths: &Paths) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(days) = fs::read_dir(paths.logs()) else {
        return out;
    };
    for day in days.flatten().filter(|d| d.path().is_dir()) {
        if let Ok(sessions) = fs::read_dir(day.path()) {
            out.extend(sessions.flatten().map(|s| s.path()).filter(|p| p.is_dir()));
        }
    }
    // date/label-HHMMSS: sort by (date, time suffix) via mtime as tiebreaker.
    out.sort_by_key(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default())
    });
    out
}

fn matches_label(dir: &Path, label: &str) -> bool {
    dir.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix(label))
        .is_some_and(|rest| rest.starts_with('-'))
}

/// Most recent session, optionally restricted to a label (`app-1091500`).
pub fn last_session(paths: &Paths, label: Option<&str>) -> Option<PathBuf> {
    session_dirs(paths)
        .into_iter()
        .rfind(|d| label.is_none_or(|l| matches_label(d, l)))
}

/// Writes a sanitized zip of the latest session for `label` (or the latest
/// session overall) to `out`. Returns the number of files included.
pub fn bundle(
    paths: &Paths,
    label: Option<&str>,
    out: &Path,
    redactor: &Redactor,
) -> io::Result<usize> {
    let session = last_session(paths, label)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no log session found"))?;
    let file = File::create(out)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0;
    let root_name = session
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "session".into());
    let mut stack = vec![session.clone()];
    while let Some(dir) = stack.pop() {
        let mut entries: Vec<_> = fs::read_dir(&dir)?.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            let meta = fs::symlink_metadata(&path)?;
            if meta.is_dir() {
                stack.push(path);
                continue;
            }
            if !meta.is_file() || meta.len() > MAX_BUNDLED_FILE {
                continue;
            }
            let rel = path.strip_prefix(&session).unwrap_or(&path);
            let name = format!("{root_name}/{}", rel.to_string_lossy());
            let bytes = fs::read(&path)?;
            let data = match String::from_utf8(bytes) {
                Ok(text) => redactor.apply(&text).into_bytes(),
                // Binary files (e.g. dumps) may contain anything; skip them.
                Err(_) => continue,
            };
            zip.start_file(redactor.apply(&name), options)
                .map_err(io::Error::other)?;
            zip.write_all(&data)?;
            count += 1;
        }
    }
    zip.finish().map_err(io::Error::other)?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn session_lifecycle_and_bundle() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        let a = LogSession::create(&paths, "app-70").unwrap();
        a.log("starting");
        fs::write(a.file("wine.log"), "token=supersecret\n").unwrap();
        fs::write(a.file("dump.bin"), [0xff, 0xfe, 0x00]).unwrap();
        let b = LogSession::create(&paths, "run-notepad").unwrap();
        b.log("hello");
        assert!(LogSession::create(&paths, "../x").is_err());

        assert_eq!(last_session(&paths, Some("app-70")).unwrap(), a.dir());
        assert!(last_session(&paths, Some("app-7")).is_none());
        assert!(last_session(&paths, None).is_some());

        let out = tmp.path().join("report.zip");
        let n = bundle(&paths, Some("app-70"), &out, &Redactor::new(None, None)).unwrap();
        assert_eq!(n, 2, "binary dump must be skipped");
        let mut archive = zip::ZipArchive::new(File::open(&out).unwrap()).unwrap();
        let mut found = false;
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).unwrap();
            if f.name().ends_with("wine.log") {
                let mut s = String::new();
                f.read_to_string(&mut s).unwrap();
                assert!(!s.contains("supersecret"));
                found = true;
            }
        }
        assert!(found);
    }
}
