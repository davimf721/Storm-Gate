# ADR 0006: No proprietary dependency in the core

## Context
Apple's D3DMetal (Game Porting Toolkit) is capable, but its redistribution
terms are not clearly compatible with an open-source runtime, and depending
on it would make Storm Gate non-free.

## Decision
The core runtime works with the open stack only (Wine, DXMT, DXVK,
VKD3D-Proton, MoltenVK, KosmicKrisp). D3DMetal can only ever be an
**optional backend installed separately by the user**; it is never bundled,
downloaded automatically or required. Shared profiles may not use it.
GPTK and CrossOver are used as a laboratory to compare behaviour and
performance.

## Consequences
- The `d3dmetal` backend value exists but launches refuse it until a
  user-installed integration is designed.
- Profile validation in shared mode rejects proprietary backends.
