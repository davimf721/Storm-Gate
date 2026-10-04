use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::{io_err, Result, RuntimeError};

/// `manifests/runtime.toml` and the `runtime.toml` inside an installed runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeManifest {
    pub runtime: RuntimeInfo,
    #[serde(default)]
    pub sources: BTreeMap<String, SourceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeInfo {
    pub version: String,
    /// Features compiled into this runtime, e.g. `msync`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceEntry {
    pub repository: String,
    /// Branch or tag the commit was taken from (informational).
    #[serde(default, rename = "ref", skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// Full 40 character commit hash. Releases are always built from commits.
    pub commit: String,
    /// SPDX license expression of the upstream project.
    pub license: String,
    #[serde(default)]
    pub experimental: bool,
    /// Optional components are not required for a runtime to be usable.
    #[serde(default)]
    pub optional: bool,
}

impl RuntimeManifest {
    pub fn parse(text: &str, origin: &Path) -> Result<Self> {
        let manifest: Self = toml::from_str(text)
            .map_err(|e| RuntimeError::Manifest(origin.to_path_buf(), e.to_string()))?;
        let errors = manifest.validate();
        if errors.is_empty() {
            Ok(manifest)
        } else {
            Err(RuntimeError::Manifest(
                origin.to_path_buf(),
                errors.join("; "),
            ))
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(io_err(path))?;
        Self::parse(&text, path)
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.runtime.features.iter().any(|f| f == feature)
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if let Err(e) = stormgate_config::validate_name(&self.runtime.version) {
            errors.push(format!("runtime.version: {e}"));
        }
        for (name, src) in &self.sources {
            if !src.repository.starts_with("https://") {
                errors.push(format!("sources.{name}.repository must use https"));
            }
            if src.commit.len() != 40 || !src.commit.bytes().all(|b| b.is_ascii_hexdigit()) {
                errors.push(format!(
                    "sources.{name}.commit must be a full 40 character hash"
                ));
            }
            if src.license.trim().is_empty() {
                errors.push(format!("sources.{name}.license must not be empty"));
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_manifest_is_valid() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests/runtime.toml");
        let manifest = RuntimeManifest::load(&path).unwrap();
        for required in ["wine", "dxmt", "dxvk", "vkd3d-proton", "moltenvk"] {
            assert!(
                manifest.sources.contains_key(required),
                "{required} missing"
            );
        }
    }

    #[test]
    fn manifest_matches_sources_lock() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manifests");
        let manifest = RuntimeManifest::load(&root.join("runtime.toml")).unwrap();
        let lock = std::fs::read_to_string(root.join("sources.lock")).unwrap();
        let mut seen = 0;
        for line in lock
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        {
            let cols: Vec<_> = line.split_whitespace().collect();
            assert_eq!(cols.len(), 3, "bad sources.lock line: {line}");
            let src = manifest
                .sources
                .get(cols[0])
                .unwrap_or_else(|| panic!("{} not in runtime.toml", cols[0]));
            assert_eq!(src.repository, cols[1], "{}", cols[0]);
            assert_eq!(src.commit, cols[2], "{}", cols[0]);
            seen += 1;
        }
        assert_eq!(
            seen,
            manifest.sources.len(),
            "sources.lock is missing entries"
        );
    }

    #[test]
    fn rejects_floating_refs() {
        let text = r#"
[runtime]
version = "0.1.0"
[sources.wine]
repository = "http://example.com/wine"
commit = "main"
license = ""
"#;
        let err = RuntimeManifest::parse(text, Path::new("x"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("https"));
        assert!(err.contains("40 character"));
        assert!(err.contains("license"));
    }
}
