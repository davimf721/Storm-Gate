use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::backend::{GraphicsBackend, SyncBackend, VulkanDriver};

/// Current game profile schema version.
pub const PROFILE_SCHEMA: u32 = 1;

/// Environment variables that the runtime owns. Profiles must express these
/// through dedicated fields instead of raw environment entries.
const RESERVED_ENV: &[&str] = &[
    "WINEPREFIX",
    "WINEDLLOVERRIDES",
    "WINEDEBUG",
    "WINEMSYNC",
    "WINEESYNC",
    "WINEFSYNC",
    "DYLD_LIBRARY_PATH",
    "DYLD_FALLBACK_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "VK_ICD_FILENAMES",
    "VK_DRIVER_FILES",
];

/// Fragments that suggest an environment variable carries a credential.
const SECRET_HINTS: &[&str] = &[
    "TOKEN", "PASSWORD", "PASSWD", "SECRET", "COOKIE", "APIKEY", "API_KEY",
];

const DLL_OVERRIDE_VALUES: &[&str] = &[
    "",
    "native",
    "builtin",
    "native,builtin",
    "builtin,native",
    "n",
    "b",
    "n,b",
    "b,n",
    "disabled",
];

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("{0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
    #[error("{0}: invalid TOML: {1}")]
    Parse(PathBuf, String),
    #[error("invalid profile: {}", .0.join("; "))]
    Invalid(Vec<String>),
}

/// How strictly a profile is validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// User profiles on the local machine. Optional proprietary backends are allowed.
    Local,
    /// Profiles shared in the repository / compatibility database. Must only
    /// use the open stack.
    Shared,
}

/// Per-game configuration. See `docs/profiles.md` for the documented schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameProfile {
    pub schema: u32,
    pub game: GameSection,
    #[serde(default)]
    pub runtime: RuntimeSection,
    #[serde(default)]
    pub graphics: GraphicsSection,
    #[serde(default)]
    pub sync: SyncSection,
    #[serde(default)]
    pub display: DisplaySection,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub environment: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dll_overrides: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appid: Option<u64>,
    pub name: String,
    /// Executable relative to the game install directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSection {
    /// Wine flavour, e.g. `proton-wine-11`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wine: Option<String>,
    /// Pin a specific side-by-side runtime version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphicsSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d3d9: Option<GraphicsBackend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d3d10: Option<GraphicsBackend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d3d11: Option<GraphicsBackend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d3d12: Option<GraphicsBackend>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vulkan: Option<VulkanDriver>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncSection {
    #[serde(default)]
    pub backend: SyncBackend,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplaySection {
    #[serde(default)]
    pub virtual_desktop: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default)]
    pub retina: bool,
}

impl GameProfile {
    /// A minimal profile with only a name and optional Steam AppID.
    pub fn new(name: &str, appid: Option<u64>) -> Self {
        Self {
            schema: PROFILE_SCHEMA,
            game: GameSection {
                appid,
                name: name.to_string(),
                executable: None,
                arguments: Vec::new(),
            },
            runtime: RuntimeSection::default(),
            graphics: GraphicsSection::default(),
            sync: SyncSection::default(),
            display: DisplaySection::default(),
            environment: BTreeMap::new(),
            dll_overrides: BTreeMap::new(),
        }
    }

    pub fn from_toml(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("profile serialization cannot fail")
    }

