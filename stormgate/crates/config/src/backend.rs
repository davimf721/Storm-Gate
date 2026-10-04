use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Translation layer used for a Direct3D API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphicsBackend {
    /// D3D10/11 -> Metal (preferred for D3D10/11).
    Dxmt,
    /// D3D9/10/11 -> Vulkan -> Vulkan-on-Metal.
    Dxvk,
    /// D3D12 -> Vulkan -> Vulkan-on-Metal (experimental).
    #[serde(alias = "vkd3d-moltenvk", alias = "vkd3d")]
    Vkd3dProton,
    /// Wine's built-in OpenGL based implementation.
    #[serde(rename = "wined3d")]
    WineD3d,
    /// Apple's D3DMetal. Optional, never bundled, installed by the user.
    #[serde(rename = "d3dmetal")]
    D3dMetal,
}

impl GraphicsBackend {
    pub const ALL: [GraphicsBackend; 5] = [
        GraphicsBackend::Dxmt,
        GraphicsBackend::Dxvk,
        GraphicsBackend::Vkd3dProton,
        GraphicsBackend::WineD3d,
        GraphicsBackend::D3dMetal,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            GraphicsBackend::Dxmt => "dxmt",
            GraphicsBackend::Dxvk => "dxvk",
            GraphicsBackend::Vkd3dProton => "vkd3d-proton",
            GraphicsBackend::WineD3d => "wined3d",
            GraphicsBackend::D3dMetal => "d3dmetal",
        }
    }

    /// Whether the backend is part of the open, redistributable stack.
    pub fn is_open(self) -> bool {
        !matches!(self, GraphicsBackend::D3dMetal)
    }

    /// Whether the backend is still considered experimental.
    pub fn is_experimental(self) -> bool {
        matches!(self, GraphicsBackend::Vkd3dProton)
    }

    /// Whether the backend renders through Vulkan (and thus needs a Vulkan driver).
    pub fn uses_vulkan(self) -> bool {
        matches!(self, GraphicsBackend::Dxvk | GraphicsBackend::Vkd3dProton)
    }
}

impl fmt::Display for GraphicsBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GraphicsBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "dxmt" => Ok(GraphicsBackend::Dxmt),
            "dxvk" => Ok(GraphicsBackend::Dxvk),
            "vkd3d-proton" | "vkd3d-moltenvk" | "vkd3d" => Ok(GraphicsBackend::Vkd3dProton),
            "wined3d" => Ok(GraphicsBackend::WineD3d),
            "d3dmetal" => Ok(GraphicsBackend::D3dMetal),
            other => Err(format!(
                "unknown graphics backend '{other}' (expected dxmt, dxvk, vkd3d-proton, wined3d or d3dmetal)"
            )),
        }
    }
}

/// Vulkan implementation running on top of Metal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VulkanDriver {
    #[default]
    #[serde(rename = "moltenvk")]
    MoltenVk,
    /// Mesa's Vulkan driver on Metal 4. Optional, requires a recent macOS.
    #[serde(rename = "kosmickrisp")]
    KosmicKrisp,
}

impl VulkanDriver {
    pub fn as_str(self) -> &'static str {
        match self {
            VulkanDriver::MoltenVk => "moltenvk",
            VulkanDriver::KosmicKrisp => "kosmickrisp",
        }
    }
}

impl fmt::Display for VulkanDriver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VulkanDriver {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "moltenvk" => Ok(VulkanDriver::MoltenVk),
            "kosmickrisp" => Ok(VulkanDriver::KosmicKrisp),
            other => Err(format!(
                "unknown Vulkan driver '{other}' (expected moltenvk or kosmickrisp)"
            )),
        }
    }
}

/// NT synchronization primitive implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncBackend {
    /// Default wineserver based synchronization.
    #[default]
    Server,
    /// Mach semaphore / ulock based synchronization (wine-msync).
    Msync,
}

impl SyncBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncBackend::Server => "server",
            SyncBackend::Msync => "msync",
        }
    }
}

impl fmt::Display for SyncBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_names() {
        for b in GraphicsBackend::ALL {
            assert_eq!(b.as_str().parse::<GraphicsBackend>().unwrap(), b);
        }
        assert_eq!(
            "vkd3d-moltenvk".parse::<GraphicsBackend>().unwrap(),
            GraphicsBackend::Vkd3dProton
        );
        assert!("crossover".parse::<GraphicsBackend>().is_err());
        assert!(!GraphicsBackend::D3dMetal.is_open());
    }
}
