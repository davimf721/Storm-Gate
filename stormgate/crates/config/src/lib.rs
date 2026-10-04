//! Shared configuration types for Storm Gate.
//!
//! This crate has no knowledge of Wine or macOS APIs. It only defines the
//! on-disk layout, the game profile schema and prefix metadata so that every
//! other crate (and the future GUI) agrees on the same formats.

mod backend;
mod paths;
mod prefix_meta;
mod profile;
pub mod time;

pub use backend::{GraphicsBackend, SyncBackend, VulkanDriver};
pub use paths::Paths;
pub use prefix_meta::{PrefixMetadata, PREFIX_METADATA_FILE};
pub use profile::{
    DisplaySection, GameProfile, GameSection, GraphicsSection, ProfileError, RuntimeSection,
    SyncSection, ValidationMode, PROFILE_SCHEMA,
};

/// Product name used in user-facing output.
pub const PRODUCT_NAME: &str = "Storm Gate";

/// Version of the Storm Gate tooling (not of the bundled runtime).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Validates names used for prefixes, runtimes and log labels.
///
/// Names become directory names, so they are restricted to a conservative
/// character set and can never contain path separators or `..`.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("name must be between 1 and 64 characters".into());
    }
    let first = name.as_bytes()[0];
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return Err("name must start with a lowercase letter or digit".into());
    }
    if !name
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err("name may only contain a-z, 0-9, '-', '_' and '.'".into());
    }
    if name.contains("..") {
        return Err("name must not contain '..'".into());
    }
    Ok(())
}

/// Turns an arbitrary string (e.g. a game folder name) into a valid name.
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    for c in input.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_end_matches('-');
    let mut slug: String = trimmed.chars().take(64).collect();
    if slug.is_empty() {
        slug = "app".into();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(validate_name("steam").is_ok());
        assert!(validate_name("steam-1091500").is_ok());
        assert!(validate_name("my_game.v2").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("../etc").is_err());
        assert!(validate_name("a/b").is_err());
        assert!(validate_name("Upper").is_err());
        assert!(validate_name("-dash").is_err());
        assert!(validate_name("a..b").is_err());
        assert!(validate_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Hollow Knight"), "hollow-knight");
        assert_eq!(slugify("  ##  "), "app");
        assert_eq!(slugify("Game: The Sequel (2024)"), "game-the-sequel-2024");
        assert!(validate_name(&slugify("ÄÖÜ weird ∑ name")).is_ok());
    }
}
