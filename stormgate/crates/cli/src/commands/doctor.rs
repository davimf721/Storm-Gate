use anyhow::Result;

use stormgate_diagnostics::{run_doctor, SystemProbe};
use stormgate_steam::STEAM_PREFIX;

use crate::context::Ctx;

pub fn run(ctx: &Ctx, json: bool) -> Result<i32> {
    let runtime = ctx.store().resolve(None);
    let steam = ctx.paths.prefix(STEAM_PREFIX);
    let report = run_doctor(
        &SystemProbe,
        runtime.as_ref(),
        steam.is_dir().then_some(steam.as_path()),
    );
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", report.render());
    }
    Ok(if report.ready() { 0 } else { 1 })
}
