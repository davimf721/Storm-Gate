use anyhow::{Context as _, Result};
use std::path::Path;

use stormgate_compatibility::{detect, resolve_profile, suggest_profile, validate_tree};
use stormgate_config::{GameProfile, ValidationMode};

use super::game::{confirm_and_save, inspect_steam_game, print_detection};
use crate::cli::ProfileCmd;
use crate::context::Ctx;

pub fn is_offline(cmd: &ProfileCmd) -> bool {
    matches!(
        cmd,
        ProfileCmd::Validate { .. } | ProfileCmd::CheckDb { .. }
    )
}

/// Commands that never touch the data directory (used by CI).
pub fn run_offline(cmd: &ProfileCmd) -> Result<i32> {
    match cmd {
        ProfileCmd::Validate { files, shared } => {
            let mode = if *shared {
                ValidationMode::Shared
            } else {
                ValidationMode::Local
            };
            let mut failed = 0;
            for f in files {
                match GameProfile::load(f, mode) {
                    Ok(_) => println!("ok   {}", f.display()),
                    Err(e) => {
                        println!("FAIL {}: {e}", f.display());
                        failed += 1;
                    }
                }
            }
            Ok(if failed == 0 { 0 } else { 1 })
        }
        ProfileCmd::CheckDb { dir } => {
            let errors = validate_tree(dir);
            for e in &errors {
                println!("FAIL {e}");
            }
            if errors.is_empty() {
                println!("ok   {}", dir.display());
            }
            Ok(if errors.is_empty() { 0 } else { 1 })
        }
        _ => unreachable!("online command"),
    }
}

pub fn run(ctx: &Ctx, cmd: ProfileCmd) -> Result<i32> {
    match cmd {
        ProfileCmd::Detect { target, save, yes } => {
            if let Ok(appid) = target.parse::<u64>() {
                let (_, game, _, detection) = inspect_steam_game(ctx, appid, None)?;
                let detection =
                    detection.context("could not find or inspect the game executable")?;
                println!("{} ({appid}) via Steam", game.manifest.name);
                print_detection(&detection);
                let mut profile = suggest_profile(&game.manifest.name, Some(appid), &detection);
                if let Ok(rel) = detection.executable.strip_prefix(&game.install_dir) {
                    profile.game.executable = Some(rel.display().to_string());
                }
                if save {
                    return Ok(if confirm_and_save(ctx, appid, &profile, yes)? {
                        0
                    } else {
                        1
                    });
                }
                println!("\nSuggested profile:\n\n{}", profile.to_toml());
                println!("Save it with: stormgate profile detect {appid} --save");
            } else {
                let exe = Path::new(&target);
                let detection = detect(exe).with_context(|| format!("cannot inspect {target}"))?;
                print_detection(&detection);
                let name = exe
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                anyhow::ensure!(!save, "--save needs a Steam AppID; for executables redirect the output to a file and pass it with `stormgate run --profile`");
                println!("\n{}", suggest_profile(&name, None, &detection).to_toml());
            }
            Ok(0)
        }
        ProfileCmd::Show { appid } => {
            let rt = ctx.runtime(None).ok();
            let db = ctx.compat_db(rt.as_ref());
            match resolve_profile(&ctx.paths, db.as_ref(), appid)? {
                (Some(p), source) => {
                    println!("# source: {}\n{}", source.label(), p.to_toml());
                }
                (None, _) => println!("No profile for {appid}; automatic detection will be used."),
            }
            Ok(0)
        }
        other => run_offline(&other),
    }
}
