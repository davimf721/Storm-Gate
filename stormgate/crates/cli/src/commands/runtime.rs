use anyhow::Result;

use crate::cli::RuntimeCmd;
use crate::context::Ctx;

pub fn run(ctx: &Ctx, cmd: RuntimeCmd) -> Result<i32> {
    let store = ctx.store();
    match cmd {
        RuntimeCmd::List => {
            let current = store.current_version();
            let list = store.list()?;
            if list.is_empty() {
                println!("No runtimes installed in {}", store.dir().display());
                println!("Build one with `make runtime` or install a release with `stormgate runtime install`.");
            }
            for rt in list {
                let mark = if current.as_deref() == Some(rt.version()) {
                    "*"
                } else {
                    " "
                };
                let features = rt
                    .manifest()
                    .map(|m| m.runtime.features.join(","))
                    .unwrap_or_default();
                println!("{mark} {:<12} {features}", rt.version());
            }
        }
        RuntimeCmd::Install {
            archive,
            sha256,
            make_default,
        } => {
            let rt = store.install_archive(&archive, &sha256)?;
            println!(
                "Installed runtime {} at {}",
                rt.version(),
                rt.root().display()
            );
            if make_default {
                store.set_current(rt.version())?;
                println!("Runtime {} is now the default.", rt.version());
            } else {
                println!(
                    "Make it the default with: stormgate runtime use {}",
                    rt.version()
                );
            }
        }
        RuntimeCmd::Use { version } => {
            store.set_current(&version)?;
            println!("Runtime {version} is now the default.");
        }
        RuntimeCmd::Remove { version } => {
            store.remove(&version)?;
            println!("Removed runtime {version}.");
        }
        RuntimeCmd::Path { version } => {
            let rt = ctx.runtime(version.as_deref())?;
            println!("{}", rt.root().display());
        }
    }
    Ok(0)
}
