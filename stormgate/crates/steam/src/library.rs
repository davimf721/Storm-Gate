use std::path::{Component, Path, PathBuf};

use crate::appinfo::AppManifest;
use crate::{vdf, Result, SteamError, STEAM_DIR_IN_PREFIX};

/// A Steam client installed inside a Wine prefix.
#[derive(Debug, Clone)]
pub struct SteamInstall {
    prefix: PathBuf,
    root: PathBuf,
}

/// An installed game resolved from an app manifest.
#[derive(Debug, Clone)]
pub struct InstalledGame {
    pub manifest: AppManifest,
    pub library: PathBuf,
    pub install_dir: PathBuf,
}

impl SteamInstall {
    /// Locates Steam in `prefix`. Fails when `steam.exe` is missing.
    pub fn find(prefix: &Path) -> Result<Self> {
        let root = prefix.join(STEAM_DIR_IN_PREFIX);
        let exe = root.join("steam.exe");
        if !exe.is_file() {
            return Err(SteamError::NotInstalled(exe));
        }
        Ok(Self {
            prefix: prefix.to_path_buf(),
            root,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn steam_exe(&self) -> PathBuf {
        self.root.join("steam.exe")
    }

    /// Library folders (host paths), starting with the Steam root.
    pub fn libraries(&self) -> Result<Vec<PathBuf>> {
        let mut libs = vec![self.root.clone()];
        let file = self.root.join("steamapps/libraryfolders.vdf");
        let text = match std::fs::read_to_string(&file) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(libs),
            Err(e) => return Err(SteamError::Io(file, e)),
        };
        let doc = vdf::parse(&text).map_err(|e| SteamError::Parse(file.clone(), e.to_string()))?;
        let Some(folders) = doc.get("libraryfolders") else {
            return Ok(libs);
        };
        for (_, entry) in folders.entries() {
            // Old format: "1" "D:\\Games"; new format: "1" { "path" "..." }.
            let path = match entry {
                vdf::Value::Str(s) => Some(s.as_str()),
                obj => obj.str("path"),
            };
            if let Some(host) = path.and_then(|p| windows_to_host(&self.prefix, p)) {
                if !libs.contains(&host) {
                    libs.push(host);
                }
            }
        }
        Ok(libs)
    }

    /// Every game with an app manifest, sorted by name.
    pub fn games(&self) -> Result<Vec<InstalledGame>> {
        let mut games = Vec::new();
        for lib in self.libraries()? {
            let steamapps = lib.join("steamapps");
            let Ok(entries) = std::fs::read_dir(&steamapps) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !(name.starts_with("appmanifest_") && name.ends_with(".acf")) {
                    continue;
                }
                if let Ok(manifest) = AppManifest::load(&entry.path()) {
                    let install_dir = steamapps.join("common").join(&manifest.installdir);
                    games.push(InstalledGame {
                        manifest,
                        library: lib.clone(),
                        install_dir,
                    });
                }
            }
        }
        games.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        Ok(games)
    }

    pub fn find_game(&self, appid: u64) -> Result<InstalledGame> {
        self.games()?
            .into_iter()
            .find(|g| g.manifest.appid == appid)
            .ok_or(SteamError::AppNotFound(appid))
    }
}

/// Maps a Windows path from inside the prefix (`D:\\Games`) to a host path.
///
/// `C:` maps to `drive_c`; other drives go through `dosdevices/<x>:`, which
/// Wine keeps as symlinks. Paths containing `..` are rejected.
pub fn windows_to_host(prefix: &Path, path: &str) -> Option<PathBuf> {
    let bytes = path.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    let drive = (bytes[0] as char).to_ascii_lowercase();
    let mut host = if drive == 'c' {
        prefix.join("drive_c")
    } else {
        prefix.join("dosdevices").join(format!("{drive}:"))
    };
    for part in path[2..].split(['\\', '/']).filter(|p| !p.is_empty()) {
        if Path::new(part)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        {
            return None;
        }
        host.push(part);
    }
    Some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_windows_paths() {
        let p = Path::new("/pfx");
        assert_eq!(
            windows_to_host(p, "C:\\Program Files (x86)\\Steam"),
            Some(PathBuf::from("/pfx/drive_c/Program Files (x86)/Steam"))
        );
        assert_eq!(
            windows_to_host(p, "d:/Games"),
            Some(PathBuf::from("/pfx/dosdevices/d:/Games"))
        );
        assert_eq!(windows_to_host(p, "C:\\..\\..\\etc"), None);
        assert_eq!(windows_to_host(p, "relative"), None);
    }

    #[test]
    fn discovers_games_across_libraries() {
        let tmp = tempfile::tempdir().unwrap();
        let prefix = tmp.path();
        let root = prefix.join(STEAM_DIR_IN_PREFIX);
        std::fs::create_dir_all(root.join("steamapps")).unwrap();
        std::fs::write(root.join("steam.exe"), "").unwrap();
        let second = prefix.join("drive_c/Games/SteamLibrary/steamapps");
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(
            root.join("steamapps/libraryfolders.vdf"),
            r#""libraryfolders" {
                "0" { "path" "C:\\Program Files (x86)\\Steam" }
                "1" { "path" "C:\\Games\\SteamLibrary" }
            }"#,
        )
        .unwrap();
        std::fs::write(
            second.join("appmanifest_1091500.acf"),
            r#""AppState" { "appid" "1091500" "name" "Zeta" "StateFlags" "4" "installdir" "Zeta" }"#,
        )
        .unwrap();
        std::fs::write(
            root.join("steamapps/appmanifest_70.acf"),
            r#""AppState" { "appid" "70" "name" "Alpha" "StateFlags" "4" "installdir" "Alpha" }"#,
        )
        .unwrap();
        std::fs::write(root.join("steamapps/appmanifest_bad.acf"), "garbage {").unwrap();

        let steam = SteamInstall::find(prefix).unwrap();
        assert_eq!(steam.libraries().unwrap().len(), 2);
        let games = steam.games().unwrap();
        let names: Vec<_> = games.iter().map(|g| g.manifest.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Zeta"]);
        let g = steam.find_game(1_091_500).unwrap();
        assert!(g
            .install_dir
            .ends_with("Games/SteamLibrary/steamapps/common/Zeta"));
        assert!(matches!(
            steam.find_game(1),
            Err(SteamError::AppNotFound(1))
        ));
    }

    #[test]
    fn missing_steam() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(matches!(
            SteamInstall::find(tmp.path()),
            Err(SteamError::NotInstalled(_))
        ));
    }
}
