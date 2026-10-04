use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;

use stormgate_config::{validate_name, GraphicsBackend, VulkanDriver};

use crate::manifest::RuntimeManifest;
use crate::{hash, io_err, Result, RuntimeError};

const MANIFEST_FILE: &str = "runtime.toml";
const CURRENT_FILE: &str = "current";

/// Development override: run with an existing Wine (e.g. a Homebrew build)
/// before a Storm Gate runtime has been built. Phase 0 only.
pub const SYSTEM_WINE_ENV: &str = "STORMGATE_WINE";

/// Pieces of a runtime that `doctor` reports on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    Wine,
    Dxmt,
    Dxvk,
    Vkd3dProton,
    MoltenVk,
    KosmicKrisp,
    Msync,
}

impl Component {
    pub const ALL: [Component; 7] = [
        Component::Wine,
        Component::Dxmt,
        Component::Dxvk,
        Component::MoltenVk,
        Component::Vkd3dProton,
        Component::KosmicKrisp,
        Component::Msync,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Component::Wine => "Wine",
            Component::Dxmt => "DXMT",
            Component::Dxvk => "DXVK",
            Component::Vkd3dProton => "VKD3D-Proton",
            Component::MoltenVk => "MoltenVK",
            Component::KosmicKrisp => "KosmicKrisp",
            Component::Msync => "MSync",
        }
    }

    pub fn is_experimental(self) -> bool {
        matches!(self, Component::Vkd3dProton | Component::KosmicKrisp)
    }

    pub fn is_optional(self) -> bool {
        matches!(
            self,
            Component::Vkd3dProton | Component::KosmicKrisp | Component::Msync
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentState {
    Present(PathBuf),
    Missing(PathBuf),
}

/// Where a runtime came from.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Kind {
    Bundled,
    /// An existing `wine` binary given through `STORMGATE_WINE`.
    System(PathBuf),
}

/// An installed (or system-provided) runtime.
#[derive(Debug, Clone)]
pub struct Runtime {
    version: String,
    root: PathBuf,
    kind: Kind,
    manifest: Option<RuntimeManifest>,
}

impl Runtime {
    /// Loads a runtime directory containing `runtime.toml`.
    pub fn open(root: &Path) -> Result<Self> {
        let manifest = RuntimeManifest::load(&root.join(MANIFEST_FILE))?;
        Ok(Self {
            version: manifest.runtime.version.clone(),
            root: root.to_path_buf(),
            kind: Kind::Bundled,
            manifest: Some(manifest),
        })
    }

    /// Wraps an existing `wine` binary (development only).
    pub fn system(wine: &Path) -> Result<Self> {
        if !wine.is_file() {
            return Err(RuntimeError::Other(format!(
                "{SYSTEM_WINE_ENV}={} does not point to a file",
                wine.display()
            )));
        }
        let bin = wine.parent().unwrap_or(Path::new("/"));
        Ok(Self {
            version: "system".into(),
            root: bin.to_path_buf(),
            kind: Kind::System(wine.to_path_buf()),
            manifest: None,
        })
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> Option<&RuntimeManifest> {
        self.manifest.as_ref()
    }

    pub fn is_system(&self) -> bool {
        matches!(self.kind, Kind::System(_))
    }

    /// The `wine` loader. Wine 11 (new WoW64) ships a single `wine`; older
    /// builds used `wine64` for 64-bit prefixes.
    pub fn wine(&self) -> PathBuf {
        if let Kind::System(wine) = &self.kind {
            return wine.clone();
        }
        let bin = self.root.join("wine/bin");
        let wine = bin.join("wine");
        if !wine.exists() && bin.join("wine64").exists() {
            return bin.join("wine64");
        }
        wine
    }

    pub fn wineserver(&self) -> PathBuf {
        match &self.kind {
            Kind::Bundled => self.root.join("wine/bin/wineserver"),
            // Next to the given wine binary (Homebrew, distro packages).
            Kind::System(_) => self.root.join("wineserver"),
        }
    }

    /// Directories to put on `DYLD_FALLBACK_LIBRARY_PATH`.
    pub fn library_dirs(&self) -> Vec<PathBuf> {
        if self.is_system() {
            return Vec::new();
        }
        [self.root.join("wine/lib"), self.root.join("moltenvk/lib")]
            .into_iter()
            .filter(|p| p.is_dir())
            .collect()
    }

    /// Directory holding a backend's Windows DLLs for one architecture.
    pub fn dll_dir(&self, backend: GraphicsBackend, x64: bool) -> Option<PathBuf> {
        if self.is_system() {
            return None;
        }
        let rel = match (backend, x64) {
            (GraphicsBackend::Dxmt, true) => "dxmt/x86_64-windows",
            (GraphicsBackend::Dxmt, false) => "dxmt/i386-windows",
            (GraphicsBackend::Dxvk, true) => "dxvk/x64",
            (GraphicsBackend::Dxvk, false) => "dxvk/x32",
            (GraphicsBackend::Vkd3dProton, true) => "vkd3d-proton/x64",
            (GraphicsBackend::Vkd3dProton, false) => "vkd3d-proton/x86",
            (GraphicsBackend::WineD3d | GraphicsBackend::D3dMetal, _) => return None,
        };
        Some(self.root.join(rel))
    }

    /// Vulkan ICD manifest for a driver, used for `VK_ICD_FILENAMES`.
    pub fn vulkan_icd(&self, driver: VulkanDriver) -> Option<PathBuf> {
        let dir = match driver {
            VulkanDriver::MoltenVk => self.root.join("moltenvk/icd.d"),
            VulkanDriver::KosmicKrisp => self.root.join("kosmickrisp/icd.d"),
        };
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .ok()?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        entries.sort();
        entries.into_iter().next()
    }

    pub fn component(&self, component: Component) -> ComponentState {
        let probe = match component {
            Component::Wine => self.wine(),
            Component::Dxmt => self
                .dll_dir(GraphicsBackend::Dxmt, true)
                .unwrap_or_default()
                .join("d3d11.dll"),
            Component::Dxvk => self
                .dll_dir(GraphicsBackend::Dxvk, true)
                .unwrap_or_default()
                .join("d3d9.dll"),
            Component::Vkd3dProton => self
                .dll_dir(GraphicsBackend::Vkd3dProton, true)
                .unwrap_or_default()
                .join("d3d12.dll"),
            Component::MoltenVk => match self.vulkan_icd(VulkanDriver::MoltenVk) {
                Some(p) => p,
                None => self.root.join("moltenvk/icd.d/MoltenVK_icd.json"),
            },
            Component::KosmicKrisp => match self.vulkan_icd(VulkanDriver::KosmicKrisp) {
                Some(p) => p,
                None => self.root.join("kosmickrisp/icd.d"),
            },
            Component::Msync => {
                let path = self.root.join(MANIFEST_FILE);
                let enabled = self
                    .manifest
                    .as_ref()
                    .is_some_and(|m| m.has_feature("msync"));
                return if enabled {
                    ComponentState::Present(path)
                } else {
                    ComponentState::Missing(path)
                };
            }
        };
        if self.is_system() && component != Component::Wine {
            return ComponentState::Missing(probe);
        }
        if probe.exists() {
            ComponentState::Present(probe)
        } else {
            ComponentState::Missing(probe)
        }
    }

    pub fn has_feature(&self, feature: &str) -> bool {
        self.manifest
            .as_ref()
            .is_some_and(|m| m.has_feature(feature))
    }
}

/// Directory of side-by-side runtimes.
#[derive(Debug, Clone)]
pub struct RuntimeStore {
    dir: PathBuf,
}

impl RuntimeStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Installed runtimes, oldest version first.
    pub fn list(&self) -> Result<Vec<Runtime>> {
        let mut out = Vec::new();
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(RuntimeError::Io(self.dir.clone(), e)),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || !path.join(MANIFEST_FILE).is_file() {
                continue;
            }
            if let Ok(rt) = Runtime::open(&path) {
                out.push(rt);
            }
        }
        out.sort_by(|a, b| compare_versions(&a.version, &b.version));
        Ok(out)
    }

    pub fn current_version(&self) -> Option<String> {
        let text = std::fs::read_to_string(self.dir.join(CURRENT_FILE)).ok()?;
        let v = text.trim().to_string();
        (!v.is_empty()).then_some(v)
    }

    pub fn get(&self, version: &str) -> Result<Runtime> {
        validate_name(version).map_err(RuntimeError::Other)?;
        let path = self.dir.join(version);
        if !path.join(MANIFEST_FILE).is_file() {
            return Err(RuntimeError::NotInstalled(version.into()));
        }
        Runtime::open(&path)
    }

    /// Picks the runtime to use: an explicit version, then the
    /// `STORMGATE_WINE` development override, then `current`, then the newest.
    pub fn resolve(&self, requested: Option<&str>) -> Result<Runtime> {
        if let Some(v) = requested {
            return self.get(v);
        }
        if let Some(wine) = std::env::var_os(SYSTEM_WINE_ENV).filter(|v| !v.is_empty()) {
            return Runtime::system(Path::new(&wine));
        }
        if let Some(v) = self.current_version() {
            return self.get(&v);
        }
        self.list()?.pop().ok_or(RuntimeError::NoneInstalled)
    }

    /// Makes `version` the default runtime (also used for rollback).
    pub fn set_current(&self, version: &str) -> Result<()> {
        self.get(version)?;
        let path = self.dir.join(CURRENT_FILE);
        std::fs::write(&path, format!("{version}\n")).map_err(io_err(&path))
    }

    /// Installs a `.tar.gz` runtime archive after verifying its SHA-256.
    ///
    /// The archive is extracted into a staging directory and only moved into
    /// place once its manifest validates. The default runtime is not changed.
    pub fn install_archive(&self, archive: &Path, expected_sha256: &str) -> Result<Runtime> {
        let actual = hash::sha256_file(archive)?;
        if !actual.eq_ignore_ascii_case(expected_sha256.trim()) {
            return Err(RuntimeError::Checksum {
                path: archive.to_path_buf(),
                expected: expected_sha256.trim().to_ascii_lowercase(),
                actual,
            });
        }
        std::fs::create_dir_all(&self.dir).map_err(io_err(&self.dir))?;
        let staging = self.dir.join(format!(".staging-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&staging);
        std::fs::create_dir_all(&staging).map_err(io_err(&staging))?;
        let result = self.install_from_staging(archive, &staging);
        let _ = std::fs::remove_dir_all(&staging);
        result
    }

    fn install_from_staging(&self, archive: &Path, staging: &Path) -> Result<Runtime> {
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(staging)
            .status()
            .map_err(io_err(Path::new("tar")))?;
        if !status.success() {
            return Err(RuntimeError::Other(format!(
                "failed to extract {}",
                archive.display()
            )));
        }
        // Accept either the runtime at the archive root or in a single folder.
        let root = if staging.join(MANIFEST_FILE).is_file() {
            staging.to_path_buf()
        } else {
            let dirs: Vec<_> = std::fs::read_dir(staging)
                .map_err(io_err(staging))?
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.join(MANIFEST_FILE).is_file())
                .collect();
            match dirs.as_slice() {
                [one] => one.clone(),
                _ => {
                    return Err(RuntimeError::Other(
                        "archive does not contain exactly one runtime.toml".into(),
                    ))
                }
            }
        };
        let staged = Runtime::open(&root)?;
        let dest = self.dir.join(staged.version());
        if dest.exists() {
            return Err(RuntimeError::AlreadyInstalled(staged.version().into()));
        }
        std::fs::rename(&root, &dest).map_err(io_err(&dest))?;
        Runtime::open(&dest)
    }

    /// Removes an installed runtime. The current default cannot be removed.
    pub fn remove(&self, version: &str) -> Result<()> {
        let rt = self.get(version)?;
        if self.current_version().as_deref() == Some(version) {
            return Err(RuntimeError::Other(format!(
                "{version} is the current runtime; switch with `stormgate runtime use` first"
            )));
        }
        std::fs::remove_dir_all(rt.root()).map_err(io_err(rt.root()))
    }
}

/// Compares dotted versions numerically where possible (`0.10.0 > 0.9.1`).
pub(crate) fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |s: &str| -> Vec<String> {
        s.split(['.', '-', '+'])
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let (pa, pb) = (parts(a), parts(b));
    for (x, y) in pa.iter().zip(pb.iter()) {
        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            _ => x.cmp(y),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    pa.len().cmp(&pb.len())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn fake_runtime(dir: &Path, version: &str, features: &[&str]) -> PathBuf {
        let root = dir.join(version);
        std::fs::create_dir_all(root.join("wine/bin")).unwrap();
        std::fs::write(root.join("wine/bin/wine"), "#!/bin/sh\n").unwrap();
        std::fs::write(root.join("wine/bin/wineserver"), "#!/bin/sh\n").unwrap();
        for (d, f) in [
            ("dxmt/x86_64-windows", "d3d11.dll"),
            ("dxmt/x86_64-windows", "dxgi.dll"),
            ("dxmt/x86_64-windows", "d3d10core.dll"),
            ("dxmt/x86_64-windows", "winemetal.dll"),
            ("dxvk/x64", "d3d9.dll"),
            ("dxvk/x32", "d3d9.dll"),
            ("dxvk/x64", "dxgi.dll"),
            ("moltenvk/icd.d", "MoltenVK_icd.json"),
        ] {
            std::fs::create_dir_all(root.join(d)).unwrap();
            std::fs::write(root.join(d).join(f), "x").unwrap();
        }
        let features: Vec<String> = features.iter().map(|f| format!("\"{f}\"")).collect();
        std::fs::write(
            root.join(MANIFEST_FILE),
            format!(
                "[runtime]\nversion = \"{version}\"\nfeatures = [{}]\n",
                features.join(", ")
            ),
        )
        .unwrap();
        root
    }

    #[test]
    fn system_wine_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("lib/wine");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("wine64"), "").unwrap();
        let rt = Runtime::system(&bin.join("wine64")).unwrap();
        assert!(rt.is_system());
        assert_eq!(rt.wine(), bin.join("wine64"));
        assert_eq!(rt.wineserver(), bin.join("wineserver"));
        assert!(rt.dll_dir(GraphicsBackend::Dxmt, true).is_none());
        assert!(matches!(
            rt.component(Component::Wine),
            ComponentState::Present(_)
        ));
        assert!(Runtime::system(&bin.join("missing")).is_err());
    }

    #[test]
    fn versions_sort_numerically() {
        assert_eq!(compare_versions("0.10.0", "0.9.1"), Ordering::Greater);
        assert_eq!(compare_versions("0.4.1", "0.4.1"), Ordering::Equal);
        assert_eq!(compare_versions("0.4", "0.4.1"), Ordering::Less);
    }

    #[test]
    fn list_resolve_and_use() {
        let tmp = tempfile::tempdir().unwrap();
        let store = RuntimeStore::new(tmp.path());
        assert!(matches!(
            store.resolve(None),
            Err(RuntimeError::NoneInstalled)
        ));
        fake_runtime(tmp.path(), "0.9.0", &[]);
        fake_runtime(tmp.path(), "0.10.0", &["msync"]);
        let versions: Vec<_> = store
            .list()
            .unwrap()
            .iter()
            .map(|r| r.version().to_string())
            .collect();
        assert_eq!(versions, vec!["0.9.0", "0.10.0"]);
        assert_eq!(store.resolve(None).unwrap().version(), "0.10.0");
        store.set_current("0.9.0").unwrap();
        assert_eq!(store.resolve(None).unwrap().version(), "0.9.0");
        assert!(store.set_current("1.0.0").is_err());
        assert!(
            store.remove("0.9.0").is_err(),
            "current runtime must not be removable"
        );
        store.remove("0.10.0").unwrap();
        assert!(store.get("0.10.0").is_err());
        assert!(store.get("../etc").is_err());
    }

    #[test]
    fn components() {
        let tmp = tempfile::tempdir().unwrap();
        let rt = Runtime::open(&fake_runtime(tmp.path(), "0.1.0", &["msync"])).unwrap();
        assert!(matches!(
            rt.component(Component::Wine),
            ComponentState::Present(_)
        ));
        assert!(matches!(
            rt.component(Component::Dxmt),
            ComponentState::Present(_)
        ));
        assert!(matches!(
            rt.component(Component::MoltenVk),
            ComponentState::Present(_)
        ));
        assert!(matches!(
            rt.component(Component::Msync),
            ComponentState::Present(_)
        ));
        assert!(matches!(
            rt.component(Component::Vkd3dProton),
            ComponentState::Missing(_)
        ));
        assert!(rt.vulkan_icd(VulkanDriver::MoltenVk).is_some());
    }

    #[test]
    fn install_verifies_checksum() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        fake_runtime(&src, "0.2.0", &[]);
        let archive = tmp.path().join("rt.tar.gz");
        let ok = Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(&src)
            .arg("0.2.0")
            .status()
            .unwrap();
        assert!(ok.success());
        let store = RuntimeStore::new(tmp.path().join("runtimes"));
        let bad = store.install_archive(&archive, &"0".repeat(64));
        assert!(matches!(bad, Err(RuntimeError::Checksum { .. })));
        let sha = hash::sha256_file(&archive).unwrap();
        let rt = store.install_archive(&archive, &sha).unwrap();
        assert_eq!(rt.version(), "0.2.0");
        assert!(
            store.current_version().is_none(),
            "install must not switch default"
        );
        assert!(matches!(
            store.install_archive(&archive, &sha),
            Err(RuntimeError::AlreadyInstalled(_))
        ));
    }
}
