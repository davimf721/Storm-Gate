# Engineering Rules

## Goal
Build an open-source Windows compatibility runtime for macOS (Storm Gate),
built using Wine and technologies from the Proton ecosystem.

## Supported target
Apple Silicon first (M1+, macOS 14+). Windows x86_64 guests through Wine
x86_64 under Rosetta 2. Windows ARM64 is Tier 2, Intel Macs Tier 3.

## Architecture
- Rust orchestration (`crates/`), one crate per subsystem.
- Wine-based Windows runtime (Proton's Wine branch, pinned in `manifests/`).
- DXMT preferred for D3D10/11; DXVK for D3D9; VKD3D-Proton (experimental) for D3D12.
- Open Vulkan-on-Metal path (MoltenVK, KosmicKrisp optional) for DXVK/VKD3D.
- Read `docs/architecture.md` and `docs/adr/` before changing structure.

## Rules
- Never introduce a proprietary dependency into the core runtime.
- Do not redistribute D3DMetal.
- Every runtime patch needs a regression test or reproduction and the metadata header in `patches/README.md`.
- Do not modify unrelated subsystems; keep changes small and in the smallest crate that owns the behaviour.
- Never silently change game profiles; write user profiles only after confirmation.
- Never bypass DRM or anti-cheat.
- Never put credentials or personal data in profiles, logs bundles or CI.
- Never mark a game or subsystem as supported without a test on real hardware.
- Do not touch the Proton tree outside `stormgate/` (ADR 0007).
- Run `make lint test` (formatting, clippy, tests, shellcheck, profile checks) before declaring a task complete.
- Prefer upstreamable fixes.
- Preserve third-party licenses.

## Commands
- `make test` — Rust tests, script lint, patch/profile/database validation.
- `make lint` — `cargo fmt --check` and `cargo clippy -D warnings`.
- `cargo run -p stormgate-cli -- <args>` — run the CLI; add `--dry-run` to print Wine commands instead of running them.
- `STORMGATE_HOME=<dir>` isolates all state; `STORMGATE_WINE=<wine>` uses an existing Wine (dev only).

## Specialised guidance
See `AGENTS.md` and `.ai/*.md` (wine, graphics, steam, macos, tests, release).
