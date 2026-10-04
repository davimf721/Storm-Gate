use anyhow::{Context as _, Result};

use stormgate_compatibility::detect;
use stormgate_graphics::{format_dll_overrides, plan, DebugOptions, GraphicsSection};

use crate::cli::GraphicsCmd;

pub fn run(cmd: &GraphicsCmd) -> Result<i32> {
    let GraphicsCmd::Probe { exe, backend, json } = cmd;
    let d = detect(exe).with_context(|| format!("cannot inspect {}", exe.display()))?;
    let p = plan(
        &d.apis,
        &GraphicsSection::default(),
        *backend,
        DebugOptions::default(),
        None,
    );
    if *json {
        let out = serde_json::json!({ "detection": d, "plan": p });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(0);
    }
    super::game::print_detection(&d);
    println!("\nPlan:");
    for a in &p.assignments {
        println!(
            "  {:<6} -> {:<13} ({})",
            a.api.as_str(),
            a.backend.as_str(),
            a.reason
        );
    }
    if let Some(v) = p.vulkan {
        println!("  Vulkan driver: {v}");
    }
    if let Some(dxgi) = p.dxgi_provider {
        println!("  dxgi.dll from: {dxgi}");
    }
    println!(
        "  WINEDLLOVERRIDES={}",
        format_dll_overrides(&p.dll_overrides)
    );
    for w in &p.warnings {
        println!("  ! {w}");
    }
    Ok(0)
}
