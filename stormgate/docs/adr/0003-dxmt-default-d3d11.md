# ADR 0003: DXMT is the default for D3D10/11

## Context
D3D10/11 is the most common API for the games Storm Gate targets first.
DXMT implements D3D10/11 directly on Metal for Wine on macOS. The DXVK route
adds a Vulkan → Metal translation (MoltenVK) with portability-subset gaps.

## Decision
D3D10/11 default to DXMT. DXVK (DXVK-macOS flavour) and WineD3D are
fallbacks, selectable per profile or with `--backend`. Default policy:
D3D12 → VKD3D-Proton, D3D10/11 → DXMT, D3D9 → DXVK, otherwise WineD3D.

## Consequences
- The METAL milestone is defined by `d3d11-triangle` passing through DXMT.
- Only one dxgi.dll can be active; DXVK's wins when D3D12 is involved.
