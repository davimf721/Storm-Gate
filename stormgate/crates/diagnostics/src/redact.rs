//! Removal of personal data from logs before they leave the machine.

use regex::Regex;

/// Replaces credentials, the home directory and the user name in text.
#[derive(Debug)]
pub struct Redactor {
    home: Option<String>,
    user: Option<Regex>,
    rules: Vec<(Regex, &'static str)>,
}

impl Redactor {
    pub fn new(home: Option<String>, user: Option<String>) -> Self {
        let rules = [
            // Must run before the key/value rule, which would eat "Bearer".
            (r"(?i)\b(bearer|basic)\s+[A-Za-z0-9._~+/=\-]{8,}", "${1} <redacted>"),
            // key=value / key: value pairs with credential-like keys.
            (
                r#"(?i)\b([A-Za-z0-9_\-]*(?:token|password|passwd|secret|cookie|session|auth|apikey|api_key)[A-Za-z0-9_\-]*)(\s*[=:]\s*|"\s*:\s*"|"\s+")[^\s"';&]+"#,
                "${1}${2}<redacted>",
            ),
            (r"(?i)steamLoginSecure=[^\s;]+", "steamLoginSecure=<redacted>"),
            // 64-bit SteamIDs identify an account.
            (r"\b7656119\d{10}\b", "<steamid>"),
            (r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}", "<email>"),
        ]
        .into_iter()
        .map(|(re, rep)| (Regex::new(re).expect("valid redaction regex"), rep))
        .collect();
        let user = user.filter(|u| u.len() >= 3).map(|u| {
            Regex::new(&format!(r"\b{}\b", regex::escape(&u))).expect("escaped user regex")
        });
        Self {
            home: home.filter(|h| h.len() > 1),
            user,
            rules,
        }
    }

    /// Uses `$HOME` and `$USER` from the environment.
    pub fn from_env() -> Self {
        Self::new(std::env::var("HOME").ok(), std::env::var("USER").ok())
    }

    pub fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        if let Some(home) = &self.home {
            out = out.replace(home.as_str(), "~");
        }
        for (re, rep) in &self.rules {
            out = re.replace_all(&out, *rep).into_owned();
        }
        if let Some(user) = &self.user {
            out = user.replace_all(&out, "<user>").into_owned();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_personal_data() {
        let r = Redactor::new(Some("/Users/davi".into()), Some("davi".into()));
        let text = "\
WINEPREFIX=/Users/davi/Library/Application Support/StormGate/prefixes/steam
access_token=abc.def.ghi refresh_token: xyz
\"password\": \"hunter2\"
Authorization: Bearer abcdefghijklmnop
Cookie steamLoginSecure=76561198000000000%7C%7Cabc
user davi logged in as 76561198012345678 mail me@example.com
david stays";
        let out = r.apply(text);
        assert!(out.contains("WINEPREFIX=~/Library/Application Support"));
        assert!(!out.contains("abc.def.ghi"));
        assert!(!out.contains("xyz"));
        assert!(!out.contains("hunter2"));
        assert!(!out.contains("abcdefghijklmnop"));
        assert!(!out.contains("76561198012345678"));
        assert!(out.contains("steamLoginSecure=<redacted>"));
        assert!(out.contains("<user> logged in as <steamid>"));
        assert!(out.contains("<email>"));
        assert!(out.contains("david stays"), "{out}");
    }

    #[test]
    fn leaves_normal_logs_alone() {
        let r = Redactor::new(None, None);
        let line = "0024:err:module:import_dll Library d3d11.dll not found";
        assert_eq!(r.apply(line), line);
    }
}
