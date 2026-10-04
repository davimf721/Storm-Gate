//! End-to-end tests of the `stormgate` binary with a fake runtime.
//! Wine is never executed: launches go through `--dry-run`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use stormgate_pe::build_test_pe;

struct Env {
    _tmp: tempfile::TempDir,
    home: PathBuf,
    work: PathBuf,
}

impl Env {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let work = tmp.path().join("work");
        std::fs::create_dir_all(&work).unwrap();
        Self {
            home,
            work,
            _tmp: tmp,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_stormgate"))
            .args(args)
            .env("STORMGATE_HOME", &self.home)
            .env_remove("STORMGATE_WINE")
            .env_remove("STORMGATE_COMPAT_DIR")
            .current_dir(&self.work)
            .output()
            .unwrap()
    }

    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(out.status.success(), "stormgate {args:?} failed:\n{text}");
        text
    }

    fn fake_runtime(&self, version: &str) {
        let root = self.home.join("runtimes").join(version);
        for (dir, file) in [
            ("wine/bin", "wine"),
            ("wine/bin", "wineserver"),
            ("dxmt/x86_64-windows", "d3d11.dll"),
            ("dxmt/x86_64-windows", "d3d10core.dll"),
            ("dxmt/x86_64-windows", "dxgi.dll"),
            ("dxmt/x86_64-windows", "winemetal.dll"),
            ("dxvk/x64", "d3d9.dll"),
            ("moltenvk/icd.d", "MoltenVK_icd.json"),
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join(file), "").unwrap();
        }
        std::fs::write(
            root.join("runtime.toml"),
            format!("[runtime]\nversion = \"{version}\"\nfeatures = [\"msync\"]\n"),
        )
        .unwrap();
    }

    fn game(&self, dir: &str, imports: &[&str]) -> PathBuf {
        let d = self.work.join(dir);
        std::fs::create_dir_all(&d).unwrap();
        let exe = d.join("Game.exe");
        std::fs::write(&exe, build_test_pe(true, imports, &[])).unwrap();
        exe
    }
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn graphics_probe_is_offline() {
    let env = Env::new();
    let exe = env.game("Probe", &["KERNEL32.dll", "d3d11.dll", "dxgi.dll"]);
    let out = env.ok(&["graphics", "probe", s(&exe)]);
    assert!(out.contains("D3D11  -> dxmt"), "{out}");
    assert!(out.contains("WINEDLLOVERRIDES="), "{out}");
    assert!(
        !env.home.exists(),
        "probe must not create the data directory"
    );
    let json = env.ok(&["graphics", "probe", "--json", s(&exe)]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v["plan"]["assignments"][0]["backend"], "dxmt");
}

#[test]
fn run_pipeline_with_fake_runtime() {
    let env = Env::new();
    let exe = env.game("My Game", &["d3d11.dll"]);

    let out = env.run(&["run", s(&exe)]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no runtime installed"));

    env.fake_runtime("0.0.1");
    assert!(env.ok(&["runtime", "list"]).contains("0.0.1"));
    env.ok(&["runtime", "use", "0.0.1"]);
    assert!(env.ok(&["runtime", "list"]).contains("* 0.0.1"));

    let out = env.ok(&["--dry-run", "run", s(&exe), "--", "-windowed"]);
    assert!(out.contains("wineboot --init"), "{out}");
    assert!(out.contains("wineserver --wait"), "{out}");
    assert!(out.contains("d3d10core,d3d11,dxgi=n,b"), "{out}");
    assert!(out.contains("-windowed"), "{out}");
    assert!(out.contains("WINEPREFIX="), "{out}");
    assert!(out.contains("RetinaMode /t REG_SZ /d n /f"), "{out}");

    let prefix = env.home.join("prefixes/my-game");
    assert!(prefix.join("stormgate.toml").is_file());
    let link = prefix.join("drive_c/windows/system32/d3d11.dll");
    assert!(
        std::fs::symlink_metadata(&link).is_ok(),
        "DXMT d3d11.dll not deployed"
    );

    assert!(env.ok(&["prefix", "list"]).contains("my-game"));
    assert!(env
        .ok(&["prefix", "inspect", "my-game"])
        .contains("Initialized: no"));
    env.ok(&["prefix", "clone", "my-game", "copy"]);
    env.ok(&["prefix", "delete", "copy", "--yes"]);
    assert!(!env.home.join("prefixes/copy").exists());

    let last = env.ok(&["logs", "last"]);
    assert!(last.contains("graphics D3D11 -> dxmt"), "{last}");
    let zip = env.work.join("report.zip");
    env.ok(&["logs", "bundle", "-o", s(&zip)]);
    assert!(zip.is_file());
}

#[test]
fn steam_game_flow() {
    let env = Env::new();
    env.fake_runtime("0.0.1");
    let steam = env
        .home
        .join("prefixes/steam/drive_c/Program Files (x86)/Steam");
    let common = steam.join("steamapps/common/Half-Life");
    std::fs::create_dir_all(&common).unwrap();
    std::fs::write(steam.join("steam.exe"), "").unwrap();
    std::fs::write(
        env.home.join("prefixes/steam/stormgate.toml"),
        "schema = 1\nname = \"steam\"\ncreated_at = 0\nruntime = \"0.0.1\"\n",
    )
    .unwrap();
    std::fs::write(
        steam.join("steamapps/appmanifest_70.acf"),
        r#""AppState" { "appid" "70" "name" "Half-Life" "StateFlags" "4" "installdir" "Half-Life" }"#,
    )
    .unwrap();
    std::fs::write(
        common.join("hl.exe"),
        build_test_pe(false, &["d3d9.dll"], &[]),
    )
    .unwrap();

    assert!(env.ok(&["steam", "library"]).contains("Half-Life"));
    let added = env.ok(&["game", "add", "70", "--yes"]);
    assert!(added.contains("d3d9 = \"dxvk\""), "{added}");
    assert!(env.home.join("games/70.toml").is_file());
    assert!(env.ok(&["profile", "show", "70"]).contains("user profile"));

    let out = env.ok(&["--dry-run", "game", "run", "70"]);
    assert!(out.contains("-shutdown"), "{out}");
    assert!(out.contains("-applaunch 70"), "{out}");
    assert!(out.contains("d3d9=n,b"), "{out}");
}

#[test]
fn profile_validation_and_doctor() {
    let env = Env::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let template = root.join("profiles/template.toml");
    env.ok(&["profile", "validate", "--shared", s(&template)]);
    env.ok(&["profile", "check-db", s(&root.join("compat"))]);

    let bad = env.work.join("bad.toml");
    std::fs::write(
        &bad,
        "schema = 1\n[game]\nname = \"x\"\n[environment]\nSTEAM_PASSWORD = \"x\"\n",
    )
    .unwrap();
    assert!(!env.run(&["profile", "validate", s(&bad)]).status.success());

    if !cfg!(target_os = "macos") {
        let out = env.run(&["doctor"]);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).contains("Result: NOT READY"));
    }
}
