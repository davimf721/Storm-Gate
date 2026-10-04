//! `stormgate` — open-source Windows game compatibility runtime for macOS.

mod cli;
mod commands;
mod context;
mod launch;

use clap::Parser;

fn main() {
    let args = cli::Cli::parse();
    let code = match commands::dispatch(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            1
        }
    };
    std::process::exit(code);
}
