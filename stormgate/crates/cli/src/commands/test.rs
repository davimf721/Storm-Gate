use anyhow::{bail, Context as _, Result};

use stormgate_compatibility::detect;

use crate::cli::TestArgs;
use crate::context::Ctx;
use crate::launch::{launch, LaunchRequest};

/// Marker printed by every synthetic test program on success.
const PASS_MARKER: &str = "STORMGATE-TEST-PASS";

pub fn run(ctx: &Ctx, args: TestArgs) -> Result<i32> {
    stormgate_config::validate_name(&args.name).map_err(anyhow::Error::msg)?;
    let exe = args.dir.join(format!("{}.exe", args.name));
    if !exe.is_file() {
        bail!(
            "{} not found; build the tests with `make -C tests/windows` (needs mingw-w64)",
            exe.display()
        );
    }
    let exe = exe.canonicalize()?;
    let detection = detect(&exe).with_context(|| format!("cannot inspect {}", exe.display()))?;
    let outcome = launch(
        ctx,
        LaunchRequest {
            label: format!("test-{}", args.name),
            prefix: "stormgate-tests".into(),
            appid: None,
            runtime: args.runtime,
            profile: None,
            detection: Some(detection),
            forced_backend: args.graphics.backend,
            debug: LaunchRequest::debug_from(&args.graphics),
            wine_args: vec![exe.display().to_string()],
            cwd: exe.parent().map(Into::into),
        },
    )?;
    let log = std::fs::read_to_string(outcome.session.file("wine.log")).unwrap_or_default();
    let marker = ctx.dry_run || log.contains(&format!("{PASS_MARKER} {}", args.name));
    let exit_ok = outcome.exit_code == 0;
    println!(
        "{}: exit status {}",
        args.name,
        if exit_ok { "PASS" } else { "FAIL" }
    );
    println!(
        "{}: pass marker {}",
        args.name,
        if marker { "PASS" } else { "FAIL" }
    );
    Ok(if exit_ok && marker { 0 } else { 1 })
}
