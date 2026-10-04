# Storm Gate

**An open-source Windows game compatibility runtime for macOS, built using
Wine and technologies from the Proton ecosystem.**

> 🇧🇷 [Leia em português](README.pt-BR.md)

Storm Gate aims to be what "Proton for Mac" should mean: a free, reproducible
runtime that runs Windows games on Apple Silicon without a subscription,
without proprietary dependencies, and without asking users to understand Wine
prefixes, DLL overrides or environment variables.

```bash
stormgate doctor
stormgate steam install
stormgate steam start
stormgate game run 1091500
stormgate run ~/Games/MyGame/game.exe
```

> **Status: pre-alpha (0.0.x, milestone BOOT).** The orchestration core,
> CLI, build scripts and test harness exist; the runtime has not yet been
> validated on real Apple Silicon hardware. No game is marked as supported.
> See the [roadmap](docs/roadmap.md).

## How it works

```text
Windows game (x86_64)
      │
Wine (Proton branch) ──── Rosetta 2 ──── Apple Silicon
      │
      ├── D3D10/11 ── DXMT ──────────────────────┐
      ├── D3D9 ────── DXVK ─────── Vulkan ──┐    │
      └── D3D12 ───── VKD3D-Proton ─ Vulkan ┤    │
                                MoltenVK /  │    │
                                KosmicKrisp ┘    │
                                      └──── Metal
```

The `stormgate` CLI (Rust) inspects the executable, picks the graphics
backend, creates an isolated prefix, deploys the right DLLs, controls the
whole Wine environment and records logs for every launch.
Details: [docs/architecture.md](docs/architecture.md).

## Principles

- **Free and open.** No essential feature needs CrossOver, a subscription, a
  cloud service or a proprietary backend. Apple's D3DMetal can only ever be
  an optional, user-installed backend.
- **Reproducible.** Every component is pinned by commit
  ([`manifests/runtime.toml`](manifests/runtime.toml)); releases ship
  `SHA256SUMS`, `sources.lock` and an SPDX SBOM.
- **Small fork.** Reuse Wine, DXMT, DXVK, VKD3D-Proton and MoltenVK; keep
  patches minimal, documented and upstreamed.
- **Legitimate compatibility.** Storm Gate never bypasses DRM or anti-cheat.

## Quick start (developers)

Requirements: Apple Silicon Mac (M1 or newer), macOS 14+, Rosetta 2, Xcode
Command Line Tools, Homebrew. An M1 MacBook Air with 8 GB is enough for
development; keep ~40 GB free or point `STORMGATE_BUILD_DIR` at an external
SSD.

```bash
git clone https://github.com/davimf721/Storm-Gate
cd Storm-Gate/stormgate
make bootstrap      # checks the toolchain, prints what is missing
make runtime        # builds Wine, DXMT, DXVK, MoltenVK and packages them
make install-runtime
make test
```

Before a runtime is built you can experiment with any existing Wine:

```bash
STORMGATE_WINE=/opt/homebrew/bin/wine cargo run -p stormgate-cli -- run notepad.exe
```

Full instructions: [docs/getting-started.md](docs/getting-started.md) and
[docs/building.md](docs/building.md).

## Repository layout

| Path | Contents |
|---|---|
| `crates/` | Rust workspace: `cli`, `runtime`, `prefix`, `process`, `steam`, `graphics`, `pe`, `input`, `config`, `diagnostics`, `compatibility` |
| `scripts/` | Bootstrap and reproducible build scripts |
| `manifests/` | Pinned sources (`runtime.toml`, `sources.lock`) |
| `patches/` | Per-component patch queues with metadata |
| `profiles/`, `compat/` | Profile template and the compatibility database |
| `tests/windows/` | Synthetic Windows programs (`hello`, `win32-window`, `d3d11-triangle`) |
| `docs/` | Architecture, guides, ADRs, roadmap |

This directory lives inside a fork of [Valve's Proton](https://github.com/ValveSoftware/Proton);
the Proton tree around it is left untouched so it can be rebased
([ADR 0007](docs/adr/0007-subdirectory-in-proton-fork.md)).

## Contributing

Bug reports with logs (`stormgate logs bundle`), game reports, profiles and
code are all welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md).
Profiles can be contributed without knowing C or Rust.

## License

Storm Gate's own code is dual-licensed under [Apache-2.0](LICENSE-APACHE) or
[MIT](LICENSE-MIT). Third-party components keep their licenses; see
[LICENSES.md](LICENSES.md).

Storm Gate is not affiliated with Valve, Apple or CodeWeavers. "Proton" and
"Steam" are trademarks of Valve Corporation.
