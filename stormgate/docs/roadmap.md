# Roadmap

Order: **correctness → stability → compatibility → performance → UX.**

```text
BOOT → METAL → STEAM → FIRST GAME → D3D9 → D3D12 → COMPATIBILITY DB → GUI → PUBLIC ALPHA
```

## Versions

| Version | Scope |
|---|---|
| 0.0.x | Win32 + prefix + CLI |
| 0.1.x | D3D11 + DXMT |
| 0.2.x | Steam |
| 0.3.x | D3D9 + Vulkan (DXVK, MoltenVK) |
| 0.4.x | D3D12 experimental |
| 0.5.x | Compatibility database |
| 0.6.x | Repair engine |
| 0.7.x | GUI (SwiftUI over the Rust core) |
| 0.8.x | Packaging, signing, notarization |
| 0.9.x | Performance and regression tracking |
| 1.0 | Stable runtime for a documented set of games |

**1.0 means:** reproducible runtime, clear installation, side-by-side updates,
usable Steam, D3D9/10/11 stable for a known set, D3D12 documented,
compatibility DB, logs, automated tests, documentation, community process,
patch policy and organised licenses — not "every game works".

## Milestones

### BOOT — *Windows executable running reproducibly on Apple Silicon*
Issues #1–#5. Current milestone.

### METAL — *D3D11 sample rendered through DXMT → Metal*
Issues #6, #7.

### STEAM — *Windows Steam installed and usable*
Issues #9, #10.

### FIRST GAME — *first Windows game documented as playable*
Issues #11–#13 plus one compatibility entry with a real test.

### D3D12 — *first D3D12 workload through an open stack*
Issues #16–#18.

## Issue backlog

Status: ✅ done in this repository · 🟡 code exists, needs validation on a Mac · ⬜ not started

| # | Issue | Status | Acceptance criteria |
|---|---|---|---|
| 1 | Repository bootstrap | ✅ | Workspace, docs, CLAUDE.md, ADRs, CI, templates, licenses |
| 2 | macOS environment doctor | 🟡 | `stormgate doctor` reports chip, macOS, Rosetta, CLT, Metal, runtime components, Steam; `scripts/bootstrap.sh` checks the toolchain |
| 3 | Reproducible Wine 11 build | 🟡 | `make wine` builds the pinned Proton Wine as x86_64 Mach-O; `wine --version` works; patches recorded in `patches/wine` |
| 4 | Prefix creation | 🟡 | `stormgate prefix create NAME` runs wineboot, waits for wineserver, writes metadata, cleans up on failure (unit tested with a recording runner; verified with Wine on Linux) |
| 5 | Win32 smoke test | 🟡 | `stormgate test hello` and `stormgate test win32-window` pass on M1; notepad.exe opens |
| 6 | DXMT build | 🟡 | `make dxmt` produces d3d11/dxgi/d3d10core/winemetal DLLs and winemetal.so |
| 7 | D3D11 triangle | 🟡 | `stormgate test d3d11-triangle` passes through DXMT (pixel check) |
| 8 | MSync patch integration | 🟡 | `WITH_MSYNC=1 make wine` applies wine-msync; A/B (`server` vs `msync`) on CPU, frame time, crashes, deadlocks before any default change |
| 9 | Steam installer | 🟡 | `stormgate steam install` downloads over HTTPS, logs SHA-256, installs silently |
| 10 | Steam bootstrap | 🟡 | `stormgate steam start`: login works, library renders, a game installs and starts |
| 11 | Per-game prefix | 🟡 | `run` derives a prefix per game folder; Steam games use the `steam` prefix with per-game settings |
| 12 | Game profile schema | ✅ | Schema 1 with validation, shared/local modes, `profile validate` |
| 13 | Log collector | ✅ | Log sessions, `logs last`, `logs bundle` with redaction |
| 14 | DXVK build | 🟡 | `make dxvk` (DXVK-macOS by default) produces x64/x32 DLLs |
| 15 | D3D9 sample | ⬜ | `tests/windows/d3d9-triangle` passes through DXVK + MoltenVK |
| 16 | MoltenVK abstraction | 🟡 | `make moltenvk` with x86_64 slice; `VulkanDriver` selects ICD; KosmicKrisp build script |
| 17 | VKD3D-Proton macOS experiment | 🟡 | `make vkd3d`; port reviewed patches from VKD3D-Proton-MacOS into `patches/vkd3d-proton` |
| 18 | D3D12 feature probe | ⬜ | `tests/windows/d3d12-probe` prints supported features and exits 0 |
| 19 | Compatibility database | ✅ | `compat/steam/<appid>/` format, CI validation, profile resolution |
| 20 | Runtime packaging | 🟡 | `make package`: archive, SHA256SUMS, sources.lock, SBOM, licenses; `runtime install --sha256` |

### Next issues

| # | Issue | Notes |
|---|---|---|
| 21 | Input diagnostics | GameController.framework bridge for `stormgate input list` |
| 22 | Audio synthetic tests | WinMM, DirectSound, XAudio2, WASAPI programs in `tests/windows/` |
| 23 | Repair engine | On DXGI init failure try DXMT → DXVK → WineD3D; record results; never persist silently; rollback |
| 24 | Crash collector | `stormgate crash last`: Wine backtrace, versions, backend, sanitized env |
| 25 | Shader cache keys | Cache keyed by game, backend + version, GPU, macOS, runtime |
| 26 | Benchmark harness | Startup time, average FPS, 1% low, frame-time variance, RAM, CPU — local only |
| 27 | Self-hosted Mac CI | Runner for Metal/Rosetta/Wine/DXMT tests; no credentials |
| 28 | KosmicKrisp backend | Build Mesa's driver; `vulkan = "kosmickrisp"` on Metal 4 systems |

## Phases (from the development plan)

| Phase | Goal | Exit criterion |
|---|---|---|
| 0 | Reproduce an existing free stack by hand | notepad.exe, a Win32 app and a D3D11 demo run |
| 1 | Minimal Wine runtime | New prefixes are created reproducibly |
| 2 | Prefix manager | create, destroy, clone, repair, inspect, shell; isolated metadata |
| 3 | DXMT / D3D11 | Correct frame, fullscreen, mouse, audio, clean exit |
| 4 | MSync | A/B tested; default only after controlled regression testing |
| 5 | Steam | Login, library, install, launch |
| 6 | D3D9 | DXVK over MoltenVK/KosmicKrisp, WineD3D fallback |
| 7 | D3D12 | Probe → triangle → small game → AAA (in that order) |
| 8 | Compatibility profiles | `profile detect` suggests, user confirms, profile saved |
| 9 | Repair engine | Automatic diagnosis with recorded results and rollback |
| 10 | GUI | SwiftUI: Library, Game, Compatibility, Runtime, Logs, Settings |
| 11 | Installer | StormGate.app, ad-hoc signed → Developer ID + notarized |
| 12 | Community release | `0.1-alpha`, clearly experimental |

## Not now

Pretty launcher, website, login, cloud telemetry, sophisticated auto-updater,
100 games, Intel Macs, Windows ARM64, anti-cheat, competitive multiplayer,
App Store.