    /// Loads and validates a profile file.
    pub fn load(path: &Path, mode: ValidationMode) -> Result<Self, ProfileError> {
        let text = std::fs::read_to_string(path).map_err(|e| ProfileError::Io(path.into(), e))?;
        let profile = Self::from_toml(&text).map_err(|e| ProfileError::Parse(path.into(), e))?;
        let errors = profile.validate(mode);
        if errors.is_empty() {
            Ok(profile)
        } else {
            Err(ProfileError::Invalid(errors))
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ProfileError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ProfileError::Io(parent.into(), e))?;
        }
        std::fs::write(path, self.to_toml()).map_err(|e| ProfileError::Io(path.into(), e))
    }

    /// Returns every problem found in the profile (empty when valid).
    pub fn validate(&self, mode: ValidationMode) -> Vec<String> {
        let mut errors = Vec::new();

        if self.schema != PROFILE_SCHEMA {
            errors.push(format!(
                "unsupported schema {} (expected {PROFILE_SCHEMA})",
                self.schema
            ));
        }
        if self.game.name.trim().is_empty() {
            errors.push("game.name must not be empty".into());
        }
        if let Some(exe) = &self.game.executable {
            if exe.starts_with('/') || exe.split(['/', '\\']).any(|c| c == "..") {
                errors.push("game.executable must be relative to the install directory".into());
            }
        }

        let g = &self.graphics;
        let check = |errors: &mut Vec<String>,
                     api: &str,
                     b: Option<GraphicsBackend>,
                     ok: &[GraphicsBackend]| {
            if let Some(b) = b {
                if !ok.contains(&b) {
                    errors.push(format!("graphics.{api} = \"{b}\" cannot implement {api}"));
                }
                if mode == ValidationMode::Shared && !b.is_open() {
                    errors.push(format!(
                        "graphics.{api} = \"{b}\" is proprietary; shared profiles must use the open stack"
                    ));
                }
            }
        };
        use GraphicsBackend::*;
        check(&mut errors, "d3d9", g.d3d9, &[Dxvk, WineD3d, D3dMetal]);
        check(
            &mut errors,
            "d3d10",
            g.d3d10,
            &[Dxmt, Dxvk, WineD3d, D3dMetal],
        );
        check(
            &mut errors,
            "d3d11",
            g.d3d11,
            &[Dxmt, Dxvk, WineD3d, D3dMetal],
        );
        check(&mut errors, "d3d12", g.d3d12, &[Vkd3dProton, D3dMetal]);

        let d = &self.display;
        if d.virtual_desktop {
            match (d.width, d.height) {
                (Some(w), Some(h))
                    if (320..=16_384).contains(&w) && (200..=16_384).contains(&h) => {}
                (None, None) => {}
                _ => errors.push(
                    "display.width and display.height must both be set to a sane resolution".into(),
                ),
            }
        } else if d.width.is_some() || d.height.is_some() {
            errors
                .push("display.width/height only apply when display.virtual_desktop = true".into());
        }

        for (key, value) in &self.environment {
            if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                errors.push(format!(
                    "environment key '{key}' is not a valid variable name"
                ));
                continue;
            }
            let upper = key.to_ascii_uppercase();
            if RESERVED_ENV.contains(&upper.as_str()) {
                errors.push(format!(
                    "environment.{key} is managed by the runtime; use the dedicated profile field"
                ));
            }
            if SECRET_HINTS.iter().any(|h| upper.contains(h)) {
                errors.push(format!(
                    "environment.{key} looks like a credential; never store secrets in profiles"
                ));
            }
            if value.contains('\n') {
                errors.push(format!("environment.{key} must be a single line"));
            }
        }

        for (dll, value) in &self.dll_overrides {
            if dll.is_empty()
                || !dll
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
            {
                errors.push(format!("dll_overrides key '{dll}' is not a valid DLL name"));
            }
            if !DLL_OVERRIDE_VALUES.contains(&value.as_str()) {
                errors.push(format!(
                    "dll_overrides.{dll} = \"{value}\" is not a valid override (use native, builtin, native,builtin, builtin,native or disabled)"
                ));
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
schema = 1

[game]
appid = 1091500
name = "Example Game"

[runtime]
wine = "proton-wine-11"

[graphics]
d3d11 = "dxmt"
d3d12 = "vkd3d-moltenvk"

[sync]
backend = "msync"

[display]
virtual_desktop = false
retina = false

[environment]
SOME_VARIABLE = "1"

[dll_overrides]
dxgi = "native,builtin"
"#;

    #[test]
    fn parses_plan_example() {
        let p = GameProfile::from_toml(EXAMPLE).unwrap();
        assert_eq!(p.game.appid, Some(1_091_500));
        assert_eq!(p.graphics.d3d11, Some(GraphicsBackend::Dxmt));
        assert_eq!(p.graphics.d3d12, Some(GraphicsBackend::Vkd3dProton));
        assert_eq!(p.sync.backend, SyncBackend::Msync);
        assert!(p.validate(ValidationMode::Shared).is_empty());
        let again = GameProfile::from_toml(&p.to_toml()).unwrap();
        assert_eq!(again, p);
    }

    #[test]
    fn rejects_unknown_fields() {
        assert!(GameProfile::from_toml("schema = 1\n[game]\nname='x'\nfoo=1\n").is_err());
    }

    #[test]
    fn rejects_bad_backend_for_api() {
        let mut p = GameProfile::new("x", None);
        p.graphics.d3d12 = Some(GraphicsBackend::Dxmt);
        p.graphics.d3d9 = Some(GraphicsBackend::Dxmt);
        assert_eq!(p.validate(ValidationMode::Local).len(), 2);
    }

    #[test]
    fn shared_profiles_must_be_open() {
        let mut p = GameProfile::new("x", None);
        p.graphics.d3d11 = Some(GraphicsBackend::D3dMetal);
        assert!(p.validate(ValidationMode::Local).is_empty());
        assert_eq!(p.validate(ValidationMode::Shared).len(), 1);
    }

    #[test]
    fn rejects_secrets_and_reserved_env() {
        let mut p = GameProfile::new("x", None);
        p.environment.insert("STEAM_TOKEN".into(), "abc".into());
        p.environment.insert("WINEPREFIX".into(), "/tmp".into());
        p.environment.insert("bad-key".into(), "1".into());
        assert_eq!(p.validate(ValidationMode::Local).len(), 3);
    }

    #[test]
    fn validates_display_and_overrides() {
        let mut p = GameProfile::new("x", None);
        p.display.width = Some(1920);
        p.dll_overrides.insert("d3d11".into(), "maybe".into());
        assert_eq!(p.validate(ValidationMode::Local).len(), 2);
        p.display.virtual_desktop = true;
        p.display.height = Some(1080);
        p.dll_overrides
            .insert("d3d11".into(), "native,builtin".into());
        assert!(p.validate(ValidationMode::Local).is_empty());
    }

    #[test]
    fn rejects_escaping_executable() {
        let mut p = GameProfile::new("x", None);
        p.game.executable = Some("../../bin/sh".into());
        assert_eq!(p.validate(ValidationMode::Local).len(), 1);
    }
}
