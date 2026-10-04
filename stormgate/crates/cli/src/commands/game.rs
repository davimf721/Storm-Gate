use anyhow::{Context as _, Result};
use std::path::PathBuf;

use stormgate_compatibility::{
    detect, guess_executable, resolve_profile, suggest_profile, user_profile_path, Detection,
};
use stormgate_config::{GameProfile, ValidationMode};
use stormgate_steam::{InstalledGame, SteamInstall, STEAM_PREFIX};

use crate::cli::GameCmd;
use crate::context::Ctx;
use crate::launch::{launch, LaunchRequest};

/// Locates an installed Steam game and inspects its executable.
pub(crate) fn inspect_steam_game(
    ctx: &Ctx,
    appid: u64,
    profile: Option<&GameProfile>,
) -> Result<(
    SteamInstall,
    InstalledGame,
    Option<PathBuf>,
    Option<Detection>,
)> {
    let steam = SteamInstall::find(&ctx.paths.prefix(STEAM_PREFIX))?;
    let game = steam.find_game(appid)?;
    let exe = profile
        .and_then(|p| p.game.executable.as_ref())
        .map(|rel| game.install_dir.join(rel.replace('\\', "/")))
        .or_else(|| guess_executable(&game.install_dir));
    let detection = exe.as_deref().and_then(|e| detect(e).ok());
    Ok((steam, game, exe, detection))
}

pub(crate) fn print_detection(d: &Detection) {
    println!("Detected:");
    println!("  executable: {}", d.executable.display());
    println!("  arch:       {}", d.machine.as_str());
    println!("  graphics:   {}", d.apis);
    if !d.api_sources.is_empty() {
        println!("  (from {})", d.api_sources.join(", "));
    }
    for ac in &d.anticheat {
        println!("  ! {}", ac.message());
    }
}

/// Shows a suggested profile and saves it only after confirmation.
pub(crate) fn confirm_and_save(
    ctx: &Ctx,
    appid: u64,
    profile: &GameProfile,
    yes: bool,
) -> Result<bool> {
    println!("\nSuggested profile:\n\n{}", profile.to_toml());
    let path = user_profile_path(&ctx.paths, appid);
    if path.exists() {
        println!("A profile already exists at {}.", path.display());
        if !ctx.confirm("Replace it?", yes) {
            return Ok(false);
        }
    } else if !ctx.confirm(&format!("Save to {}?", path.display()), yes) {
        println!("Not saved.");
        return Ok(false);
    }
    let errors = profile.validate(ValidationMode::Local);
    anyhow::ensure!(
        errors.is_empty(),
        "suggested profile is invalid: {}",
        errors.join("; ")
    );
    profile.save(&path)?;
    println!("Saved {}", path.display());
    Ok(true)
}

pub fn run(ctx: &Ctx, cmd: GameCmd) -> Result<i32> {
    match cmd {
        GameCmd::Add { appid, yes } => {
            let (_, game, exe, detection) = inspect_steam_game(ctx, appid, None)?;
            println!("{} ({appid})", game.manifest.name);
            let detection = detection.with_context(|| match exe {
                Some(e) => format!("cannot inspect {}", e.display()),
                None => format!("no executable found in {}", game.install_dir.display()),
            })?;
            print_detection(&detection);
            let mut profile = suggest_profile(&game.manifest.name, Some(appid), &detection);
            if let Ok(rel) = detection.executable.strip_prefix(&game.install_dir) {
                profile.game.executable = Some(rel.display().to_string());
            }
            Ok(if confirm_and_save(ctx, appid, &profile, yes)? {
                0
            } else {
                1
            })
        }
        GameCmd::Run {
            appid,
            direct,
            runtime,
            graphics,
            args,
        } => {
            let probe_rt = ctx.runtime(runtime.as_deref()).ok();
            let db = ctx.compat_db(probe_rt.as_ref());
            let (profile, source) = resolve_profile(&ctx.paths, db.as_ref(), appid)?;
            let (steam, game, exe, detection) = inspect_steam_game(ctx, appid, profile.as_ref())?;
            eprintln!("{} ({appid}) using {}", game.manifest.name, source.label());

            let (wine_args, cwd) = if direct {
                let exe = exe.context("no executable found; set game.executable in the profile")?;
                let mut a = vec![exe.display().to_string()];
                a.extend(args);
                (a, exe.parent().map(PathBuf::from))
            } else {
                // A running client would start the game with *its* environment.
                super::steam::stop(ctx, runtime.as_deref())?;
                let mut a = vec![
                    steam.steam_exe().display().to_string(),
                    "-applaunch".to_string(),
                    appid.to_string(),
                ];
                a.extend(args);
                (a, Some(steam.root().to_path_buf()))
            };

            let outcome = launch(
                ctx,
                LaunchRequest {
                    label: format!("app-{appid}"),
                    prefix: STEAM_PREFIX.into(),
                    appid: Some(appid),
                    runtime,
                    profile,
                    detection,
                    forced_backend: graphics.backend,
                    debug: LaunchRequest::debug_from(&graphics),
                    wine_args,
                    cwd,
                },
            )?;
            Ok(outcome.exit_code)
        }
        GameCmd::List => {
            let steam = SteamInstall::find(&ctx.paths.prefix(STEAM_PREFIX))?;
            let rt = ctx.runtime(None).ok();
            let db = ctx.compat_db(rt.as_ref());
            for g in steam.games()? {
                let source = resolve_profile(&ctx.paths, db.as_ref(), g.manifest.appid)
                    .map(|(_, s)| s.label())
                    .unwrap_or("invalid profile");
                println!("{:>10}  {:<40} {source}", g.manifest.appid, g.manifest.name);
            }
            Ok(0)
        }
    }
}
