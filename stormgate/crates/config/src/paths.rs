use std::env;
use std::io;
use std::path::{Path, PathBuf};

/// Environment variable that overrides the Storm Gate data root.
pub const HOME_ENV: &str = "STORMGATE_HOME";

/// On-disk layout of a Storm Gate installation.
///
/// ```text
/// ~/Library/Application Support/StormGate/
/// ├── runtimes/
/// ├── prefixes/
/// ├── games/
/// ├── cache/
/// └── logs/
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    /// Uses an explicit root directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Resolves the root from `STORMGATE_HOME` or the platform default.
    ///
    /// On macOS this is `~/Library/Application Support/StormGate`. Other
    /// platforms are only used for development, so the XDG data directory is
    /// used there.
    pub fn from_env() -> io::Result<Self> {
        if let Some(root) = env::var_os(HOME_ENV).filter(|v| !v.is_empty()) {
            return Ok(Self::new(root));
        }
        let home = env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
        if cfg!(target_os = "macos") {
            return Ok(Self::new(
                home.join("Library/Application Support/StormGate"),
            ));
        }
        let data = env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"));
        Ok(Self::new(data.join("stormgate")))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }

    pub fn prefixes(&self) -> PathBuf {
        self.root.join("prefixes")
    }

    pub fn prefix(&self, name: &str) -> PathBuf {
        self.prefixes().join(name)
    }

    /// User-confirmed game profiles (`games/<appid>.toml` or `games/<name>.toml`).
    pub fn games(&self) -> PathBuf {
        self.root.join("games")
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn downloads(&self) -> PathBuf {
        self.cache().join("downloads")
    }

    /// Shader cache directory. The key must already encode game, backend,
    /// backend version, GPU, macOS version and runtime version.
    pub fn shader_cache(&self, key: &str) -> PathBuf {
        self.cache().join("shaders").join(key)
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Creates every top-level directory.
    pub fn ensure(&self) -> io::Result<()> {
        for dir in [
            self.runtimes(),
            self.prefixes(),
            self.games(),
            self.cache(),
            self.logs(),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = Paths::new(tmp.path());
        paths.ensure().unwrap();
        for d in ["runtimes", "prefixes", "games", "cache", "logs"] {
            assert!(tmp.path().join(d).is_dir(), "{d} missing");
        }
        assert_eq!(paths.prefix("steam"), tmp.path().join("prefixes/steam"));
    }
}
