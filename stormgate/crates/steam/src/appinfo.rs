use std::path::Path;

use crate::vdf;
use crate::{Result, SteamError};

/// Parsed `steamapps/appmanifest_<appid>.acf`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppManifest {
    pub appid: u64,
    pub name: String,
    pub installdir: String,
    pub state_flags: u32,
}

impl AppManifest {
    /// StateFlags bit 2 (`4`) means "fully installed".
    pub fn is_fully_installed(&self) -> bool {
        self.state_flags & 4 != 0
    }

    pub fn parse(text: &str, origin: &Path) -> Result<Self> {
        let err = |m: String| SteamError::Parse(origin.to_path_buf(), m);
        let doc = vdf::parse(text).map_err(|e| err(e.to_string()))?;
        let state = doc
            .get("AppState")
            .ok_or_else(|| err("missing AppState".into()))?;
        let field = |k: &str| {
            state
                .str(k)
                .map(str::to_string)
                .ok_or_else(|| err(format!("missing {k}")))
        };
        let appid = field("appid")?
            .parse()
            .map_err(|_| err("appid is not a number".into()))?;
        let installdir = field("installdir")?;
        if installdir.is_empty()
            || installdir.contains(['/', '\\'])
            || installdir == ".."
            || installdir == "."
        {
            return Err(err(format!("suspicious installdir '{installdir}'")));
        }
        Ok(Self {
            appid,
            name: state.str("name").unwrap_or_default().to_string(),
            installdir,
            state_flags: state
                .str("StateFlags")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        })
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).map_err(|e| SteamError::Io(path.to_path_buf(), e))?;
        Self::parse(&text, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest() {
        let m = AppManifest::parse(
            r#""AppState" { "appid" "1091500" "name" "Example" "StateFlags" "4" "installdir" "Example Game" }"#,
            Path::new("x"),
        )
        .unwrap();
        assert_eq!(m.appid, 1_091_500);
        assert_eq!(m.installdir, "Example Game");
        assert!(m.is_fully_installed());
    }

    #[test]
    fn rejects_traversal() {
        let r = AppManifest::parse(
            r#""AppState" { "appid" "1" "installdir" "..\\..\\Windows" }"#,
            Path::new("x"),
        );
        assert!(r.is_err());
    }
}
