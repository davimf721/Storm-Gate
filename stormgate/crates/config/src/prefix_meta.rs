use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::profile::ProfileError;

/// Metadata file stored at the root of every Storm Gate managed prefix.
pub const PREFIX_METADATA_FILE: &str = "stormgate.toml";

/// Metadata describing a Wine prefix created by Storm Gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrefixMetadata {
    pub schema: u32,
    pub name: String,
    /// Unix timestamp (seconds).
    pub created_at: u64,
    /// Runtime version used to create (or last repair) the prefix.
    pub runtime: String,
    /// Windows architecture of the prefix. Only `win64` (new WoW64) for now.
    #[serde(default = "default_arch")]
    pub arch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appid: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cloned_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

fn default_arch() -> String {
    "win64".into()
}

impl PrefixMetadata {
    pub fn new(name: &str, runtime: &str, created_at: u64) -> Self {
        Self {
            schema: 1,
            name: name.to_string(),
            created_at,
            runtime: runtime.to_string(),
            arch: default_arch(),
            appid: None,
            cloned_from: None,
            notes: None,
        }
    }

    pub fn load(path: &Path) -> Result<Self, ProfileError> {
        let text = std::fs::read_to_string(path).map_err(|e| ProfileError::Io(path.into(), e))?;
        toml::from_str(&text).map_err(|e| ProfileError::Parse(path.into(), e.to_string()))
    }

    pub fn save(&self, path: &Path) -> Result<(), ProfileError> {
        let text = toml::to_string_pretty(self)
            .map_err(|e| ProfileError::Parse(path.into(), e.to_string()))?;
        std::fs::write(path, text).map_err(|e| ProfileError::Io(path.into(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join(PREFIX_METADATA_FILE);
        let mut meta = PrefixMetadata::new("test", "0.0.1", 42);
        meta.appid = Some(1_091_500);
        meta.save(&file).unwrap();
        assert_eq!(PrefixMetadata::load(&file).unwrap(), meta);
    }
}
