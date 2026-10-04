//! Process launching.
//!
//! The runtime owns the whole Wine environment. Variables such as
//! `WINEPREFIX` or `WINEDLLOVERRIDES` inherited from the user's shell are
//! removed so a launch is always reproducible from its profile and logs.

use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use stormgate_config::{DisplaySection, SyncBackend};
use stormgate_graphics::{format_dll_overrides, GraphicsPlan};
use stormgate_runtime::Runtime;

/// Variables removed from the inherited environment before every launch.
pub const SCRUBBED_ENV: &[&str] = &[
    "WINEPREFIX",
    "WINEDLLOVERRIDES",
    "WINEDEBUG",
    "WINEARCH",
    "WINELOADER",
    "WINESERVER",
    "WINEDLLPATH",
    "WINEMSYNC",
    "WINEESYNC",
    "WINEFSYNC",
    "DYLD_LIBRARY_PATH",
    "DYLD_FALLBACK_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "VK_ICD_FILENAMES",
    "VK_DRIVER_FILES",
    "VK_INSTANCE_LAYERS",
    "DXVK_HUD",
    "DXVK_LOG_PATH",
    "DXVK_LOG_LEVEL",
    "DXMT_LOG_PATH",
    "DXMT_LOG_LEVEL",
    "VKD3D_DEBUG",
    "VKD3D_SHADER_DEBUG",
    "VKD3D_LOG_FILE",
    "MTL_DEBUG_LAYER",
    "METAL_DEVICE_WRAPPER_TYPE",
];

/// Wine debug channel presets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WineDebug {
    /// Quiet, for normal play.
    #[default]
    Quiet,
    /// Errors and warnings, module loading and exceptions.
    Verbose,
}

impl WineDebug {
    fn value(self) -> &'static str {
        match self {
            WineDebug::Quiet => "-all,err+all",
            WineDebug::Verbose => "+loaddll,+seh,+module,err+all,warn+all",
        }
    }
}

/// Builds the environment for a Wine process.
#[derive(Debug, Clone)]
pub struct WineEnv {
    vars: BTreeMap<String, String>,
    overrides: BTreeMap<String, String>,
}

impl WineEnv {
    pub fn new(runtime: &Runtime, prefix: &Path) -> Self {
        let mut vars = BTreeMap::new();
        vars.insert("WINEPREFIX".into(), prefix.display().to_string());
        vars.insert("WINEDEBUG".into(), WineDebug::Quiet.value().into());
        vars.insert(
            "WINESERVER".into(),
            runtime.wineserver().display().to_string(),
        );
        let libs: Vec<String> = runtime
            .library_dirs()
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        if !libs.is_empty() {
            // Keep the system defaults after ours (see dyld(1)).
            let mut paths = libs;
            paths.push("/usr/local/lib".into());
            paths.push("/usr/lib".into());
            vars.insert("DYLD_FALLBACK_LIBRARY_PATH".into(), paths.join(":"));
        }
        let mut overrides = BTreeMap::new();
        // Avoid Wine creating macOS menu entries / file associations.
        overrides.insert("winemenubuilder.exe".into(), "disabled".into());
        Self { vars, overrides }
    }

    pub fn debug(mut self, debug: WineDebug) -> Self {
        self.vars.insert("WINEDEBUG".into(), debug.value().into());
        self
    }

    pub fn sync(mut self, sync: SyncBackend, runtime: &Runtime) -> (Self, Option<String>) {
        let mut warning = None;
        if sync == SyncBackend::Msync {
            if runtime.has_feature("msync") {
                self.vars.insert("WINEMSYNC".into(), "1".into());
            } else {
                warning = Some(format!(
                    "profile requests msync but runtime {} was built without it; using wineserver sync",
                    runtime.version()
                ));
            }
        }
        (self, warning)
    }

