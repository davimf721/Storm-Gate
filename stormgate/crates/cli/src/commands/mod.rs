mod doctor;
mod game;
mod graphics;
mod logs;
mod prefix;
mod profile;
mod run;
mod runtime;
mod steam;
mod test;

use anyhow::Result;

use crate::cli::{Cli, Command, InputCmd};
use crate::context::Ctx;

/// Runs a command and returns the process exit code.
pub fn dispatch(cli: Cli) -> Result<i32> {
    // Pure commands that must not create the data directory.
    match &cli.command {
        Command::Graphics(cmd) => return graphics::run(cmd),
        Command::Input(InputCmd::List) => return input_list(),
        Command::Profile(cmd) if profile::is_offline(cmd) => return profile::run_offline(cmd),
        _ => {}
    }
    let ctx = Ctx::new(cli.dry_run)?;
    match cli.command {
        Command::Doctor { json } => doctor::run(&ctx, json),
        Command::Runtime(cmd) => runtime::run(&ctx, cmd),
        Command::Prefix(cmd) => prefix::run(&ctx, cmd),
        Command::Run(args) => run::run(&ctx, args),
        Command::Steam(cmd) => steam::run(&ctx, cmd),
        Command::Game(cmd) => game::run(&ctx, cmd),
        Command::Profile(cmd) => profile::run(&ctx, cmd),
        Command::Logs(cmd) => logs::run(&ctx, cmd),
        Command::Test(args) => test::run(&ctx, args),
        Command::Graphics(_) | Command::Input(_) => unreachable!("handled above"),
    }
}

fn input_list() -> Result<i32> {
    match stormgate_input::list_controllers() {
        Ok(list) if list.is_empty() => {
            println!("No controllers connected.");
            Ok(0)
        }
        Ok(list) => {
            for c in list {
                println!("{c}");
            }
            Ok(0)
        }
        Err(e) => {
            eprintln!("{e}");
            eprintln!("Wine still handles XInput/DirectInput for connected controllers.");
            Ok(2)
        }
    }
}

/// Human readable byte size.
pub(crate) fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = bytes as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}
