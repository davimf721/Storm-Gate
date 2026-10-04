use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use stormgate_config::{GameProfile, ValidationMode};

use crate::{CompatError, Result};

/// Overall rating, same scale as ProtonDB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Rating {
    Platinum,
    Gold,
    Silver,
    Bronze,
    Broken,
    /// Cannot work by design (e.g. kernel anti-cheat). Never worked around.
    Blocked,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AreaResult {
    Pass,
    Partial,
    Fail,
    #[default]
    Untested,
}

/// Per-area results recorded for a game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Areas {
    #[serde(default)]
    pub boot: AreaResult,
    #[serde(default)]
    pub menu: AreaResult,
    #[serde(default)]
    pub gameplay: AreaResult,
    #[serde(default)]
    pub audio: AreaResult,
    #[serde(default)]
    pub input: AreaResult,
    #[serde(default)]
    pub graphics: AreaResult,
    #[serde(default)]
    pub performance: AreaResult,
    #[serde(default)]
    pub online: AreaResult,
    #[serde(default)]
    pub anti_cheat: AreaResult,
}

/// Hardware/software a result was obtained on. Results without real
/// hardware testing must not be recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestedOn {
    pub chip: String,
    pub macos: String,
    pub runtime: String,
    pub date: String,
}

/// `status.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub schema: u32,
    pub rating: Rating,
    #[serde(default)]
    pub areas: Areas,
    #[serde(default)]
    pub tested_on: Vec<TestedOn>,
    #[serde(default)]
    pub known_issues: Vec<String>,
}

impl Status {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.schema != 1 {
            errors.push(format!("unsupported status schema {}", self.schema));
        }
        let ranked = !matches!(self.rating, Rating::Broken | Rating::Blocked);
        if ranked && self.tested_on.is_empty() {
            errors.push("a playable rating requires at least one tested_on entry".into());
        }
        if ranked && self.areas.boot != AreaResult::Pass {
            errors.push("a playable rating requires areas.boot = \"pass\"".into());
        }
        errors
    }
}

/// One database entry.
#[derive(Debug, Clone)]
pub struct CompatEntry {
    pub appid: u64,
    pub dir: PathBuf,
    pub profile: Option<GameProfile>,
    pub status: Option<Status>,
}

/// Database rooted at a `compat/` directory.
#[derive(Debug, Clone)]
pub struct CompatDb {
    root: PathBuf,
}

impl CompatDb {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn steam_dir(&self) -> PathBuf {
        self.root.join("steam")
    }

    pub fn appids(&self) -> Result<Vec<u64>> {
        let dir = self.steam_dir();
        let mut ids = Vec::new();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(ids),
            Err(e) => return Err(CompatError::Io(dir, e)),
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !entry.path().is_dir() {
                continue;
            }
            match name.parse::<u64>() {
                Ok(id) => ids.push(id),
                Err(_) => {
                    return Err(CompatError::Invalid(
                        entry.path(),
                        "directory name must be a numeric Steam AppID".into(),
                    ))
                }
            }
        }
        ids.sort_unstable();
        Ok(ids)
    }

    /// Loads an entry. `Ok(None)` when the AppID has no entry.
    pub fn get(&self, appid: u64) -> Result<Option<CompatEntry>> {
        let dir = self.steam_dir().join(appid.to_string());
        if !dir.is_dir() {
            return Ok(None);
        }
        let profile_path = dir.join("profile.toml");
        let profile = if profile_path.is_file() {
            let p = GameProfile::load(&profile_path, ValidationMode::Shared)
                .map_err(|e| CompatError::Invalid(profile_path.clone(), e.to_string()))?;
            if p.game.appid != Some(appid) {
                return Err(CompatError::Invalid(
                    profile_path,
                    format!("game.appid must be {appid}"),
                ));
            }
            Some(p)
        } else {
            None
        };
        let status_path = dir.join("status.json");
        let status = if status_path.is_file() {
            Some(load_status(&status_path)?)
        } else {
            None
        };
        if profile.is_none() && status.is_none() {
            return Err(CompatError::Invalid(
                dir,
                "entry needs a profile.toml or a status.json".into(),
            ));
        }
        Ok(Some(CompatEntry {
            appid,
            dir,
            profile,
            status,
        }))
    }
}

fn load_status(path: &Path) -> Result<Status> {
    let text = std::fs::read_to_string(path).map_err(|e| CompatError::Io(path.into(), e))?;
    let status: Status = serde_json::from_str(&text)
        .map_err(|e| CompatError::Invalid(path.into(), e.to_string()))?;
    let errors = status.validate();
    if !errors.is_empty() {
        return Err(CompatError::Invalid(path.into(), errors.join("; ")));
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, text: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn loads_entries() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "steam/70/profile.toml",
            "schema = 1\n[game]\nappid = 70\nname = \"Half-Life\"\n[graphics]\nd3d9 = \"dxvk\"\n",
        );
        write(
            tmp.path(),
            "steam/70/status.json",
            r#"{"schema":1,"rating":"GOLD","areas":{"boot":"pass","menu":"pass"},
               "tested_on":[{"chip":"M1","macos":"15.6","runtime":"0.1.0","date":"2026-10-04"}]}"#,
        );
        let db = CompatDb::new(tmp.path());
        assert_eq!(db.appids().unwrap(), vec![70]);
        let e = db.get(70).unwrap().unwrap();
        assert_eq!(e.status.unwrap().rating, Rating::Gold);
        assert!(db.get(71).unwrap().is_none());
    }

    #[test]
    fn rejects_untested_ratings_and_mismatched_ids() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            tmp.path(),
            "steam/1/status.json",
            r#"{"schema":1,"rating":"PLATINUM"}"#,
        );
        write(
            tmp.path(),
            "steam/2/profile.toml",
            "schema = 1\n[game]\nappid = 3\nname = \"x\"\n",
        );
        write(
            tmp.path(),
            "steam/4/profile.toml",
            "schema = 1\n[game]\nappid = 4\nname = \"x\"\n[graphics]\nd3d11 = \"d3dmetal\"\n",
        );
        write(
            tmp.path(),
            "steam/5/status.json",
            r#"{"schema":1,"rating":"BLOCKED","known_issues":["kernel anti-cheat"]}"#,
        );
        let db = CompatDb::new(tmp.path());
        assert!(db.get(1).is_err());
        assert!(db.get(2).is_err());
        assert!(
            db.get(4).is_err(),
            "shared profiles must not use proprietary backends"
        );
        assert!(db.get(5).is_ok());
    }
}
