# Architecture Decision Records

Big decisions are recorded here so that contributors (and AI agents) do not
reinvent the architecture. To change one, add a new ADR that supersedes it.

| ADR | Decision | Status |
|---|---|---|
| [0001](0001-rust-core.md) | Rust for orchestration | Accepted |
| [0002](0002-apple-silicon-first.md) | Apple Silicon first, x86_64 guests via Rosetta 2 | Accepted |
| [0003](0003-dxmt-default-d3d11.md) | DXMT is the default for D3D10/11 | Accepted |
| [0004](0004-vulkan-abstraction.md) | Vulkan-on-Metal is abstracted (MoltenVK, KosmicKrisp) | Accepted |
| [0005](0005-per-game-prefix.md) | One prefix per game by default | Accepted |
| [0006](0006-no-proprietary-core.md) | No proprietary dependency in the core | Accepted |
| [0007](0007-subdirectory-in-proton-fork.md) | Storm Gate lives in `stormgate/` inside the Proton fork | Accepted |
| [0008](0008-naming.md) | Naming and trademarks | Accepted |

Template: Context · Decision · Consequences.
