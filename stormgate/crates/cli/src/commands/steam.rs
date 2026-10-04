use anyhow::{bail, Context as _, Result};

use stormgate_diagnostics::LogSession;
use stormgate_prefix::PrefixManager;
use stormgate_process::{LaunchSpec, WineEnv};
use stormgate_runtime::sha256_file;
use stormgate_steam::{bootstrap, process, SteamInstall, STEAM_PREFIX};

use crate::cli::SteamCmd;
use crate::context::Ctx;

pub fn run(ctx: &Ctx, cmd: SteamCmd) -> Result<i32> {
    match cmd {
        SteamCmd::Install {
            installer,
            sha256,
            runtime,
        } => install(ctx, installer, sha256, runtime.as_deref()),
        SteamCmd::Start { runtime, args } => {
            let rt = ctx.runtime(runtime.as_deref())?;
            let prefix = ctx.paths.prefix(STEAM_PREFIX);
            let steam = SteamInstall::find(&prefix)?;
            let session = LogSession::create(&ctx.paths, "steam")?;
            let env = WineEnv::new(&rt, &prefix).build();
            let spec = process::start_spec(&rt.wine(), &steam, env, &args)
                .log_file(session.file("steam.log"));
            session.log(&format!("exec {}", spec.display()));
            println!("Starting Steam. Log in inside the Steam window; Storm Gate never sees your credentials.");
            println!("logs: {}", session.dir().display());
            Ok(ctx.runner().run(&spec)?)
        }
        SteamCmd::Stop { runtime } => {
            stop(ctx, runtime.as_deref())?;
            Ok(0)
        }
        SteamCmd::Library => {
            let steam = SteamInstall::find(&ctx.paths.prefix(STEAM_PREFIX))?;
            let games = steam.games()?;
            if games.is_empty() {
                println!("No games installed yet. Install them from the Steam window.");
            }
            for g in games {
                let state = if g.manifest.is_fully_installed() {
                    ""
                } else {
                    " (installing)"
                };
                println!("{:>10}  {}{state}", g.manifest.appid, g.manifest.name);
            }
            Ok(0)
        }
    }
}

/// Asks a running Steam client in the Steam prefix to exit.
pub fn stop(ctx: &Ctx, runtime: Option<&str>) -> Result<()> {
    let rt = ctx.runtime(runtime)?;
    let prefix = ctx.paths.prefix(STEAM_PREFIX);
    let steam = SteamInstall::find(&prefix)?;
    let env = WineEnv::new(&rt, &prefix).build();
    let spec = LaunchSpec::new(rt.wine())
        .arg(steam.steam_exe().display().to_string())
        .arg("-shutdown")
        .env(env);
    ctx.runner().run(&spec)?;
    Ok(())
}

fn install(
    ctx: &Ctx,
    installer: Option<std::path::PathBuf>,
    expected: Option<String>,
    runtime: Option<&str>,
) -> Result<i32> {
    let rt = ctx.runtime(runtime)?;
    let session = LogSession::create(&ctx.paths, "steam-install")?;
    let mgr = PrefixManager::new(&ctx.paths, ctx.runner());
    let prefix = if mgr.exists(STEAM_PREFIX) {
        mgr.open(STEAM_PREFIX)?
    } else {
        println!("Creating the Steam prefix...");
        mgr.create(STEAM_PREFIX, &rt, None, &session.file("wineboot.log"))?
    };

    let installer = match installer {
        Some(p) => p,
        None if ctx.dry_run => {
            println!("[dry-run] download {}", bootstrap::INSTALLER_URL);
            ctx.paths.downloads().join(bootstrap::INSTALLER_NAME)
        }
        None => {
            println!("Downloading {}", bootstrap::INSTALLER_URL);
            bootstrap::download_installer(&ctx.paths.downloads())?
        }
    };
    if !ctx.dry_run {
        let actual = sha256_file(&installer)
            .with_context(|| format!("cannot hash {}", installer.display()))?;
        session.log(&format!(
            "installer {} sha256 {actual}",
            installer.display()
        ));
        println!("SteamSetup.exe sha256: {actual}");
        if let Some(expected) = expected {
            if !expected.trim().eq_ignore_ascii_case(&actual) {
                bail!("installer checksum mismatch: expected {expected}, got {actual}");
            }
        }
    }

    let env = WineEnv::new(&rt, &prefix.path).build();
    let spec = bootstrap::install_spec(&rt.wine(), &installer, env.clone())
        .log_file(session.file("steam.log"));
    session.log(&format!("exec {}", spec.display()));
    let code = ctx.runner().run(&spec)?;
    if code != 0 {
        bail!(
            "Steam installer exited with {code}; see {}",
            session.dir().display()
        );
    }
    let wait = LaunchSpec::new(rt.wineserver()).arg("--wait").env(env);
    ctx.runner().run(&wait)?;
    if !ctx.dry_run {
        SteamInstall::find(&prefix.path)?;
    }
    println!("Steam installed. Start it with: stormgate steam start");
    Ok(0)
}
