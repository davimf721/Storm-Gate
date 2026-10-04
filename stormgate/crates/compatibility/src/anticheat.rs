use serde::Serialize;
use std::path::{Path, PathBuf};

/// Anti-cheat families recognised by file names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AntiCheatKind {
    EasyAntiCheat,
    BattlEye,
    /// Riot Vanguard (kernel driver; never supported).
    Vanguard,
    /// Other kernel-mode drivers shipped with the game.
    KernelDriver,
}

impl AntiCheatKind {
    pub fn label(self) -> &'static str {
        match self {
            AntiCheatKind::EasyAntiCheat => "Easy Anti-Cheat",
            AntiCheatKind::BattlEye => "BattlEye",
            AntiCheatKind::Vanguard => "Riot Vanguard",
            AntiCheatKind::KernelDriver => "kernel-mode driver",
        }
    }

    /// Kernel-level protection cannot run under Wine at all.
    pub fn is_kernel_level(self) -> bool {
        matches!(self, AntiCheatKind::Vanguard | AntiCheatKind::KernelDriver)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AntiCheat {
    pub kind: AntiCheatKind,
    pub evidence: PathBuf,
}

impl AntiCheat {
    /// User-facing explanation. Storm Gate never tries to bypass protection.
    pub fn message(&self) -> String {
        if self.kind.is_kernel_level() {
            format!(
                "This game uses a kernel-level anti-cheat ({}) that is not supported by this runtime.",
                self.kind.label()
            )
        } else {
            format!(
                "This game ships {}. Online modes only work if the developer enabled support for Wine/Proton; Storm Gate will not bypass it.",
                self.kind.label()
            )
        }
    }
}

const MAX_DEPTH: usize = 3;
const MAX_ENTRIES: usize = 20_000;

fn classify(name: &str) -> Option<AntiCheatKind> {
    let n = name.to_ascii_lowercase();
    if n == "easyanticheat" || n.starts_with("easyanticheat_") || n == "start_protected_game.exe" {
        Some(AntiCheatKind::EasyAntiCheat)
    } else if n == "battleye" || n.starts_with("beservice") || n.starts_with("beclient") {
        Some(AntiCheatKind::BattlEye)
    } else if n == "vgk.sys" || n == "vgc.exe" {
        Some(AntiCheatKind::Vanguard)
    } else if n.ends_with(".sys") {
        Some(AntiCheatKind::KernelDriver)
    } else {
        None
    }
}

/// Scans a game directory (bounded depth) for known anti-cheat files.
/// Detection is informational only.
pub fn scan_anticheat(dir: &Path) -> Vec<AntiCheat> {
    let mut found: Vec<AntiCheat> = Vec::new();
    let mut seen = 0usize;
    let mut stack = vec![(dir.to_path_buf(), 0usize)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return found;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(kind) = classify(&name) {
                if !found.iter().any(|f| f.kind == kind) {
                    found.push(AntiCheat {
                        kind,
                        evidence: entry.path(),
                    });
                }
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if is_dir && depth < MAX_DEPTH {
                stack.push((entry.path(), depth + 1));
            }
        }
    }
    found.sort_by_key(|a| a.kind.label());
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_known_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("EasyAntiCheat")).unwrap();
        std::fs::create_dir_all(tmp.path().join("bin/drivers")).unwrap();
        std::fs::write(tmp.path().join("bin/drivers/protect.sys"), "").unwrap();
        std::fs::write(tmp.path().join("game.exe"), "").unwrap();
        let found = scan_anticheat(tmp.path());
        let kinds: Vec<_> = found.iter().map(|f| f.kind).collect();
        assert!(kinds.contains(&AntiCheatKind::EasyAntiCheat));
        assert!(kinds.contains(&AntiCheatKind::KernelDriver));
        assert!(found.iter().any(|f| f.message().contains("kernel-level")));
    }

    #[test]
    fn clean_directory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("game.exe"), "").unwrap();
        assert!(scan_anticheat(tmp.path()).is_empty());
    }
}
