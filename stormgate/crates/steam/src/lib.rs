//! Steam integration.
//!
//! Storm Gate runs the *Windows* Steam client inside a dedicated Wine prefix
//! (`prefixes/steam`). Steam-specific workarounds live here and never in the
//! Wine patch queue unless strictly necessary.
//!
//! This crate never handles credentials: the user logs in manually inside
//! the Steam window.

pub mod appinfo;
pub mod bootstrap;
pub mod library;
pub mod process;
pub mod vdf;
pub mod webhelper;

pub use appinfo::AppManifest;
pub use library::{InstalledGame, SteamInstall};

/// Name of the shared prefix that hosts the Steam client.
pub const STEAM_PREFIX: &str = "steam";

/// Steam's default install directory inside the prefix.
pub const STEAM_DIR_IN_PREFIX: &str = "drive_c/Program Files (x86)/Steam";

#[derive(Debug, thiserror::Error)]
pub enum SteamError {
    #[error("{0}: {1}")]
    Io(std::path::PathBuf, #[source] std::io::Error),
    #[error("{0}: {1}")]
    Parse(std::path::PathBuf, String),
    #[error("Steam is not installed in this prefix (missing {0}); run `stormgate steam install`")]
    NotInstalled(std::path::PathBuf),
    #[error("app {0} is not installed in any Steam library")]
    AppNotFound(u64),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SteamError>;