    pub fn graphics(mut self, plan: &GraphicsPlan, runtime: &Runtime) -> Self {
        self.overrides.extend(
            plan.dll_overrides
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        for (k, v) in &plan.env {
            self.vars.insert(k.clone(), v.clone());
        }
        if let Some(driver) = plan.vulkan {
            if let Some(icd) = runtime.vulkan_icd(driver) {
                let icd = icd.display().to_string();
                self.vars.insert("VK_ICD_FILENAMES".into(), icd.clone());
                self.vars.insert("VK_DRIVER_FILES".into(), icd);
            }
        }
        self
    }

    /// Applies profile DLL overrides (they win over the graphics plan).
    pub fn dll_overrides(mut self, overrides: &BTreeMap<String, String>) -> Self {
        self.overrides
            .extend(overrides.iter().map(|(k, v)| (k.clone(), v.clone())));
        self
    }

    /// Applies profile environment variables. Validation already rejected
    /// reserved and credential-like keys.
    pub fn profile_env(mut self, env: &BTreeMap<String, String>) -> Self {
        for (k, v) in env {
            self.vars.insert(k.clone(), v.clone());
        }
        self
    }

    pub fn var(mut self, key: &str, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    pub fn build(&self) -> BTreeMap<String, String> {
        let mut vars = self.vars.clone();
        if !self.overrides.is_empty() {
            vars.insert(
                "WINEDLLOVERRIDES".into(),
                format_dll_overrides(&self.overrides),
            );
        }
        vars
    }
}

/// Arguments to start a program through Wine's virtual desktop when asked.
pub fn desktop_args(display: &DisplaySection, label: &str) -> Vec<String> {
    if !display.virtual_desktop {
        return Vec::new();
    }
    let (w, h) = (
        display.width.unwrap_or(1920),
        display.height.unwrap_or(1080),
    );
    vec!["explorer".into(), format!("/desktop={label},{w}x{h}")]
}

/// A fully described process launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    /// stdout and stderr are appended here when set.
    pub log_file: Option<PathBuf>,
}

impl LaunchSpec {
    pub fn new(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: BTreeMap::new(),
            log_file: None,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn log_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.log_file = Some(path.into());
        self
    }

    /// Shell-like rendering for logs (environment first).
    pub fn display(&self) -> String {
        let mut parts: Vec<String> = self
            .env
            .iter()
            .map(|(k, v)| format!("{k}={}", quote(v)))
            .collect();
        parts.push(quote(&self.program.display().to_string()));
        parts.extend(self.args.iter().map(|a| quote(a)));
        parts.join(" ")
    }
}

fn quote(s: &str) -> String {
    if !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./=:,+@%".contains(&b))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Executes launch specs. Abstracted so orchestration logic can be tested
/// without Wine.
pub trait Runner {
    /// Runs to completion and returns the exit code (`-1` when killed by a signal).
    fn run(&self, spec: &LaunchSpec) -> io::Result<i32>;
}

/// Runs processes on the host.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run(&self, spec: &LaunchSpec) -> io::Result<i32> {
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args);
        for key in SCRUBBED_ENV {
            cmd.env_remove(key);
        }
        cmd.envs(&spec.env);
        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }
        if let Some(log) = &spec.log_file {
            if let Some(parent) = log.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let file = File::options().create(true).append(true).open(log)?;
            cmd.stdout(Stdio::from(file.try_clone()?));
            cmd.stderr(Stdio::from(file));
        }
        let status = cmd.status().map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("failed to start {}: {e}", spec.program.display()),
            )
        })?;
        Ok(status.code().unwrap_or(-1))
    }
}

/// Records launches instead of executing them (tests and `--dry-run`).
#[derive(Debug, Default)]
pub struct RecordingRunner {
    pub launches: std::cell::RefCell<Vec<LaunchSpec>>,
    /// Exit codes returned in order; `0` once exhausted.
    pub exit_codes: std::cell::RefCell<Vec<i32>>,
}

