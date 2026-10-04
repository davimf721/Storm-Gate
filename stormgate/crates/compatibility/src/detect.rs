use serde::Serialize;
use std::path::{Path, PathBuf};

use stormgate_config::{GameProfile, GraphicsBackend, SyncBackend};
use stormgate_graphics::default_backend;
use stormgate_pe::{inspect_file, GraphicsApi, GraphicsApis, Machine, PeInfo};

use crate::anticheat::{scan_anticheat, AntiCheat};

/// Engine DLLs scanned besides the executable (many games load D3D from them).
const MAX_SIBLING_DLLS: usize = 64;

/// Result of inspecting a game executable and its directory.
#[derive(Debug, Clone, Serialize)]
pub struct Detection {
    pub executable: PathBuf,
    pub machine: Machine,
    pub apis: GraphicsApis,
    /// Modules (besides the exe) that contributed graphics imports.
    pub api_sources: Vec<String>,
    pub anticheat: Vec<AntiCheat>,
    #[serde(skip)]
    pub pe: PeInfo,
}

impl Detection {
    /// The newest Direct3D version found.
    pub fn primary_api(&self) -> Option<GraphicsApi> {
        self.apis.iter().filter(|a| *a != GraphicsApi::Dxgi).last()
    }
}

/// Inspects `exe`, plus DLLs next to it, and scans for anti-cheat.
pub fn detect(exe: &Path) -> Result<Detection, stormgate_pe::PeError> {
    let pe = inspect_file(exe)?;
    let mut apis = pe.graphics_apis();
    let mut api_sources = Vec::new();
    let dir = exe.parent().unwrap_or(Path::new("."));
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut dlls: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("dll")))
            .collect();
        dlls.sort();
        for dll in dlls.into_iter().take(MAX_SIBLING_DLLS) {
            let name = dll
                .file_name()
                .map(|n| n.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            // A game shipping its own d3d11.dll etc. is a wrapper, not a user.
            if stormgate_graphics::MANAGED_DLLS
                .iter()
                .any(|m| name == format!("{m}.dll"))
            {
                continue;
            }
            if let Ok(info) = inspect_file(&dll) {
                let found = info.graphics_apis();
                if !found.is_empty() {
                    apis.extend(&found);
                    api_sources.push(name);
                }
            }
        }
    }
    Ok(Detection {
        executable: exe.to_path_buf(),
        machine: pe.machine,
        apis,
        api_sources,
        anticheat: scan_anticheat(dir),
        pe,
    })
}

/// Builds a profile suggestion from a detection. Nothing is saved here; the
/// caller must ask the user before writing it.
pub fn suggest_profile(name: &str, appid: Option<u64>, detection: &Detection) -> GameProfile {
    let mut p = GameProfile::new(name, appid);
    for api in detection.apis.iter() {
        let b = default_backend(api);
        match api {
            GraphicsApi::D3D9 => p.graphics.d3d9 = Some(b),
            GraphicsApi::D3D10 => p.graphics.d3d10 = Some(b),
            GraphicsApi::D3D11 => p.graphics.d3d11 = Some(b),
            GraphicsApi::D3D12 => p.graphics.d3d12 = Some(b),
            _ => {}
        }
    }
    if p.graphics.d3d12 == Some(GraphicsBackend::Vkd3dProton) || p.graphics.d3d9.is_some() {
        p.graphics.vulkan = Some(Default::default());
    }
    // MSync stays opt-in until regression testing makes it the default.
    p.sync.backend = SyncBackend::Server;
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use stormgate_pe::build_test_pe;

    #[test]
    fn detects_engine_dll_apis() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("Game.exe");
        std::fs::write(
            &exe,
            build_test_pe(true, &["KERNEL32.dll", "UnityPlayer.dll"], &[]),
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("UnityPlayer.dll"),
            build_test_pe(true, &["d3d11.dll", "dxgi.dll"], &[]),
        )
        .unwrap();
        // A bundled wrapper DLL must not count as an API user.
        std::fs::write(
            tmp.path().join("d3d9.dll"),
            build_test_pe(true, &["d3d9.dll"], &[]),
        )
        .unwrap();
        let d = detect(&exe).unwrap();
        assert_eq!(d.machine, Machine::Amd64);
        assert!(d.apis.contains(GraphicsApi::D3D11));
        assert!(!d.apis.contains(GraphicsApi::D3D9));
        assert_eq!(d.api_sources, vec!["unityplayer.dll"]);
        assert_eq!(d.primary_api(), Some(GraphicsApi::D3D11));

        let p = suggest_profile("Game", Some(1), &d);
        assert_eq!(p.graphics.d3d11, Some(GraphicsBackend::Dxmt));
        assert!(p
            .validate(stormgate_config::ValidationMode::Shared)
            .is_empty());
    }

    #[test]
    fn suggests_vkd3d_for_d3d12() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("x.exe");
        std::fs::write(&exe, build_test_pe(true, &["d3d12.dll"], &[])).unwrap();
        let d = detect(&exe).unwrap();
        let p = suggest_profile("x", None, &d);
        assert_eq!(p.graphics.d3d12, Some(GraphicsBackend::Vkd3dProton));
        assert!(p.graphics.vulkan.is_some());
    }
}

/// Executables that are never the game itself.
const NOT_THE_GAME: &[&str] = &[
    "crashhandler",
    "crashreport",
    "unins",
    "redist",
    "dxsetup",
    "prereq",
    "vcredist",
    "vc_redist",
    "dotnet",
    "easyanticheat",
    "beservice",
    "start_protected_game",
];

/// Picks the most likely game executable in an install directory (largest
/// `.exe` in the top two levels, skipping installers and crash handlers).
pub fn guess_executable(dir: &Path) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 0)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                if depth < 2 {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if !name.ends_with(".exe") || NOT_THE_GAME.iter().any(|n| name.contains(n)) {
                continue;
            }
            candidates.push((meta.len(), std::cmp::Reverse(depth), path));
        }
    }
    candidates.sort();
    candidates.pop().map(|(_, _, p)| p)
}

#[cfg(test)]
mod guess_tests {
    use super::*;

    #[test]
    fn picks_largest_game_exe() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("Game.exe"), vec![0u8; 1000]).unwrap();
        std::fs::write(tmp.path().join("UnityCrashHandler64.exe"), vec![0u8; 5000]).unwrap();
        std::fs::write(tmp.path().join("unins000.exe"), vec![0u8; 5000]).unwrap();
        std::fs::create_dir_all(tmp.path().join("_CommonRedist")).unwrap();
        std::fs::write(
            tmp.path().join("_CommonRedist/vc_redist.x64.exe"),
            vec![0u8; 9000],
        )
        .unwrap();
        assert_eq!(
            guess_executable(tmp.path()).unwrap(),
            tmp.path().join("Game.exe")
        );
        assert!(guess_executable(&tmp.path().join("missing")).is_none());
    }
}
