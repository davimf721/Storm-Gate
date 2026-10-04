# Graphics agent

**Owns:** DXMT, DXVK (DXVK-macOS flavour), VKD3D-Proton, MoltenVK,
KosmicKrisp, shader bugs, Metal, `crates/graphics`, `crates/pe` API detection,
`tests/windows/d3d*`.

Rules:
- Default policy: D3D12 → VKD3D-Proton (experimental), D3D10/11 → DXMT,
  D3D9 → DXVK, fallback WineD3D. Change it only through an ADR.
- Every backend bug becomes a minimal reproduction (a synthetic test in
  `tests/windows` or a captured trace), not "game X is black".
- D3D12 order: feature probe → triangle → small game → AAA. Never start with AAA.
- Heavy validation (Vulkan layers, Metal validation) is opt-in only.
- Vulkan on macOS is a portability subset: never assume Linux behaviour;
  query features and fail with a clear message.
- D3DMetal is optional and never bundled or required.
