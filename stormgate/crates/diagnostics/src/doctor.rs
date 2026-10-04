use serde::Serialize;
use std::path::Path;

use stormgate_runtime::{Component, ComponentState, Runtime, RuntimeError};

use crate::system::{HostProbe, SystemInfo};

/// Oldest macOS release Storm Gate targets.
pub const MIN_MACOS: u32 = 14;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

impl CheckStatus {
    pub fn symbol(self) -> &'static str {
        match self {
            CheckStatus::Ok => "✓",
            CheckStatus::Warn => "!",
            CheckStatus::Fail => "✗",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub status: CheckStatus,
    pub label: String,
    /// What to do about a warning or failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl Check {
    fn ok(label: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Ok,
            label: label.into(),
            hint: None,
        }
    }
    fn warn(label: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Warn,
            label: label.into(),
            hint: Some(hint.into()),
        }
    }
    fn fail(label: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            status: CheckStatus::Fail,
            label: label.into(),
            hint: Some(hint.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub title: &'static str,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub system: SystemInfo,
    pub sections: Vec<Section>,
}

impl DoctorReport {
    pub fn ready(&self) -> bool {
        self.sections
            .iter()
            .flat_map(|s| &s.checks)
            .all(|c| c.status != CheckStatus::Fail)
    }

    pub fn render(&self) -> String {
        let mut out = String::from("Storm Gate Doctor\n");
        for section in &self.sections {
            out.push('\n');
            out.push_str(section.title);
            out.push('\n');
            for c in &section.checks {
                out.push_str(&format!("{} {}\n", c.status.symbol(), c.label));
                if let Some(h) = &c.hint {
                    for line in h.lines() {
                        out.push_str(&format!("    {line}\n"));
                    }
                }
            }
        }
        out.push_str(&format!(
            "\nResult: {}\n",
            if self.ready() { "READY" } else { "NOT READY" }
        ));
        out
    }
}

/// Runs every check. `runtime` is the resolved runtime (or why there is none);
/// `steam_prefix` is the Steam prefix path when it exists.
pub fn run_doctor(
    probe: &dyn HostProbe,
    runtime: Result<&Runtime, &RuntimeError>,
    steam_prefix: Option<&Path>,
) -> DoctorReport {
    let system = SystemInfo::collect(probe);
    let mut sections = Vec::new();

    // System
    let mut checks = Vec::new();
    let mac = system.os == "macos";
    if !mac {
        checks.push(Check::fail(
            format!("Host is {} (macOS required)", system.os),
            "Storm Gate runs on macOS. Other hosts are only supported for development (cargo test).",
        ));
    } else if system.is_apple_silicon() {
        let chip = system
            .chip
            .clone()
            .unwrap_or_else(|| "Apple Silicon".into());
        let mem = system
            .memory_gb
            .map(|g| format!(", {g} GB"))
            .unwrap_or_default();
        checks.push(Check::ok(format!("Apple Silicon: {chip}{mem}")));
    } else {
        checks.push(Check::warn(
            format!("Intel Mac ({})", system.chip.clone().unwrap_or_default()),
            "Intel Macs are Tier 3 and not tested yet.",
        ));
    }
    if system.translated {
        checks.push(Check::warn(
            "stormgate itself is running under Rosetta",
            "Install the arm64 build of stormgate; only Wine needs Rosetta.",
        ));
    }
    if mac {
        match system.macos_major() {
            Some(v) if v >= MIN_MACOS => checks.push(Check::ok(format!(
                "macOS {} supported",
                system.macos_version.clone().unwrap_or_default()
            ))),
            Some(_) => checks.push(Check::warn(
                format!(
                    "macOS {} is older than macOS {MIN_MACOS}",
                    system.macos_version.clone().unwrap_or_default()
                ),
                "Older releases are untested; Metal and Rosetta behaviour may differ.",
            )),
            None => checks.push(Check::warn("macOS version unknown", "sw_vers failed")),
        }
        if system.is_apple_silicon() {
            if system.rosetta_installed {
                checks.push(Check::ok("Rosetta 2 installed"));
            } else {
                checks.push(Check::fail(
                    "Rosetta 2 missing",
                    "Install it yourself (this accepts Apple's license):\n/usr/sbin/softwareupdate --install-rosetta --agree-to-license",
                ));
            }
        }
        if probe.command("/usr/bin/xcode-select", &["-p"]).is_some() {
            checks.push(Check::ok("Xcode Command Line Tools"));
        } else {
            checks.push(Check::warn(
                "Xcode Command Line Tools missing",
                "Needed only to build the runtime: xcode-select --install",
            ));
        }
        let metal = probe
            .command("/usr/sbin/system_profiler", &["SPDisplaysDataType"])
            .is_some_and(|out| out.contains("Metal"));
        if metal {
            checks.push(Check::ok("Metal available"));
        } else {
            checks.push(Check::fail(
                "Metal support not reported by system_profiler",
                "DXMT and MoltenVK require a Metal capable GPU.",
            ));
        }
    }
    sections.push(Section {
        title: "System",
        checks,
    });

    // Runtime
    let mut checks = Vec::new();
    match runtime {
        Err(e) => checks.push(Check::fail("No runtime", e.to_string())),
        Ok(rt) => {
            if rt.is_system() {
                checks.push(Check::warn(
                    format!("Using development Wine at {}", rt.wine().display()),
                    "STORMGATE_WINE is set; graphics backends are unavailable in this mode.",
                ));
            } else {
                checks.push(Check::ok(format!(
                    "Runtime {} ({})",
                    rt.version(),
                    rt.root().display()
                )));
            }
            let components: &[Component] = if rt.is_system() {
                &[Component::Wine]
            } else {
                &Component::ALL
            };
            for &component in components {
                let state = rt.component(component);
                let label = component.label();
                checks.push(match state {
                    ComponentState::Present(_) if component.is_experimental() => Check::warn(
                        format!("{label} experimental"),
                        "Works for some workloads only; see docs/graphics/d3d12.md.",
                    ),
                    ComponentState::Present(_) => Check::ok(label),
                    ComponentState::Missing(p) if component.is_optional() => Check::warn(
                        format!("{label} not included"),
                        format!("optional component (looked for {})", p.display()),
                    ),
                    ComponentState::Missing(p) => Check::fail(
                        format!("{label} missing"),
                        format!("expected {}", p.display()),
                    ),
                });
            }
        }
    }
    sections.push(Section {
        title: "Runtime",
        checks,
    });

    // Steam
    let mut checks = Vec::new();
    match steam_prefix {
        None => checks.push(Check::warn(
            "Steam prefix not created",
            "Optional: stormgate steam install",
        )),
        Some(p) => {
            checks.push(Check::ok("Steam prefix"));
            let exe = p.join("drive_c/Program Files (x86)/Steam/steam.exe");
            if probe.path_exists(&exe) {
                checks.push(Check::ok("steam.exe found"));
            } else {
                checks.push(Check::warn(
                    "steam.exe not found",
                    "Run: stormgate steam install",
                ));
            }
        }
    }
    sections.push(Section {
        title: "Steam",
        checks,
    });

    DoctorReport { system, sections }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    struct Fake {
        os: &'static str,
        arch: &'static str,
        commands: HashMap<String, String>,
        paths: Vec<PathBuf>,
    }

    impl HostProbe for Fake {
        fn os(&self) -> &'static str {
            self.os
        }
        fn arch(&self) -> &'static str {
            self.arch
        }
        fn command(&self, program: &str, args: &[&str]) -> Option<String> {
            self.commands
                .get(&format!("{program} {}", args.join(" ")))
                .cloned()
        }
        fn path_exists(&self, path: &Path) -> bool {
            self.paths.iter().any(|p| p == path)
        }
    }

    fn m1(rosetta: bool) -> Fake {
        let mut commands = HashMap::new();
        for (k, v) in [
            ("/usr/sbin/sysctl -n hw.optional.arm64", "1"),
            ("/usr/sbin/sysctl -n machdep.cpu.brand_string", "Apple M1"),
            ("/usr/sbin/sysctl -n hw.memsize", "8589934592"),
            ("/usr/sbin/sysctl -n sysctl.proc_translated", "0"),
            ("/usr/bin/sw_vers -productVersion", "15.6"),
            (
                "/usr/bin/xcode-select -p",
                "/Library/Developer/CommandLineTools",
            ),
            (
                "/usr/sbin/system_profiler SPDisplaysDataType",
                "Metal Support: Metal 3",
            ),
        ] {
            commands.insert(k.to_string(), v.to_string());
        }
        Fake {
            os: "macos",
            arch: "aarch64",
            commands,
            paths: if rosetta {
                vec![PathBuf::from(crate::system::ROSETTA_RUNTIME)]
            } else {
                vec![]
            },
        }
    }

    fn runtime(tmp: &Path) -> Runtime {
        let root = tmp.join("rt");
        std::fs::create_dir_all(root.join("wine/bin")).unwrap();
        std::fs::write(root.join("wine/bin/wine"), "").unwrap();
        for (d, f) in [
            ("dxmt/x86_64-windows", "d3d11.dll"),
            ("dxvk/x64", "d3d9.dll"),
            ("moltenvk/icd.d", "MoltenVK_icd.json"),
        ] {
            std::fs::create_dir_all(root.join(d)).unwrap();
            std::fs::write(root.join(d).join(f), "").unwrap();
        }
        std::fs::write(
            root.join("runtime.toml"),
            "[runtime]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        Runtime::open(&root).unwrap()
    }

    #[test]
    fn ready_m1() {
        let tmp = tempfile::tempdir().unwrap();
        let rt = runtime(tmp.path());
        let report = run_doctor(&m1(true), Ok(&rt), None);
        let text = report.render();
        assert!(report.ready(), "{text}");
        assert!(text.contains("✓ Apple Silicon: Apple M1, 8 GB"));
        assert!(text.contains("✓ Rosetta 2 installed"));
        assert!(text.contains("! VKD3D-Proton not included"));
        assert!(text.contains("Result: READY"));
    }

    #[test]
    fn missing_rosetta_is_fatal_with_hint() {
        let tmp = tempfile::tempdir().unwrap();
        let rt = runtime(tmp.path());
        let report = run_doctor(&m1(false), Ok(&rt), None);
        assert!(!report.ready());
        let text = report.render();
        assert!(text.contains("✗ Rosetta 2 missing"));
        assert!(text.contains("softwareupdate --install-rosetta --agree-to-license"));
    }

    #[test]
    fn linux_dev_host() {
        let fake = Fake {
            os: "linux",
            arch: "x86_64",
            commands: HashMap::new(),
            paths: vec![],
        };
        let report = run_doctor(&fake, Err(&RuntimeError::NoneInstalled), None);
        assert!(!report.ready());
        assert!(report.render().contains("macOS required"));
    }
}
