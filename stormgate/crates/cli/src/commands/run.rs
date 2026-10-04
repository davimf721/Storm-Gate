use anyhow::{bail, Context as _, Result};

use stormgate_compatibility::detect;
use stormgate_config::{slugify, GameProfile, ValidationMode};
use stormgate_pe::Machine;

use crate::cli::RunArgs;
use crate::context::Ctx;
use crate::launch::{launch, LaunchRequest};

pub fn run(ctx: &Ctx, args: RunArgs) -> Result<i32> {
    let exe = args
        .exe
        .canonicalize()
        .with_context(|| format!("cannot find {}", args.exe.display()))?;
    let detection = detect(&exe).with_context(|| format!("cannot inspect {}", exe.display()))?;
    match detection.machine {
        m if m.is_tier1() => {}
        Machine::Arm64 => eprintln!(
            "warning: {} is an ARM64 Windows binary; Windows ARM64 support is Tier 2 and not ready",
            exe.display()
        ),
        m => bail!("unsupported executable architecture: {}", m.as_str()),
    }

    let profile = match &args.profile {
        Some(path) => Some(GameProfile::load(path, ValidationMode::Local)?),
        None => None,
    };
    let dir = exe.parent().context("executable has no parent directory")?;
    let stem = exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let prefix = match args.prefix {
        Some(p) => p,
        None => slugify(
            &dir.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| stem.clone()),
        ),
    };
    let mut wine_args = vec![exe.display().to_string()];
    wine_args.extend(args.args);

    let outcome = launch(
        ctx,
        LaunchRequest {
            label: format!("run-{}", slugify(&stem)),
            prefix,
            appid: profile.as_ref().and_then(|p| p.game.appid),
            runtime: args.runtime,
            profile,
            detection: Some(detection),
            forced_backend: args.graphics.backend,
            debug: LaunchRequest::debug_from(&args.graphics),
            wine_args,
            cwd: Some(dir.to_path_buf()),
        },
    )?;
    if outcome.exit_code != 0 {
        eprintln!(
            "{stem} exited with code {}; logs in {}",
            outcome.exit_code,
            outcome.session.dir().display()
        );
    }
    Ok(outcome.exit_code)
}
