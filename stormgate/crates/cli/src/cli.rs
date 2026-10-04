use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use stormgate_config::GraphicsBackend;

#[derive(Debug, Parser)]
#[command(
    name = "stormgate",
    version,
    about = "Storm Gate: run Windows games on macOS with Wine and an open graphics stack",
    long_about = "Storm Gate is an open-source Windows game compatibility runtime for macOS,\n\
                  built using Wine and technologies from the Proton ecosystem.\n\n\
                  Data lives in ~/Library/Application Support/StormGate (override with STORMGATE_HOME)."
)]
pub struct Cli {
    /// Print the commands that would run instead of running them.
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check that this Mac and the runtime are ready.
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Manage side-by-side runtimes.
    #[command(subcommand)]
    Runtime(RuntimeCmd),
    /// Manage isolated Wine prefixes.
    #[command(subcommand)]
    Prefix(PrefixCmd),
    /// Run a Windows executable.
    Run(RunArgs),
    /// Windows Steam client.
    #[command(subcommand)]
    Steam(SteamCmd),
    /// Steam games.
    #[command(subcommand)]
    Game(GameCmd),
    /// Game profiles.
    #[command(subcommand)]
    Profile(ProfileCmd),
    /// Graphics diagnostics.
    #[command(subcommand)]
    Graphics(GraphicsCmd),
    /// Controller diagnostics.
    #[command(subcommand)]
    Input(InputCmd),
    /// Launch logs.
    #[command(subcommand)]
    Logs(LogsCmd),
    /// Run a synthetic Windows test program (see tests/windows).
    Test(TestArgs),
}

#[derive(Debug, Subcommand)]
pub enum RuntimeCmd {
    /// List installed runtimes.
    List,
    /// Install a runtime archive (.tar.gz) after verifying its SHA-256.
    Install {
        archive: PathBuf,
        /// Expected SHA-256 (from the release's SHA256SUMS).
        #[arg(long)]
        sha256: String,
        /// Also make it the default runtime.
        #[arg(long = "use")]
        make_default: bool,
    },
    /// Switch the default runtime (also used for rollback).
    Use { version: String },
    /// Remove an installed runtime.
    Remove { version: String },
    /// Print the directory of the resolved runtime.
    Path {
        #[arg(long)]
        version: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum PrefixCmd {
    /// Create and initialize a prefix (runs wineboot).
    Create {
        name: String,
        #[arg(long)]
        runtime: Option<String>,
    },
    /// Delete a prefix managed by Storm Gate.
    #[command(alias = "destroy")]
    Delete {
        name: String,
        /// Do not ask for confirmation.
        #[arg(long, short)]
        yes: bool,
    },
    /// Copy a prefix.
    Clone { source: String, dest: String },
    /// Re-run wineboot --update on a prefix.
    Repair {
        name: String,
        #[arg(long)]
        runtime: Option<String>,
    },
    /// Show prefix metadata and health.
    Inspect { name: String },
    /// List prefixes.
    List,
    /// Open a shell with the prefix's Wine environment.
    Shell {
        name: String,
        #[arg(long)]
        runtime: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct GraphicsFlags {
    /// Force a graphics backend for every API it supports.
    #[arg(long, value_parser = parse_backend)]
    pub backend: Option<GraphicsBackend>,
    /// Enable Wine and backend logs (written to the log session).
    #[arg(long)]
    pub debug_graphics: bool,
    /// Enable Vulkan validation layers (slow).
    #[arg(long)]
    pub vulkan_validation: bool,
    /// Enable the Metal API validation layer (slow).
    #[arg(long)]
    pub metal_validation: bool,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Path to the .exe.
    pub exe: PathBuf,
    /// Prefix name (default: derived from the game folder name).
    #[arg(long)]
    pub prefix: Option<String>,
    /// Runtime version (default: current).
    #[arg(long)]
    pub runtime: Option<String>,
    /// Game profile TOML to apply.
    #[arg(long)]
    pub profile: Option<PathBuf>,
    #[command(flatten)]
    pub graphics: GraphicsFlags,
    /// Arguments passed to the game.
    #[arg(last = true)]
    pub args: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum SteamCmd {
    /// Create the Steam prefix and install the Windows Steam client.
    Install {
        /// Use a local SteamSetup.exe instead of downloading it.
        #[arg(long)]
        installer: Option<PathBuf>,
        /// Require the installer to match this SHA-256.
        #[arg(long)]
        sha256: Option<String>,
        #[arg(long)]
        runtime: Option<String>,
    },
    /// Start the Steam client (log in manually in its window).
    Start {
        #[arg(long)]
        runtime: Option<String>,
        /// Extra arguments for steam.exe.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Ask a running Steam client to exit.
    Stop {
        #[arg(long)]
        runtime: Option<String>,
    },
    /// List games installed in the Steam prefix.
    Library,
}

#[derive(Debug, Subcommand)]
pub enum GameCmd {
    /// Detect a Steam game and save a profile for it (asks first).
    Add {
        appid: u64,
        #[arg(long, short)]
        yes: bool,
    },
    /// Launch a Steam game.
    Run {
        appid: u64,
        /// Run the executable directly instead of through the Steam client.
        #[arg(long)]
        direct: bool,
        #[arg(long)]
        runtime: Option<String>,
        #[command(flatten)]
        graphics: GraphicsFlags,
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// List Steam games with their profile source.
    List,
}

#[derive(Debug, Subcommand)]
pub enum ProfileCmd {
    /// Inspect a Steam AppID or an .exe and suggest a profile.
    Detect {
        /// Steam AppID or path to an executable.
        target: String,
        /// Save the suggestion as the user profile (asks first).
        #[arg(long)]
        save: bool,
        #[arg(long, short)]
        yes: bool,
    },
    /// Validate profile files.
    Validate {
        files: Vec<PathBuf>,
        /// Apply the rules for profiles shared in the repository.
        #[arg(long)]
        shared: bool,
    },
    /// Validate a compatibility database directory.
    CheckDb { dir: PathBuf },
    /// Print the profile that would be used for a Steam AppID.
    Show { appid: u64 },
}

#[derive(Debug, Subcommand)]
pub enum GraphicsCmd {
    /// Show the APIs an executable uses and the backends Storm Gate would pick.
    Probe {
        exe: PathBuf,
        #[arg(long, value_parser = parse_backend)]
        backend: Option<GraphicsBackend>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum InputCmd {
    /// List connected controllers.
    List,
}

#[derive(Debug, Subcommand)]
pub enum LogsCmd {
    /// Show the most recent log session.
    Last {
        /// Steam AppID or session label.
        target: Option<String>,
    },
    /// Write a sanitized zip of the latest session.
    Bundle {
        /// Steam AppID or session label.
        target: Option<String>,
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
pub struct TestArgs {
    /// Test name (hello, win32-window, d3d11-triangle, ...).
    pub name: String,
    /// Directory with built test executables.
    #[arg(long, default_value = "tests/windows/build")]
    pub dir: PathBuf,
    #[arg(long)]
    pub runtime: Option<String>,
    #[command(flatten)]
    pub graphics: GraphicsFlags,
}

fn parse_backend(s: &str) -> Result<GraphicsBackend, String> {
    s.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_is_consistent() {
        Cli::command().debug_assert();
    }
}
