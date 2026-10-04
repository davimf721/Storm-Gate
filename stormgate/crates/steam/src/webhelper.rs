//! `steamwebhelper` (Chromium Embedded Framework) workarounds.
//!
//! CEF is the most fragile part of the Steam client under Wine on macOS.
//! Workarounds are collected here, isolated from the Wine patch queue, and
//! each one must reference a reproduction (issue or test) before it is added.

/// Extra arguments always passed to `steam.exe`.
///
/// Intentionally empty until the Steam milestone has reproductions on real
/// hardware; users can experiment with `stormgate steam start -- <args>`.
pub const DEFAULT_ARGS: &[&str] = &[];

/// Arguments for a launch: defaults first, then user supplied ones.
pub fn steam_args(extra: &[String]) -> Vec<String> {
    DEFAULT_ARGS
        .iter()
        .map(|s| s.to_string())
        .chain(extra.iter().cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_user_args() {
        let args = steam_args(&["-console".to_string()]);
        assert_eq!(args.last().unwrap(), "-console");
    }
}
