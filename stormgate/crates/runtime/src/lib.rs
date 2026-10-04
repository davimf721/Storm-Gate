//! Runtime management.
//!
//! A runtime is a self-contained directory with Wine and the graphics
//! translation layers, installed side by side under `runtimes/<version>/`:
//!
//! ```text
//! runtimes/0.1.0/
//! ├── runtime.toml               manifest with pinned sources and features
//! ├── wine/bin/{wine,wineserver}
//! ├── dxmt/{x86_64-windows,i386-windows}/*.dll
//! ├── dxvk/{x64,x32}/*.dll
//! ├── vkd3d-proton/{x64,x86}/*.dll
//! └── moltenvk/{lib/libMoltenVK.dylib,icd.d/MoltenVK_icd.json}
//! ```
//!
//! Runtimes are never modified in place: updates install a new version next
//! to the old one and `runtime use` switches the default (rollback included).

mod deploy;
mod hash;
mod manifest;
mod store;

pub use deploy::{deploy_backend_dlls, DeployedDll};
pub use hash::{sha256_file, sha256_hex};
pub use manifest::{RuntimeInfo, RuntimeManifest, SourceEntry};
pub use store::{Component, ComponentState, Runtime, RuntimeStore};

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("{0}: {1}")]
    Io(std::path::PathBuf, #[source] std::io::Error),
    #[error("invalid runtime manifest {0}: {1}")]
    Manifest(std::path::PathBuf, String),
    #[error("no runtime installed; build one with `make runtime` or install one with `stormgate runtime install`")]
    NoneInstalled,
    #[error("runtime {0} is not installed")]
    NotInstalled(String),
    #[error("runtime {0} is already installed")]
    AlreadyInstalled(String),
    #[error("checksum mismatch for {path}: expected {expected}, got {actual}")]
    Checksum {
        path: std::path::PathBuf,
        expected: String,
        actual: String,
    },
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;

pub(crate) fn io_err(path: &std::path::Path) -> impl FnOnce(std::io::Error) -> RuntimeError + '_ {
    move |e| RuntimeError::Io(path.to_path_buf(), e)
}
