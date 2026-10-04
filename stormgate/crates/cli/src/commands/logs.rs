use anyhow::{Context as _, Result};
use std::path::PathBuf;

use stormgate_diagnostics::{bundle, last_session, Redactor};

use crate::cli::LogsCmd;
use crate::context::Ctx;

/// `1091500` -> `app-1091500`; anything else is used as a label as is.
fn label(target: Option<String>) -> Option<String> {
    target.map(|t| {
        if t.parse::<u64>().is_ok() {
            format!("app-{t}")
        } else {
            t
        }
    })
}

pub fn run(ctx: &Ctx, cmd: LogsCmd) -> Result<i32> {
    match cmd {
        LogsCmd::Last { target } => {
            let label = label(target);
            let dir = last_session(&ctx.paths, label.as_deref()).context("no log sessions yet")?;
            println!("{}", dir.display());
            if let Ok(text) = std::fs::read_to_string(dir.join("launcher.log")) {
                println!("\n{text}");
            }
            let wine = dir.join("wine.log");
            if let Ok(text) = std::fs::read_to_string(&wine) {
                let lines: Vec<&str> = text.lines().collect();
                let tail = &lines[lines.len().saturating_sub(30)..];
                println!("--- last {} lines of wine.log ---", tail.len());
                for l in tail {
                    println!("{l}");
                }
            }
            Ok(0)
        }
        LogsCmd::Bundle { target, output } => {
            let label = label(target);
            let name = format!(
                "stormgate-report-{}.zip",
                label.as_deref().unwrap_or("latest")
            );
            let out = output.unwrap_or_else(|| PathBuf::from(name));
            let n = bundle(&ctx.paths, label.as_deref(), &out, &Redactor::from_env())?;
            println!(
                "Wrote {} ({n} files, personal data redacted).",
                out.display()
            );
            println!("Review it before sharing; redaction is best effort.");
            Ok(0)
        }
    }
}
