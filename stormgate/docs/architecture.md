# Architecture

Storm Gate does **not** port the Proton monorepo to macOS. Proton is built
around Linux, the Steam Runtime containers, fsync and Vulkan drivers that
Darwin does not have. Instead Storm Gate is a new macOS runtime that
consumes small, pinned forks of the components that matter and fills the
macOS-specific gaps ([ADR 0007](adr/0007-subdirectory-in-proton-fork.md)).

```text
                         ┌─────────────────┐
                         │   Storm Gate    │
                         │ CLI / (SwiftUI) │
                         └────────┬────────┘
                       ┌──────────▼──────────┐
                       │ Runtime orchestrator │   crates/cli (launch.rs)
                       └──────────┬──────────┘
          ┌───────────────────────┼──────────────────────┐
       Prefixes                 Steam               Profiles / compat DB
     crates/prefix          crates/steam        crates/config, crates/compatibility
          └───────────────────────┼──────────────────────┘
                                  ▼
                       Wine (Proton branch, x86_64)  ── Rosetta 2
                                  │
               ┌──────────────────┼──────────────────┐
             D3D9             D3D10/11             D3D12
             DXVK               DXMT           VKD3D-Proton
               │                  │                  │
             Vulkan               │                Vulkan
       MoltenVK | KosmicKrisp     │        MoltenVK | KosmicKrisp
               └──────────────────┼──────────────────┘
                                Metal → Apple Silicon
```

## Crates

| Crate | Responsibility |
|---|---|
| `config` | Data directory layout (`Paths`), game profile schema + validation, prefix metadata, backend enums, name validation |
| `pe` | Dependency-free PE reader: machine, subsystem, imports, delay imports → graphics APIs |
| `graphics` | Backend selection policy, DLL overrides, dxgi provider, debug/validation environment |
| `runtime` | Runtime manifests, side-by-side store (`list/use/install/remove`), component discovery, DLL deployment into prefixes, SHA-256 |
| `process` | Controlled Wine environment (`WineEnv`), scrubbing of inherited variables, `LaunchSpec`, `Runner` trait (real / recording / dry-run) |
| `prefix` | Isolated prefixes: create (wineboot + wineserver wait), repair, clone, delete (only managed ones), inspect |
| `steam` | VDF parser, library folders, app manifests, installer download, `-applaunch`, webhelper workarounds |
| `compatibility` | Compatibility DB (`compat/`), profile resolution, executable detection, anti-cheat detection |
| `diagnostics` | `doctor`, log sessions, sanitized log bundles, system snapshot |
| `input` | Controller discovery (macOS bridge pending) |
| `cli` | `stormgate` binary and the launch pipeline |

Dependencies point "down": `cli` → (`compatibility`, `steam`, `prefix`,
`diagnostics`) → (`process`, `runtime`) → (`graphics`) → (`pe`, `config`).
Every behaviour has a single owner so contributors can fix one subsystem
without learning the rest.

## Launch pipeline

`stormgate run game.exe` and `stormgate game run <appid>`:

1. **Inspect** the PE: architecture (x86/x86_64 = Tier 1), imported graphics
   APIs (exe + engine DLLs next to it), anti-cheat files (informational).
2. **Resolve the profile**: user profile (`games/<appid>.toml`) >
   compatibility database (`compat/steam/<appid>/profile.toml`) > detection.
3. **Resolve the runtime**: `--runtime`, profile pin, `STORMGATE_WINE`
   (development), `runtimes/current`, newest installed.
4. **Prefix**: open or create (`wineboot --init`, then `wineserver --wait`).
5. **Graphics plan**: one backend per API (forced > profile > policy),
   explicit `builtin` overrides for every unused Direct3D DLL.
6. **Deploy** backend DLLs as symlinks into `system32` / `syswow64`.
7. **Environment**: built from scratch — `WINEPREFIX`, `WINEDLLOVERRIDES`,
   `WINEDEBUG`, `WINEMSYNC`, `DYLD_FALLBACK_LIBRARY_PATH`, `VK_ICD_FILENAMES`,
   profile variables — after removing inherited Wine/DYLD/Vulkan variables.
8. **Run and record**: `launcher.log`, `wine.log`, `system.json`,
   `graphics/` in `logs/<date>/<label>-<time>/`.

## Data layout

```text
~/Library/Application Support/StormGate/     (STORMGATE_HOME overrides)
├── runtimes/<version>/   side-by-side runtimes, `current` file
├── prefixes/<name>/      one Wine prefix per app (+ shared `steam`)
├── games/<appid>.toml    user-confirmed profiles
├── cache/                downloads, shader caches
└── logs/<date>/<label>-<time>/
```

## Steam

The Windows Steam client runs in the shared `steam` prefix. Games are
started with `steam.exe -applaunch <appid>` so Steamworks behaves exactly as
on Windows. Because a running client would start the game with *its own*
environment, `game run` first asks Steam to shut down, then restarts it with
the game's environment. Per-game prefixes therefore apply to standalone
games; Steam games share the Steam prefix but still get per-game graphics,
sync and environment settings.

## Design records

See [docs/adr](adr/README.md).
