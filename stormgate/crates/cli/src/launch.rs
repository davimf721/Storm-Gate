//! Shared launch pipeline used by `run`, `game run` and `test`.
//!
//! 1. resolve runtime  2. open/create prefix  3. plan graphics
//! 4. deploy backend DLLs  5. build environment  6. log  7. run  8. record

use anyhow::{bail, Context as _, Result};
use std::path::PathBuf;

use stormgate_compatibility::Detection;
use stormgate_config::{GameProfile, GraphicsBackend};
use stormgate_diagnostics::{LogSession, SystemInfo, SystemProbe};
use stormgate_graphics::{plan, DebugOptions};
use stormgate_prefix::PrefixManager;
use stormgate_process::{desktop_args, LaunchSpec, WineDebug, WineEnv};
use stormgate_runtime::deploy_backend_dlls;

use crate::cli::GraphicsFlags;
use crate::context::Ctx;

pub struct LaunchRequest {
    /// Log session label, e.g. `app-1091500` or `run-notepad`.
    pub label: String,
    pub prefix: String,
    pub appid: Option<u64>,
    pub runtime: Option<String>,
    pub profile: Option<GameProfile>,
    pub detection: Option<Detection>,
    pub forced_backend: Option<GraphicsBackend>,
    pub debug: DebugOptions,
    /// Arguments after `wine` (program first).
    pub wine_args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

impl LaunchRequest {
    pub fn debug_from(flags: &GraphicsFlags) -> DebugOptions {
        DebugOptions {
            logs: flags.debug_graphics,
            vulkan_validation: flags.vulkan_validation,
            metal_validation: flags.metal_validation,
        }
    }
}

pub struct LaunchOutcome {
    pub exit_code: i32,
    pub session: LogSession,
}

pub fn launch(ctx: &Ctx, req: LaunchRequest) -> Result<LaunchOutcome> {
    let session =
        LogSession::create(&ctx.paths, &req.label).context("cannot create log session")?;
    let system = SystemInfo::collect(&SystemProbe);
    let _ = session.write_system(&system);
    let note = |line: String| {
        session.log(&line);
        eprintln!("{line}");
    };

    // 1. Runtime (profile pin wins over the default).
    let pinned = req.profile.as_ref().and_then(|p| p.runtime.version.clone());
    let runtime = ctx.runtime(req.runtime.as_deref().or(pinned.as_deref()))?;
    session.log(&format!(
        "runtime {} at {}",
        runtime.version(),
        runtime.root().display()
    ));

    // 2. Prefix.
    let manager = PrefixManager::new(&ctx.paths, ctx.runner());
    let prefix = if manager.exists(&req.prefix) {
        manager.open(&req.prefix)?
    } else {
        note(format!(
            "creating prefix '{}' (first run takes a while)",
            req.prefix
        ));
        manager.create(
            &req.prefix,
            &runtime,
            req.appid,
            &session.file("wineboot.log"),
        )?
    };
    session.log(&format!("prefix {}", prefix.path.display()));

    // 3. Graphics plan.
    let default_profile = GameProfile::new(&req.label, req.appid);
    let profile = req.profile.as_ref().unwrap_or(&default_profile);
    let apis = req
        .detection
        .as_ref()
        .map(|d| d.apis.clone())
        .unwrap_or_default();
    let mut forced = req.forced_backend;
    if runtime.is_system() && forced.is_none() {
        note(
            "development Wine (STORMGATE_WINE): graphics backends unavailable, using wined3d"
                .into(),
        );
        forced = Some(GraphicsBackend::WineD3d);
    }
    let graphics_log = if req.debug.logs {
        Some(session.graphics_dir()?)
    } else {
        None
    };
    let gplan = plan(
        &apis,
        &profile.graphics,
        forced,
        req.debug,
        graphics_log.as_deref(),
    );
    for a in &gplan.assignments {
        session.log(&format!(
            "graphics {} -> {} ({})",
            a.api, a.backend, a.reason
        ));
    }
    if let Some(v) = gplan.vulkan {
        session.log(&format!("vulkan driver {v}"));
    }
    for w in &gplan.warnings {
        note(format!("warning: {w}"));
    }
    if gplan
        .assignments
        .iter()
        .any(|a| a.backend == GraphicsBackend::D3dMetal)
    {
        bail!("the D3DMetal backend is not integrated yet; it will remain optional and user-installed (docs/adr/0006-no-proprietary-core.md)");
    }
    if let Some(d) = &req.detection {
        if d.apis.is_empty() && d.pe.subsystem == stormgate_pe::Subsystem::Gui {
            note("note: no graphics API found in the imports; it may be loaded dynamically (WineD3D will be used)".into());
        }
        for ac in &d.anticheat {
            note(format!("warning: {}", ac.message()));
        }
    }

    // 4. Backend DLLs.
    let deployed = deploy_backend_dlls(&runtime, &prefix.path, &gplan)?;
    for d in &deployed {
        session.log(&format!(
            "deployed {} ({}) -> {}",
            d.dll,
            d.backend,
            d.target.display()
        ));
    }

    // 5. Environment.
    let (env, sync_warning) = WineEnv::new(&runtime, &prefix.path)
        .debug(if req.debug.logs {
            WineDebug::Verbose
        } else {
            WineDebug::Quiet
        })
        .sync(profile.sync.backend, &runtime);
    if let Some(w) = sync_warning {
        note(format!("warning: {w}"));
    }
    let env = env
        .graphics(&gplan, &runtime)
        .dll_overrides(&profile.dll_overrides)
        .profile_env(&profile.environment)
        .build();

    // winemac.drv reads Retina mode from the registry; set it explicitly so
    // the profile is the only source of truth.
    let retina = LaunchSpec::new(runtime.wine())
        .args(retina_reg_args(profile.display.retina))
        .env(env.clone())
        .log_file(session.file("wine.log"));
    let code = ctx.runner().run(&retina)?;
    if code != 0 {
        note(format!(
            "warning: could not set RetinaMode (exit code {code})"
        ));
    }

    let mut args = desktop_args(&profile.display, &req.label);
    args.extend(req.wine_args.iter().cloned());
    args.extend(profile.game.arguments.iter().cloned());
    let mut spec = LaunchSpec::new(runtime.wine())
        .args(args)
        .env(env)
        .log_file(session.file("wine.log"));
    if let Some(cwd) = &req.cwd {
        spec = spec.cwd(cwd);
    }

    // 6-8. Run and record.
    session.log(&format!("exec {}", spec.display()));
    eprintln!("logs: {}", session.dir().display());
    let exit_code = ctx
        .runner()
        .run(&spec)
        .with_context(|| format!("failed to run {}", runtime.wine().display()))?;
    session.log(&format!("exit code {exit_code}"));
    Ok(LaunchOutcome { exit_code, session })
}

/// `wine reg add` arguments for winemac.drv's Retina mode.
pub fn retina_reg_args(retina: bool) -> Vec<String> {
    [
        "reg",
        "add",
        r"HKCU\Software\Wine\Mac Driver",
        "/v",
        "RetinaMode",
        "/t",
        "REG_SZ",
        "/d",
        if retina { "y" } else { "n" },
        "/f",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}
