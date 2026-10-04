# Game profiles

A profile describes how one game is launched. Template:
[`profiles/template.toml`](../profiles/template.toml).

```toml
schema = 1

[game]
appid = 1091500
name = "Example Game"
executable = "bin/x64/Game.exe"   # relative to the install dir (optional)
arguments = ["-windowed"]          # optional

[runtime]
wine = "proton-wine-11"
version = "0.4.1"                  # optional pin (side-by-side runtimes)

[graphics]
d3d11 = "dxmt"                     # dxmt | dxvk | wined3d
d3d12 = "vkd3d-proton"             # experimental
d3d9 = "dxvk"                      # dxvk | wined3d
vulkan = "moltenvk"                # moltenvk | kosmickrisp

[sync]
backend = "msync"                  # server (default) | msync

[display]
virtual_desktop = false
width = 1920                       # only with virtual_desktop
height = 1080
retina = false

[environment]
SOME_VARIABLE = "1"

[dll_overrides]
dxgi = "native,builtin"
```

## Validation rules

- Unknown keys are errors (typos never silently do nothing).
- A backend must be able to implement the API it is assigned to.
- Shared profiles (`--shared`, the compatibility DB) may only use the open
  stack — `d3dmetal` is rejected.
- `environment` cannot set runtime-managed variables (`WINEPREFIX`,
  `WINEDLLOVERRIDES`, `WINEDEBUG`, `WINEMSYNC`, `DYLD_*`, `VK_ICD_FILENAMES`...)
  or anything that looks like a credential (`*TOKEN*`, `*PASSWORD*`, ...).
- `game.executable` must stay inside the install directory.

```bash
stormgate profile validate --shared compat/steam/*/profile.toml
stormgate profile check-db compat
```

## Where profiles come from

1. `~/Library/Application Support/StormGate/games/<appid>.toml` — written only
   after you confirm (`stormgate game add`, `stormgate profile detect --save`).
2. `compat/steam/<appid>/profile.toml` — the shared database.
3. Automatic detection from the executable's imports.

Storm Gate never changes a saved profile silently.
