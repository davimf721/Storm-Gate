//! Compatibility database and automatic detection.
//!
//! The database is versioned in Git (`compat/steam/<appid>/`):
//!
//! ```text
//! compat/steam/1091500/
//! ├── profile.toml   game profile (shared, open stack only)
//! ├── status.json    rating and per-area test results
//! └── notes.md       human notes
//! ```
//!
//! Profile resolution order: user profile (`games/<appid>.toml`, written only
//! after the user confirms) > database profile > automatic detection.

mod anticheat;
mod db;
mod detect;

pub use anticheat::{scan_anticheat, AntiCheat, AntiCheatKind};
pub use db::{AreaResult, CompatDb, CompatEntry, Rating, Status};
pub use detect::{detect, guess_executable, suggest_profile, Detection};

use std::path::Path;
use stormgate_config::{GameProfile, Paths, ValidationMode};

#[derive(Debug, thiserror::Error)]
pub enum CompatError {
    #[error("{0}: {1}")]
    Io(std::path::PathBuf, #[source] std::io::Error),
    #[error("{0}: {1}")]
    Invalid(std::path::PathBuf, String),
}

pub type Result<T> = std::result::Result<T, CompatError>;

/// Where a resolved profile came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSource {
    User,
    Database,
    None,
}

impl ProfileSource {
    pub fn label(self) -> &'static str {
        match self {
            ProfileSource::User => "user profile",
            ProfileSource::Database => "compatibility database",
            ProfileSource::None => "automatic detection",
        }
    }
}

/// Path of a user-confirmed profile for a Steam AppID.
pub fn user_profile_path(paths: &Paths, appid: u64) -> std::path::PathBuf {
    paths.games().join(format!("{appid}.toml"))
}

/// Resolves the profile for a Steam game.
pub fn resolve_profile(
    paths: &Paths,
    db: Option<&CompatDb>,
    appid: u64,
) -> Result<(Option<GameProfile>, ProfileSource)> {
    let user = user_profile_path(paths, appid);
    if user.is_file() {
        let p = GameProfile::load(&user, ValidationMode::Local)
            .map_err(|e| CompatError::Invalid(user.clone(), e.to_string()))?;
        return Ok((Some(p), ProfileSource::User));
    }
    if let Some(entry) = db.and_then(|db| db.get(appid).transpose()).transpose()? {
        if let Some(p) = entry.profile {
            return Ok((Some(p), ProfileSource::Database));
        }
    }
    Ok((None, ProfileSource::None))
}

/// Validates every profile and status file under a database root (CI).
pub fn validate_tree(root: &Path) -> Vec<String> {
    let db = CompatDb::new(root);
    let mut errors = Vec::new();
    match db.appids() {
        Ok(ids) => {
            for id in ids {
                if let Err(e) = db.get(id) {
                    errors.push(e.to_string());
                }
            }
        }
        Err(e) => errors.push(e.to_string()),
    }
    errors
}
