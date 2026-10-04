use std::path::{Path, PathBuf};

use stormgate_config::GraphicsBackend;
use stormgate_graphics::{GraphicsApi, GraphicsPlan};

use crate::store::Runtime;
use crate::{io_err, Result, RuntimeError};

/// A DLL placed into a prefix for one launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeployedDll {
    pub dll: String,
    pub backend: GraphicsBackend,
    pub source: PathBuf,
    pub target: PathBuf,
}

fn provider(plan: &GraphicsPlan, dll: &str) -> Option<GraphicsBackend> {
    let by_api = |apis: &[GraphicsApi]| {
        plan.assignments
            .iter()
            .find(|a| apis.contains(&a.api))
            .map(|a| a.backend)
    };
    match dll {
        "dxgi" => plan.dxgi_provider,
        "d3d9" => by_api(&[GraphicsApi::D3D9]),
        "d3d11" | "d3d10core" => {
            by_api(&[GraphicsApi::D3D11, GraphicsApi::D3D10, GraphicsApi::Dxgi])
        }
        "d3d12" | "d3d12core" => by_api(&[GraphicsApi::D3D12]),
        _ => None,
    }
}

/// Links the native DLLs required by `plan` into the prefix's `system32`
/// (64-bit) and `syswow64` (32-bit) directories.
///
/// DLLs not claimed by the plan are left alone: the plan forces them to
/// Wine's builtins via `WINEDLLOVERRIDES`, so stale files are never loaded.
pub fn deploy_backend_dlls(
    runtime: &Runtime,
    prefix: &Path,
    plan: &GraphicsPlan,
) -> Result<Vec<DeployedDll>> {
    let mut wanted: Vec<(String, GraphicsBackend)> = Vec::new();
    for (dll, mode) in &plan.dll_overrides {
        if !mode.starts_with('n') {
            continue;
        }
        if let Some(b) = provider(plan, dll) {
            wanted.push((dll.clone(), b));
        }
    }
    if plan
        .assignments
        .iter()
        .any(|a| a.backend == GraphicsBackend::Dxmt)
    {
        // Bridge between DXMT's PE DLLs and its unix side.
        wanted.push(("winemetal".into(), GraphicsBackend::Dxmt));
    }

    let windows = prefix.join("drive_c/windows");
    let mut deployed = Vec::new();
    for (dll, backend) in wanted {
        for (x64, sysdir) in [(true, "system32"), (false, "syswow64")] {
            let Some(dir) = runtime.dll_dir(backend, x64) else {
                return Err(RuntimeError::Other(format!(
                    "runtime {} does not provide {backend}; install a Storm Gate runtime",
                    runtime.version()
                )));
            };
            let source = dir.join(format!("{dll}.dll"));
            if !source.is_file() {
                if x64 {
                    return Err(RuntimeError::Other(format!(
                        "runtime {} is missing {}",
                        runtime.version(),
                        source.display()
                    )));
                }
                // 32-bit builds of a backend are optional.
                continue;
            }
            let target_dir = windows.join(sysdir);
            std::fs::create_dir_all(&target_dir).map_err(io_err(&target_dir))?;
            let target = target_dir.join(format!("{dll}.dll"));
            place(&source, &target)?;
            deployed.push(DeployedDll {
                dll: dll.clone(),
                backend,
                source,
                target,
            });
        }
    }
    Ok(deployed)
}

fn place(source: &Path, target: &Path) -> Result<()> {
    if let Ok(meta) = std::fs::symlink_metadata(target) {
        if meta.is_dir() {
            return Err(RuntimeError::Other(format!(
                "{} is a directory",
                target.display()
            )));
        }
        std::fs::remove_file(target).map_err(io_err(target))?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target).map_err(io_err(target))
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(source, target)
            .map(|_| ())
            .map_err(io_err(target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tests::fake_runtime;
    use stormgate_graphics::{plan, DebugOptions, GraphicsApis, GraphicsSection};

    fn apis(list: &[GraphicsApi]) -> GraphicsApis {
        let mut s = GraphicsApis::default();
        for a in list {
            s.insert(*a);
        }
        s
    }

    #[test]
    fn deploys_dxmt() {
        let tmp = tempfile::tempdir().unwrap();
        let rt = Runtime::open(&fake_runtime(tmp.path(), "0.1.0", &[])).unwrap();
        let prefix = tmp.path().join("pfx");
        // Pretend wineboot left a builtin placeholder behind.
        let sys32 = prefix.join("drive_c/windows/system32");
        std::fs::create_dir_all(&sys32).unwrap();
        std::fs::write(sys32.join("d3d11.dll"), "wine builtin").unwrap();

        let p = plan(
            &apis(&[GraphicsApi::D3D11]),
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        let out = deploy_backend_dlls(&rt, &prefix, &p).unwrap();
        let names: Vec<_> = out.iter().map(|d| d.dll.as_str()).collect();
        assert_eq!(names, vec!["d3d10core", "d3d11", "dxgi", "winemetal"]);
        assert_eq!(
            std::fs::read_link(sys32.join("d3d11.dll")).unwrap(),
            rt.root().join("dxmt/x86_64-windows/d3d11.dll")
        );
        // Re-deploying is idempotent.
        deploy_backend_dlls(&rt, &prefix, &p).unwrap();
    }

    #[test]
    fn deploys_dxvk_both_arches_and_reports_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let rt = Runtime::open(&fake_runtime(tmp.path(), "0.1.0", &[])).unwrap();
        let prefix = tmp.path().join("pfx");
        let p = plan(
            &apis(&[GraphicsApi::D3D9]),
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        let out = deploy_backend_dlls(&rt, &prefix, &p).unwrap();
        assert_eq!(out.len(), 2);
        assert!(prefix.join("drive_c/windows/syswow64/d3d9.dll").exists());

        let p = plan(
            &apis(&[GraphicsApi::D3D12]),
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        assert!(deploy_backend_dlls(&rt, &prefix, &p).is_err());
    }
}
