//! Graphics backend selection.
//!
//! Turns the set of Direct3D APIs a game uses (plus profile overrides) into a
//! concrete plan: which translation layer handles each API, which DLL
//! overrides Wine needs and which environment variables to set.
//!
//! Default policy (docs/adr/0003-dxmt-default-d3d11.md):
//!
//! ```text
//! D3D12      -> VKD3D-Proton (experimental) over Vulkan-on-Metal
//! D3D10/11   -> DXMT (Metal)
//! D3D9       -> DXVK over Vulkan-on-Metal
//! otherwise  -> WineD3D
//! ```

use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

pub use stormgate_config::{GraphicsBackend, GraphicsSection, VulkanDriver};
pub use stormgate_pe::{GraphicsApi, GraphicsApis};

/// One API -> backend decision and why it was made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assignment {
    pub api: GraphicsApi,
    pub backend: GraphicsBackend,
    pub reason: String,
}

/// Debug switches for `--debug-graphics`. Heavy validation is opt-in only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugOptions {
    pub logs: bool,
    pub vulkan_validation: bool,
    pub metal_validation: bool,
}

/// Complete graphics configuration for one launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GraphicsPlan {
    pub assignments: Vec<Assignment>,
    pub vulkan: Option<VulkanDriver>,
    /// Backend whose `dxgi.dll` is deployed. Only one dxgi can be active per
    /// launch; VKD3D-Proton needs DXVK's dxgi, so it wins when D3D12 is used.
    pub dxgi_provider: Option<GraphicsBackend>,
    /// Wine DLL overrides (dll -> mode), e.g. `d3d11 -> native,builtin`.
    pub dll_overrides: BTreeMap<String, String>,
    pub env: BTreeMap<String, String>,
    pub warnings: Vec<String>,
}

impl GraphicsPlan {
    pub fn backends(&self) -> Vec<GraphicsBackend> {
        let mut out: Vec<_> = self.assignments.iter().map(|a| a.backend).collect();
        out.sort_by_key(|b| b.as_str());
        out.dedup();
        out
    }

    pub fn primary(&self) -> Option<&Assignment> {
        self.assignments.last()
    }
}

/// Direct3D DLLs whose load order Storm Gate always sets explicitly.
pub const MANAGED_DLLS: [&str; 7] = [
    "d3d9",
    "d3d10core",
    "d3d11",
    "d3d12",
    "d3d12core",
    "dxgi",
    "d3d8",
];

/// Default backend for an API, before profile overrides.
pub fn default_backend(api: GraphicsApi) -> GraphicsBackend {
    match api {
        GraphicsApi::D3D12 => GraphicsBackend::Vkd3dProton,
        GraphicsApi::D3D10 | GraphicsApi::D3D11 | GraphicsApi::Dxgi => GraphicsBackend::Dxmt,
        GraphicsApi::D3D9 => GraphicsBackend::Dxvk,
        GraphicsApi::D3D8 | GraphicsApi::OpenGL | GraphicsApi::Vulkan => GraphicsBackend::WineD3d,
    }
}

fn profile_override(section: &GraphicsSection, api: GraphicsApi) -> Option<GraphicsBackend> {
    match api {
        GraphicsApi::D3D9 => section.d3d9,
        GraphicsApi::D3D10 => section.d3d10.or(section.d3d11),
        GraphicsApi::D3D11 | GraphicsApi::Dxgi => section.d3d11,
        GraphicsApi::D3D12 => section.d3d12,
        _ => None,
    }
}

