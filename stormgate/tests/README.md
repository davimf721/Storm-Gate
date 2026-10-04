# Tests

| Directory | Contents |
|---|---|
| `windows/` | Synthetic Windows programs (`make -C tests/windows`, `stormgate test <name>`) |
| `games/` | Procedure for testing commercial games and recording results |
| (crates) | Unit tests live next to the code; CLI integration tests in `crates/cli/tests/` |

See [docs/development/testing.md](../docs/development/testing.md).

## Synthetic programs

| Test | Checks | Status |
|---|---|---|
| `hello` | console I/O, process exit code | ✅ |
| `win32-window` | window creation, painting, message loop | ✅ |
| `d3d11-triangle` | D3D11 device, HLSL compile, draw, readback, pixel check | ✅ |
| `d3d9-triangle` | D3D9 via DXVK | ⬜ issue #15 |
| `d3d12-probe` / `d3d12-triangle` | D3D12 via VKD3D-Proton | ⬜ issue #18 |
| `vulkan-triangle` | native Vulkan via MoltenVK | ⬜ |
| `xaudio`, `winmm`, `dsound`, `wasapi` | audio starts, device detected, no hang, device change | ⬜ issue #22 |
| `xinput` | controller enumeration | ⬜ issue #21 |
| `filesystem` | paths, case sensitivity, long names | ⬜ |
