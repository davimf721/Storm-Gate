use anyhow::{bail, Result};

use stormgate_prefix::PrefixManager;
use stormgate_process::{LaunchSpec, Runner, SystemRunner, WineEnv};

use super::human_size;
use crate::cli::PrefixCmd;
use crate::context::Ctx;
use stormgate_diagnostics::LogSession;

pub fn run(ctx: &Ctx, cmd: PrefixCmd) -> Result<i32> {
    let mgr = PrefixManager::new(&ctx.paths, ctx.runner());
    match cmd {
        PrefixCmd::Create { name, runtime } => {
            let rt = ctx.runtime(runtime.as_deref())?;
            let session = LogSession::create(&ctx.paths, &format!("prefix-{name}"))?;
            println!("Creating prefix '{name}' with runtime {}...", rt.version());
            let p = mgr.create(&name, &rt, None, &session.file("wineboot.log"))?;
            println!("Created {}", p.path.display());
        }
        PrefixCmd::Delete { name, yes } => {
            let p = mgr.open(&name)?;
            if !ctx.confirm(
                &format!("Delete prefix '{name}' at {}?", p.path.display()),
                yes,
            ) {
                println!("Nothing deleted.");
                return Ok(1);
            }
            mgr.delete(&name)?;
            println!("Deleted prefix '{name}'.");
        }
        PrefixCmd::Clone { source, dest } => {
            let p = mgr.clone_prefix(&source, &dest)?;
            println!("Cloned '{source}' to {}", p.path.display());
        }
        PrefixCmd::Repair { name, runtime } => {
            let rt = ctx.runtime(runtime.as_deref())?;
            let session = LogSession::create(&ctx.paths, &format!("prefix-{name}"))?;
            mgr.repair(&name, &rt, &session.file("wineboot.log"))?;
            println!("Repaired '{name}' with runtime {}.", rt.version());
        }
        PrefixCmd::Inspect { name } => {
            let r = mgr.inspect(&name)?;
            let m = &r.prefix.metadata;
            println!("Name:        {}", m.name);
            println!("Path:        {}", r.prefix.path.display());
            println!("Runtime:     {}", m.runtime);
            println!("Arch:        {}", m.arch);
            println!(
                "Created:     {}",
                stormgate_config::time::rfc3339(m.created_at)
            );
            if let Some(appid) = m.appid {
                println!("AppID:       {appid}");
            }
            if let Some(src) = &m.cloned_from {
                println!("Cloned from: {src}");
            }
            println!("Size:        {}", human_size(r.size_bytes));
            println!("Initialized: {}", if r.initialized { "yes" } else { "no" });
            if !r.missing.is_empty() {
                println!("Missing:     {}", r.missing.join(", "));
                println!("Fix with:    stormgate prefix repair {name}");
            }
        }
        PrefixCmd::List => {
            let list = mgr.list()?;
            if list.is_empty() {
                println!("No prefixes. Create one with `stormgate prefix create <name>`.");
            }
            for p in list {
                println!("{:<24} runtime {}", p.metadata.name, p.metadata.runtime);
            }
        }
        PrefixCmd::Shell { name, runtime } => {
            let p = mgr.open(&name)?;
            let rt = ctx.runtime(runtime.as_deref())?;
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
            let wine_bin = rt.wine().parent().map(|p| p.display().to_string());
            let mut env = WineEnv::new(&rt, &p.path).build();
            if let Some(bin) = wine_bin {
                let path = std::env::var("PATH").unwrap_or_default();
                env.insert("PATH".into(), format!("{bin}:{path}"));
            }
            env.insert("STORMGATE_PREFIX".into(), name.clone());
            println!("Entering prefix '{name}' (exit the shell to leave).");
            let spec = LaunchSpec::new(shell).env(env);
            if ctx.dry_run {
                ctx.runner().run(&spec)?;
                return Ok(0);
            }
            let code = SystemRunner.run(&spec)?;
            if code != 0 {
                bail!("shell exited with {code}");
            }
        }
    }
    Ok(0)
}
