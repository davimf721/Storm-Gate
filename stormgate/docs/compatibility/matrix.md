# Compatibility matrix

Never mark a cell without a recorded test on that hardware/OS
(`status.json` → `tested_on`). `?` = untested.

## By chip

| Subsystem | M1 | M2 | M3 | M4 | Status |
|---|:-:|:-:|:-:|:-:|---|
| Win32 (hello, win32-window) | ? | ? | ? | ? | Pre-alpha |
| D3D9 (DXVK + MoltenVK) | ? | ? | ? | ? | Not started |
| D3D11 (DXMT) | ? | ? | ? | ? | Pre-alpha |
| D3D12 (VKD3D-Proton) | ? | ? | ? | ? | Experimental |
| XInput | ? | ? | ? | ? | Not started |
| Audio | ? | ? | ? | ? | Not started |
| Steam | ? | ? | ? | ? | Pre-alpha |

## By macOS version

Especially relevant for Metal, Rosetta, GameController, Vulkan-on-Metal and
driver bugs.

| Subsystem | macOS 14 | macOS 15 | macOS 26 | macOS 27 |
|---|:-:|:-:|:-:|:-:|
| Rosetta 2 + Wine | ? | ? | ? | ? |
| DXMT | ? | ? | ? | ? |
| MoltenVK | ? | ? | ? | ? |
| KosmicKrisp (Metal 4) | n/a | n/a | ? | ? |
| GameController | ? | ? | ? | ? |