/// Builds the plan for the detected APIs.
///
/// `forced` (from `--backend`) wins over the profile, which wins over the
/// default policy. A forced backend that cannot implement an API is ignored
/// for that API with a warning.
pub fn plan(
    apis: &GraphicsApis,
    profile: &GraphicsSection,
    forced: Option<GraphicsBackend>,
    debug: DebugOptions,
    log_dir: Option<&Path>,
) -> GraphicsPlan {
    let mut warnings = Vec::new();
    let mut assignments = Vec::new();

    let mut detected: Vec<GraphicsApi> = apis.iter().collect();
    // A bare dxgi import means D3D10/11/12 loaded at runtime; only treat it as
    // D3D11 when nothing more specific was found.
    if detected.len() > 1 {
        detected.retain(|a| *a != GraphicsApi::Dxgi);
    }

    for api in detected {
        let (backend, reason) = if let Some(f) = forced.filter(|f| supports(*f, api)) {
            (f, "forced on the command line".to_string())
        } else if let Some(p) = profile_override(profile, api) {
            (p, "game profile".to_string())
        } else {
            (default_backend(api), "default policy".to_string())
        };
        if let Some(f) = forced.filter(|f| !supports(*f, api)) {
            warnings.push(format!(
                "{f} cannot implement {api}; using {backend} for it"
            ));
        }
        if backend.is_experimental() {
            warnings.push(format!("{backend} is experimental on macOS"));
        }
        if !backend.is_open() {
            warnings.push(format!(
                "{backend} is an optional proprietary backend and must be installed separately by the user"
            ));
        }
        assignments.push(Assignment {
            api,
            backend,
            reason,
        });
    }
    if assignments.iter().any(|a| a.api == GraphicsApi::D3D8) {
        warnings.push("D3D8 runs through WineD3D; d8vk integration is not done yet".into());
    }

    let vulkan = assignments
        .iter()
        .any(|a| a.backend.uses_vulkan())
        .then(|| profile.vulkan.unwrap_or_default());

    let has = |b: GraphicsBackend| assignments.iter().any(|a| a.backend == b);
    let dxgi_provider = if has(GraphicsBackend::Vkd3dProton) {
        Some(GraphicsBackend::Dxvk)
    } else if has(GraphicsBackend::Dxmt) {
        Some(GraphicsBackend::Dxmt)
    } else if has(GraphicsBackend::Dxvk) {
        Some(GraphicsBackend::Dxvk)
    } else {
        None
    };
    if has(GraphicsBackend::Vkd3dProton) && has(GraphicsBackend::Dxmt) {
        warnings.push(
            "D3D12 and DXMT are both in use; dxgi.dll comes from DXVK, which may break DXMT presentation"
                .into(),
        );
    }

    let mut dll_overrides = BTreeMap::new();
    for a in &assignments {
        for (dll, mode) in overrides_for(a.api, a.backend) {
            dll_overrides.insert(dll.to_string(), mode.to_string());
        }
    }

    // DLLs managed by Storm Gate that no backend claimed stay on Wine's
    // builtins, so a DLL deployed by an earlier launch is never picked up.
    if !assignments
        .iter()
        .any(|a| a.backend == GraphicsBackend::D3dMetal)
    {
        for dll in MANAGED_DLLS {
            dll_overrides
                .entry(dll.to_string())
                .or_insert_with(|| "builtin".to_string());
        }
    }

    let mut env = BTreeMap::new();
    if debug.logs {
        let dir = log_dir.map(|d| d.display().to_string());
        for b in assignments.iter().map(|a| a.backend) {
            debug_env(b, dir.as_deref(), &mut env);
        }
        if vulkan == Some(VulkanDriver::MoltenVk) {
            env.insert("MVK_CONFIG_LOG_LEVEL".into(), "3".into());
        }
    }
    if debug.vulkan_validation && vulkan.is_some() {
        env.insert(
            "VK_INSTANCE_LAYERS".into(),
            "VK_LAYER_KHRONOS_validation".into(),
        );
    }
    if debug.metal_validation {
        env.insert("MTL_DEBUG_LAYER".into(), "1".into());
        env.insert("METAL_DEVICE_WRAPPER_TYPE".into(), "1".into());
    }

    GraphicsPlan {
        assignments,
        vulkan,
        dxgi_provider,
        dll_overrides,
        env,
        warnings,
    }
}

