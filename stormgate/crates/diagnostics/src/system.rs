//! Host facts (chip, macOS version, Rosetta...). Everything goes through
//! [`HostProbe`] so `doctor` can be tested on any platform.

use serde::Serialize;
use std::path::Path;
use std::process::Command;

/// Read-only questions about the host.
pub trait HostProbe {
    fn os(&self) -> &'static str;
    fn arch(&self) -> &'static str;
    /// Runs a command and returns trimmed stdout when it exits successfully.
    fn command(&self, program: &str, args: &[&str]) -> Option<String>;
    fn path_exists(&self, path: &Path) -> bool;
}

/// The real host.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemProbe;

impl HostProbe for SystemProbe {
    fn os(&self) -> &'static str {
        std::env::consts::OS
    }

    fn arch(&self) -> &'static str {
        std::env::consts::ARCH
    }

    fn command(&self, program: &str, args: &[&str]) -> Option<String> {
        let out = Command::new(program).args(args).output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

/// Snapshot written as `system.json` in each log session.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub macos_version: Option<String>,
    pub chip: Option<String>,
    pub memory_gb: Option<u64>,
    /// True when this process itself runs under Rosetta.
    pub translated: bool,
    pub rosetta_installed: bool,
    pub stormgate_version: String,
}

/// Rosetta's runtime library; present only once Rosetta 2 is installed.
pub const ROSETTA_RUNTIME: &str = "/Library/Apple/usr/libexec/oah/libRosettaRuntime";

impl SystemInfo {
    pub fn collect(probe: &dyn HostProbe) -> Self {
        let mac = probe.os() == "macos";
        let sysctl = |key: &str| {
            if mac {
                probe.command("/usr/sbin/sysctl", &["-n", key])
            } else {
                None
            }
        };
        let arm64_hw = sysctl("hw.optional.arm64").as_deref() == Some("1");
        Self {
            os: probe.os().into(),
            // A translated binary reports x86_64; report the real hardware.
            arch: if arm64_hw {
                "aarch64".into()
            } else {
                probe.arch().into()
            },
            macos_version: if mac {
                probe.command("/usr/bin/sw_vers", &["-productVersion"])
            } else {
                None
            },
            chip: sysctl("machdep.cpu.brand_string"),
            memory_gb: sysctl("hw.memsize")
                .and_then(|s| s.parse::<u64>().ok())
                .map(|b| b / (1 << 30)),
            translated: sysctl("sysctl.proc_translated").as_deref() == Some("1"),
            rosetta_installed: mac
                && (probe.path_exists(Path::new(ROSETTA_RUNTIME))
                    || probe.command("/usr/bin/pgrep", &["-q", "oahd"]).is_some()),
            stormgate_version: stormgate_config::VERSION.into(),
        }
    }

    pub fn is_apple_silicon(&self) -> bool {
        self.os == "macos" && self.arch == "aarch64"
    }

    /// Major macOS version (e.g. 15 for "15.6.1").
    pub fn macos_major(&self) -> Option<u32> {
        self.macos_version
            .as_deref()?
            .split('.')
            .next()?
            .parse()
            .ok()
    }
}
