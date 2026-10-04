# Licensing

Storm Gate combines its own code with third-party components. Each part keeps
its own license.

## Storm Gate's own code

Everything under `stormgate/` that is not derived from a copyleft project —
the Rust crates, scripts, test programs, profiles and documentation — is
dual-licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option (`SPDX-License-Identifier: Apache-2.0 OR MIT`). Unless you
explicitly state otherwise, any contribution intentionally submitted for
inclusion is dual-licensed as above, without additional terms.

## Patches

Patches in `patches/<component>/` modify third-party code and are licensed
under **that component's license** (e.g. Wine patches are LGPL-2.1-or-later).

## Third-party components

| Component | Upstream | License | Shipped in runtime |
|---|---|---|---|
| Wine (Proton branch) | https://github.com/ValveSoftware/wine | LGPL-2.1-or-later | yes |
| DXMT | https://github.com/3Shain/dxmt | Zlib | yes |
| DXVK / DXVK-macOS | https://github.com/doitsujin/dxvk, https://github.com/Gcenx/DXVK-macOS | Zlib | yes |
| VKD3D-Proton | https://github.com/HansKristian-Work/vkd3d-proton | LGPL-2.1-or-later | experimental builds |
| MoltenVK | https://github.com/KhronosGroup/MoltenVK | Apache-2.0 | yes |
| wine-msync | https://github.com/marzent/wine-msync | LGPL-2.1 | when built with `WITH_MSYNC=1` |
| VKD3D-Proton-MacOS | https://github.com/metalsharp/VKD3D-Proton-MacOS | see upstream | reference for patches only |

The exact commits are pinned in [`manifests/runtime.toml`](manifests/runtime.toml).
`scripts/package.sh` copies each component's license files into the
runtime's `licenses/` directory and writes an SPDX SBOM next to the archive.
Licenses listed here must be re-checked against upstream whenever a pin
changes; the upstream `LICENSE`/`COPYING` file is authoritative.

### LGPL obligations

Wine and VKD3D-Proton are LGPL. Runtime releases must allow users to obtain
the corresponding source: the pinned commits plus `patches/` in this
repository at the release tag satisfy that, and the release notes must link
to them.

## The surrounding Proton tree

This directory lives inside a fork of Valve's Proton repository. Proton's
top-level code is BSD-3-Clause ([`../LICENSE.proton`](../LICENSE.proton));
its components keep their own licenses. Storm Gate does not modify those
files.

## Proprietary components

Apple's D3DMetal / Game Porting Toolkit is **not** part of Storm Gate and is
never redistributed. Rosetta 2 is an operating system component installed by
the user. See [ADR 0006](docs/adr/0006-no-proprietary-core.md).

## Trademarks

"Proton" and "Steam" are trademarks of Valve Corporation. Storm Gate is not
affiliated with or endorsed by Valve, Apple, or CodeWeavers, and is not
"Proton for macOS". See [ADR 0008](docs/adr/0008-naming.md).
