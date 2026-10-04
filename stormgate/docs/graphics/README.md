# Graphics

| API | Default backend | Path | Status |
|---|---|---|---|
| D3D8 | WineD3D | OpenGL (Wine) | fallback only |
| D3D9 | DXVK (DXVK-macOS) | Vulkan → MoltenVK → Metal | planned (0.3) |
| D3D10/11 | DXMT | Metal | target of milestone METAL (0.1) |
| D3D12 | VKD3D-Proton | Vulkan → MoltenVK → Metal | experimental (0.4) |
| Vulkan | native | MoltenVK / KosmicKrisp | via the Vulkan driver |
| OpenGL | Wine | Apple OpenGL | as-is |

Selection order for each API: `--backend` > profile `[graphics]` > default
policy. Inspect a decision without running anything:

```bash
stormgate graphics probe Game.exe
```

## DLL deployment

Each launch symlinks the chosen backend's DLLs into the prefix
(`system32` for 64-bit, `syswow64` for 32-bit) and sets `WINEDLLOVERRIDES`:
used DLLs `native,builtin`, every other Direct3D DLL `builtin`, so files from
earlier launches are never picked up by accident. Only one `dxgi.dll` can be
active: DXMT's for D3D10/11, DXVK's when D3D12 (VKD3D-Proton) is involved.

## Vulkan drivers

`[graphics] vulkan = "moltenvk"` (default) or `"kosmickrisp"` (Mesa, needs
Metal 4 / recent macOS). The runtime sets `VK_ICD_FILENAMES` and
`VK_DRIVER_FILES` to the driver's ICD inside the runtime.

## Debugging

`--debug-graphics` enables backend logs (`DXVK_LOG_*`, `DXMT_LOG_*`,
`VKD3D_DEBUG`, `MVK_CONFIG_LOG_LEVEL`) into the session's `graphics/`
directory. `--vulkan-validation` and `--metal-validation` add the heavy
validation layers; never leave them on for normal play.

## D3DMetal

Apple's D3DMetal is a useful laboratory to compare behaviour and
performance, but it is optional, never bundled and never required
([ADR 0006](../adr/0006-no-proprietary-core.md)). The `d3dmetal` backend
value is reserved for a future user-installed integration.

See also [d3d12.md](d3d12.md).