/// Whether a backend can implement an API at all.
pub fn supports(backend: GraphicsBackend, api: GraphicsApi) -> bool {
    use GraphicsApi::*;
    use GraphicsBackend::*;
    match backend {
        Dxmt => matches!(api, D3D10 | D3D11 | Dxgi),
        Dxvk => matches!(api, D3D9 | D3D10 | D3D11 | Dxgi),
        Vkd3dProton => matches!(api, D3D12),
        WineD3d => !matches!(api, D3D12 | Vulkan),
        D3dMetal => matches!(api, D3D11 | D3D12 | Dxgi),
    }
}

/// DLLs that must be loaded from the backend instead of Wine's builtins.
fn overrides_for(api: GraphicsApi, backend: GraphicsBackend) -> Vec<(&'static str, &'static str)> {
    const NB: &str = "native,builtin";
    const B: &str = "builtin";
    use GraphicsApi::*;
    use GraphicsBackend::*;
    match (backend, api) {
        (Dxmt, _) => vec![("d3d11", NB), ("d3d10core", NB), ("dxgi", NB)],
        (Dxvk, D3D9) => vec![("d3d9", NB)],
        (Dxvk, _) => vec![("d3d11", NB), ("d3d10core", NB), ("dxgi", NB)],
        // VKD3D-Proton relies on DXVK's dxgi for swapchains.
        (Vkd3dProton, _) => vec![("d3d12", NB), ("d3d12core", NB), ("dxgi", NB)],
        (WineD3d, D3D9) => vec![("d3d9", B)],
        (WineD3d, D3D8) => vec![("d3d8", B)],
        (WineD3d, D3D10 | D3D11 | Dxgi) => vec![("d3d11", B), ("d3d10core", B), ("dxgi", B)],
        (WineD3d, _) => vec![],
        // D3DMetal ships its own Wine integration; nothing generic to override.
        (D3dMetal, _) => vec![],
    }
}

fn debug_env(backend: GraphicsBackend, log_dir: Option<&str>, env: &mut BTreeMap<String, String>) {
    match backend {
        GraphicsBackend::Dxvk => {
            env.insert("DXVK_LOG_LEVEL".into(), "info".into());
            if let Some(d) = log_dir {
                env.insert("DXVK_LOG_PATH".into(), d.into());
            }
        }
        GraphicsBackend::Dxmt => {
            // DXMT follows DXVK's logging conventions.
            env.insert("DXMT_LOG_LEVEL".into(), "info".into());
            if let Some(d) = log_dir {
                env.insert("DXMT_LOG_PATH".into(), d.into());
            }
        }
        GraphicsBackend::Vkd3dProton => {
            env.insert("VKD3D_DEBUG".into(), "warn".into());
            env.insert("VKD3D_SHADER_DEBUG".into(), "warn".into());
            if let Some(d) = log_dir {
                env.insert("VKD3D_LOG_FILE".into(), format!("{d}/vkd3d.log"));
            }
        }
        GraphicsBackend::WineD3d | GraphicsBackend::D3dMetal => {}
    }
}

