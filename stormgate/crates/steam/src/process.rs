//! Launch specs for the Steam client.

use std::collections::BTreeMap;
use std::path::Path;

use stormgate_process::LaunchSpec;

use crate::library::SteamInstall;
use crate::webhelper;

/// Starts the Steam client.
pub fn start_spec(
    wine: &Path,
    steam: &SteamInstall,
    env: BTreeMap<String, String>,
    extra: &[String],
) -> LaunchSpec {
    LaunchSpec::new(wine)
        .arg(steam.steam_exe().display().to_string())
        .args(webhelper::steam_args(extra))
        .cwd(steam.root())
        .env(env)
}

/// Asks Steam to launch a game (`steam.exe -applaunch <appid> [args]`).
///
/// Going through the client keeps Steamworks/DRM behaviour legitimate: the
/// game runs exactly as Steam would start it on Windows.
pub fn applaunch_spec(
    wine: &Path,
    steam: &SteamInstall,
    appid: u64,
    env: BTreeMap<String, String>,
    game_args: &[String],
) -> LaunchSpec {
    LaunchSpec::new(wine)
        .arg(steam.steam_exe().display().to_string())
        .args(webhelper::steam_args(&[]))
        .arg("-applaunch")
        .arg(appid.to_string())
        .args(game_args.iter().cloned())
        .cwd(steam.root())
        .env(env)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::STEAM_DIR_IN_PREFIX;

    #[test]
    fn applaunch() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join(STEAM_DIR_IN_PREFIX);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("steam.exe"), "").unwrap();
        let steam = SteamInstall::find(tmp.path()).unwrap();
        let spec = applaunch_spec(
            Path::new("/rt/wine"),
            &steam,
            1_091_500,
            BTreeMap::new(),
            &["-windowed".to_string()],
        );
        assert!(spec.args[0].ends_with("steam.exe"));
        assert_eq!(&spec.args[1..], &["-applaunch", "1091500", "-windowed"]);
    }
}
