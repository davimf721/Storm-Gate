use serde::Serialize;
use std::fmt;

/// Graphics API inferred from a module's imports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphicsApi {
    D3D8,
    D3D9,
    D3D10,
    D3D11,
    D3D12,
    /// Imports dxgi.dll but no specific Direct3D runtime (loaded dynamically).
    Dxgi,
    Vulkan,
    OpenGL,
}

impl GraphicsApi {
    pub fn as_str(self) -> &'static str {
        match self {
            GraphicsApi::D3D8 => "D3D8",
            GraphicsApi::D3D9 => "D3D9",
            GraphicsApi::D3D10 => "D3D10",
            GraphicsApi::D3D11 => "D3D11",
            GraphicsApi::D3D12 => "D3D12",
            GraphicsApi::Dxgi => "DXGI",
            GraphicsApi::Vulkan => "Vulkan",
            GraphicsApi::OpenGL => "OpenGL",
        }
    }

    fn from_dll(dll: &str) -> Option<Self> {
        let dll = dll.to_ascii_lowercase();
        let stem = dll.strip_suffix(".dll").unwrap_or(&dll);
        Some(match stem {
            "d3d8" => GraphicsApi::D3D8,
            "d3d9" => GraphicsApi::D3D9,
            "d3d10" | "d3d10_1" | "d3d10core" => GraphicsApi::D3D10,
            "d3d11" => GraphicsApi::D3D11,
            "d3d12" | "d3d12core" => GraphicsApi::D3D12,
            "dxgi" => GraphicsApi::Dxgi,
            "vulkan-1" => GraphicsApi::Vulkan,
            "opengl32" => GraphicsApi::OpenGL,
            _ => return None,
        })
    }
}

impl fmt::Display for GraphicsApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Set of graphics APIs, ordered from oldest to newest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct GraphicsApis(Vec<GraphicsApi>);

impl GraphicsApis {
    pub fn from_imports<'a>(imports: impl IntoIterator<Item = &'a str>) -> Self {
        let mut set = Self::default();
        for dll in imports {
            if let Some(api) = GraphicsApi::from_dll(dll) {
                set.insert(api);
            }
        }
        set
    }

    pub fn insert(&mut self, api: GraphicsApi) {
        if let Err(pos) = self.0.binary_search(&api) {
            self.0.insert(pos, api);
        }
    }

    pub fn extend(&mut self, other: &GraphicsApis) {
        for api in &other.0 {
            self.insert(*api);
        }
    }

    pub fn contains(&self, api: GraphicsApi) -> bool {
        self.0.binary_search(&api).is_ok()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = GraphicsApi> + '_ {
        self.0.iter().copied()
    }
}

impl fmt::Display for GraphicsApis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("none detected");
        }
        let names: Vec<_> = self.0.iter().map(|a| a.as_str()).collect();
        f.write_str(&names.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_apis() {
        let apis =
            GraphicsApis::from_imports(["KERNEL32.DLL", "D3D11.dll", "dxgi.dll", "d3d9.dll"]);
        assert!(apis.contains(GraphicsApi::D3D11));
        assert!(apis.contains(GraphicsApi::D3D9));
        assert!(apis.contains(GraphicsApi::Dxgi));
        assert!(!apis.contains(GraphicsApi::D3D12));
        assert_eq!(apis.to_string(), "D3D9, D3D11, DXGI");
    }
}
