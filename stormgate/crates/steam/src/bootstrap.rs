//! Downloading and running the Windows Steam installer.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use stormgate_process::LaunchSpec;

use crate::{Result, SteamError};

/// Official Windows Steam installer. Valve updates it in place, so it cannot
/// be pinned by hash; the hash of every download is logged instead and can
/// be enforced with `--sha256`.
pub const INSTALLER_URL: &str =
    "https://cdn.cloudflare.steamstatic.com/client/installer/SteamSetup.exe";

pub const INSTALLER_NAME: &str = "SteamSetup.exe";

/// `curl` arguments for a safe download: HTTPS only (also on redirects),
/// TLS 1.2+, fail on HTTP errors, never piped into a shell.
pub fn curl_args(url: &str, dest: &Path) -> Vec<String> {
    vec![
        "--fail".into(),
        "--location".into(),
        "--proto".into(),
        "=https".into(),
        "--proto-redir".into(),
        "=https".into(),
        "--tlsv1.2".into(),
        "--silent".into(),
        "--show-error".into(),
        "--output".into(),
        dest.display().to_string(),
        url.into(),
    ]
}

/// Downloads the installer into `dir` and returns its path.
pub fn download_installer(dir: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(dir).map_err(|e| SteamError::Io(dir.to_path_buf(), e))?;
    let dest = dir.join(INSTALLER_NAME);
    let partial = dir.join(format!("{INSTALLER_NAME}.part"));
    let status = Command::new("curl")
        .args(curl_args(INSTALLER_URL, &partial))
        .status()
        .map_err(|e| SteamError::Io(PathBuf::from("curl"), e))?;
    if !status.success() {
        let _ = std::fs::remove_file(&partial);
        return Err(SteamError::Other(format!(
            "downloading {INSTALLER_URL} failed ({status})"
        )));
    }
    std::fs::rename(&partial, &dest).map_err(|e| SteamError::Io(dest.clone(), e))?;
    Ok(dest)
}

/// Silent install of Steam into the prefix's default location.
pub fn install_spec(wine: &Path, installer: &Path, env: BTreeMap<String, String>) -> LaunchSpec {
    LaunchSpec::new(wine)
        .arg(installer.display().to_string())
        .arg("/S")
        .env(env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curl_is_https_only() {
        let args = curl_args(INSTALLER_URL, Path::new("/tmp/x"));
        let joined = args.join(" ");
        assert!(joined.contains("--proto =https"));
        assert!(joined.contains("--proto-redir =https"));
        assert!(joined.contains("--fail"));
        assert_eq!(args.last().unwrap(), INSTALLER_URL);
        assert!(INSTALLER_URL.starts_with("https://"));
    }

    #[test]
    fn silent_install() {
        let spec = install_spec(
            Path::new("/rt/wine"),
            Path::new("/c/SteamSetup.exe"),
            BTreeMap::new(),
        );
        assert_eq!(spec.args, vec!["/c/SteamSetup.exe", "/S"]);
    }
}