impl Runner for RecordingRunner {
    fn run(&self, spec: &LaunchSpec) -> io::Result<i32> {
        self.launches.borrow_mut().push(spec.clone());
        let mut codes = self.exit_codes.borrow_mut();
        Ok(if codes.is_empty() { 0 } else { codes.remove(0) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stormgate_graphics::{plan, DebugOptions, GraphicsApi, GraphicsApis, GraphicsSection};

    fn runtime(tmp: &Path, features: &str) -> Runtime {
        let root = tmp.join("rt");
        std::fs::create_dir_all(root.join("wine/bin")).unwrap();
        std::fs::create_dir_all(root.join("wine/lib")).unwrap();
        std::fs::create_dir_all(root.join("moltenvk/icd.d")).unwrap();
        std::fs::write(root.join("moltenvk/icd.d/MoltenVK_icd.json"), "{}").unwrap();
        std::fs::write(
            root.join("runtime.toml"),
            format!("[runtime]\nversion = \"0.1.0\"\nfeatures = [{features}]\n"),
        )
        .unwrap();
        Runtime::open(&root).unwrap()
    }

    #[test]
    fn builds_controlled_environment() {
        let tmp = tempfile::tempdir().unwrap();
        let rt = runtime(tmp.path(), "\"msync\"");
        let mut apis = GraphicsApis::default();
        apis.insert(GraphicsApi::D3D9);
        let gplan = plan(
            &apis,
            &GraphicsSection::default(),
            None,
            DebugOptions::default(),
            None,
        );
        let mut profile_overrides = BTreeMap::new();
        profile_overrides.insert("xinput1_3".to_string(), "native,builtin".to_string());

        let (env, warning) = WineEnv::new(&rt, Path::new("/pfx")).sync(SyncBackend::Msync, &rt);
        assert!(warning.is_none());
        let env = env
            .graphics(&gplan, &rt)
            .dll_overrides(&profile_overrides)
            .build();
        assert_eq!(env["WINEPREFIX"], "/pfx");
        assert_eq!(env["WINEMSYNC"], "1");
        assert!(env["VK_ICD_FILENAMES"].ends_with("MoltenVK_icd.json"));
        assert!(env["DYLD_FALLBACK_LIBRARY_PATH"].contains("wine/lib"));
        let overrides = &env["WINEDLLOVERRIDES"];
        assert!(overrides.contains("winemenubuilder.exe="));
        assert!(overrides.contains("d3d9,xinput1_3=n,b"), "{overrides}");
    }

    #[test]
    fn msync_falls_back_with_warning() {
        let tmp = tempfile::tempdir().unwrap();
        let rt = runtime(tmp.path(), "");
        let (env, warning) = WineEnv::new(&rt, Path::new("/pfx")).sync(SyncBackend::Msync, &rt);
        assert!(warning.is_some());
        assert!(!env.build().contains_key("WINEMSYNC"));
    }

    #[test]
    fn virtual_desktop_args() {
        let mut d = DisplaySection::default();
        assert!(desktop_args(&d, "x").is_empty());
        d.virtual_desktop = true;
        d.width = Some(1280);
        d.height = Some(720);
        assert_eq!(
            desktop_args(&d, "game"),
            vec!["explorer", "/desktop=game,1280x720"]
        );
    }

    #[test]
    fn system_runner_captures_logs_and_exit_code() {
        let tmp = tempfile::tempdir().unwrap();
        let log = tmp.path().join("logs/out.log");
        let mut env = BTreeMap::new();
        env.insert("STORMGATE_TEST".to_string(), "hello".to_string());
        let spec = LaunchSpec::new("/bin/sh")
            .arg("-c")
            .arg("echo $STORMGATE_TEST; echo prefix=${WINEPREFIX:-unset} >&2; exit 3")
            .env(env)
            .log_file(&log);
        let code = SystemRunner.run(&spec).unwrap();
        assert_eq!(code, 3);
        let out = std::fs::read_to_string(&log).unwrap();
        assert!(out.contains("hello"));
        assert!(out.contains("prefix=unset"));
    }

    #[test]
    fn display_quotes() {
        let spec = LaunchSpec::new("/usr/bin/wine").arg("C:\\Games\\My Game.exe");
        assert_eq!(spec.display(), "/usr/bin/wine 'C:\\Games\\My Game.exe'");
    }
}
