use anyhow::{Context as _, Result};
use std::io::{self, BufRead, IsTerminal, Write};
use std::path::PathBuf;

use stormgate_compatibility::CompatDb;
use stormgate_config::Paths;
use stormgate_process::{LaunchSpec, Runner, SystemRunner};
use stormgate_runtime::{Runtime, RuntimeStore};

/// Environment variable pointing to a compatibility database checkout.
pub const COMPAT_ENV: &str = "STORMGATE_COMPAT_DIR";

/// Prints launches instead of running them (`--dry-run`). Storm Gate's own
/// bookkeeping (metadata, logs) is still written.
struct DryRunner;

impl Runner for DryRunner {
    fn run(&self, spec: &LaunchSpec) -> io::Result<i32> {
        println!("[dry-run] {}", spec.display());
        Ok(0)
    }
}

pub struct Ctx {
    pub paths: Paths,
    pub dry_run: bool,
    runner: Box<dyn Runner>,
}

impl Ctx {
    pub fn new(dry_run: bool) -> Result<Self> {
        let paths = Paths::from_env().context("cannot determine the Storm Gate data directory")?;
        paths
            .ensure()
            .with_context(|| format!("cannot create {}", paths.root().display()))?;
        let runner: Box<dyn Runner> = if dry_run {
            Box::new(DryRunner)
        } else {
            Box::new(SystemRunner)
        };
        Ok(Self {
            paths,
            dry_run,
            runner,
        })
    }

    pub fn runner(&self) -> &dyn Runner {
        self.runner.as_ref()
    }

    pub fn store(&self) -> RuntimeStore {
        RuntimeStore::new(self.paths.runtimes())
    }

    pub fn runtime(&self, requested: Option<&str>) -> Result<Runtime> {
        Ok(self.store().resolve(requested)?)
    }

    /// Compatibility database: `STORMGATE_COMPAT_DIR`, the runtime's bundled
    /// copy, or `./compat` in a source checkout.
    pub fn compat_db(&self, runtime: Option<&Runtime>) -> Option<CompatDb> {
        let candidates = [
            std::env::var_os(COMPAT_ENV).map(PathBuf::from),
            runtime.map(|r| r.root().join("compat")),
            Some(PathBuf::from("compat")),
        ];
        candidates
            .into_iter()
            .flatten()
            .find(|p| p.join("steam").is_dir())
            .map(CompatDb::new)
    }

    /// Asks a yes/no question. `assume_yes` skips the prompt; without a
    /// terminal the answer is "no" so nothing changes silently.
    pub fn confirm(&self, question: &str, assume_yes: bool) -> bool {
        if assume_yes {
            return true;
        }
        if !io::stdin().is_terminal() {
            eprintln!("{question} [y/N] (no terminal; pass --yes to confirm)");
            return false;
        }
        print!("{question} [y/N] ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if io::stdin().lock().read_line(&mut line).is_err() {
            return false;
        }
        matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
    }
}