/// Formats overrides for `WINEDLLOVERRIDES` (`a,b=n,b;c=b`).
pub fn format_dll_overrides(overrides: &BTreeMap<String, String>) -> String {
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (dll, mode) in overrides {
        let mode = match mode.as_str() {
            "native,builtin" => "n,b",
            "builtin,native" => "b,n",
            "native" => "n",
            "builtin" => "b",
            "disabled" => "",
            other => other,
        };
        groups.entry(mode).or_default().push(dll);
    }
    groups
        .into_iter()
        .map(|(mode, dlls)| format!("{}={mode}", dlls.join(",")))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apis(list: &[GraphicsApi]) -> GraphicsApis {
        let mut s = GraphicsApis::default();
        for a in list {
            s.insert(*a);
        }
        s
    }

    #[test]
    fn default_policy() {
        let p = plan(
            &apis(&[GraphicsApi::D3D11, GraphicsApi::Dxgi]),
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        assert_eq!(p.backends(), vec![GraphicsBackend::Dxmt]);
        assert_eq!(p.vulkan, None);
        assert_eq!(p.dll_overrides.get("d3d11").unwrap(), "native,builtin");
        assert_eq!(p.dll_overrides.get("d3d12").unwrap(), "builtin");
        assert!(p.warnings.is_empty());
        assert_eq!(p.dxgi_provider, Some(GraphicsBackend::Dxmt));

        let p = plan(
            &apis(&[GraphicsApi::D3D12]),
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        assert_eq!(p.backends(), vec![GraphicsBackend::Vkd3dProton]);
        assert_eq!(p.vulkan, Some(VulkanDriver::MoltenVk));
        assert!(p.warnings.iter().any(|w| w.contains("experimental")));
        assert_eq!(p.dxgi_provider, Some(GraphicsBackend::Dxvk));

        let p = plan(
            &apis(&[GraphicsApi::D3D9]),
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        assert_eq!(p.backends(), vec![GraphicsBackend::Dxvk]);
        assert_eq!(p.dll_overrides.get("d3d9").unwrap(), "native,builtin");
    }

    #[test]
    fn profile_and_forced_overrides() {
        let section = GraphicsSection {
            d3d11: Some(GraphicsBackend::Dxvk),
            vulkan: Some(VulkanDriver::KosmicKrisp),
            ..Default::default()
        };
        let p = plan(
            &apis(&[GraphicsApi::D3D11]),
            &section,
            None,
            DebugOptions::default(),
            None,
        );
        assert_eq!(p.assignments[0].backend, GraphicsBackend::Dxvk);
        assert_eq!(p.assignments[0].reason, "game profile");
        assert_eq!(p.vulkan, Some(VulkanDriver::KosmicKrisp));

        let p = plan(
            &apis(&[GraphicsApi::D3D11, GraphicsApi::D3D9]),
            &section,
            Some(GraphicsBackend::WineD3d),
            DebugOptions::default(),
            None,
        );
        assert!(p
            .assignments
            .iter()
            .all(|a| a.backend == GraphicsBackend::WineD3d));
        assert_eq!(p.dll_overrides.get("d3d9").unwrap(), "builtin");
    }

    #[test]
    fn forced_backend_that_cannot_apply() {
        let p = plan(
            &apis(&[GraphicsApi::D3D12]),
            &GraphicsSection::default(),
            Some(GraphicsBackend::Dxmt),
            DebugOptions::default(),
            None,
        );
        assert_eq!(p.assignments[0].backend, GraphicsBackend::Vkd3dProton);
        assert!(p.warnings.iter().any(|w| w.contains("cannot implement")));
    }

    #[test]
    fn no_api_detected() {
        let p = plan(
            &GraphicsApis::default(),
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        assert!(p.assignments.is_empty());
        assert!(p.dll_overrides.values().all(|m| m == "builtin"));
        assert_eq!(p.dll_overrides.len(), MANAGED_DLLS.len());
        assert!(p.warnings.is_empty());
    }

    #[test]
    fn debug_env_only_when_requested() {
        let a = apis(&[GraphicsApi::D3D9]);
        let quiet = plan(
            &a,
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        assert!(quiet.env.is_empty());
        let loud = plan(
            &a,
            &GraphicsSection::default(),
            None,
            DebugOptions {
                logs: true,
                vulkan_validation: true,
                metal_validation: false,
            },
            Some(Path::new("/tmp/logs")),
        );
        assert_eq!(loud.env.get("DXVK_LOG_PATH").unwrap(), "/tmp/logs");
        assert!(loud.env.contains_key("VK_INSTANCE_LAYERS"));
        assert!(!loud.env.contains_key("MTL_DEBUG_LAYER"));
    }

    #[test]
    fn formats_overrides() {
        let mut o = BTreeMap::new();
        o.insert("d3d11".to_string(), "native,builtin".to_string());
        o.insert("dxgi".to_string(), "native,builtin".to_string());
        o.insert("d3d9".to_string(), "builtin".to_string());
        o.insert("winemenubuilder.exe".to_string(), "disabled".to_string());
        assert_eq!(
            format_dll_overrides(&o),
            "winemenubuilder.exe=;d3d9=b;d3d11,dxgi=n,b"
        );
    }
}
