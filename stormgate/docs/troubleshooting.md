# Troubleshooting

Start with `stormgate doctor`, then `stormgate logs last [appid]`.

| Symptom | Try |
|---|---|
| `Rosetta 2 missing` | `/usr/sbin/softwareupdate --install-rosetta --agree-to-license` |
| `no runtime installed` | `make runtime && make install-runtime`, or `STORMGATE_WINE=...` for experiments |
| `runtime ... is missing .../d3d11.dll` | The runtime was built without that backend; rebuild or force `--backend wined3d` |
| Prefix broken after an update | `stormgate prefix repair <name>` (or `prefix clone` first to keep a copy) |
| Black window / no rendering | `stormgate run --debug-graphics ...`; compare `--backend dxmt` vs `--backend dxvk` vs `--backend wined3d` |
| Fullscreen/Retina issues | Profile `[display] virtual_desktop = true`, `width`/`height`, `retina = false` |
| Hangs or stutters with MSync | Set `[sync] backend = "server"` in the profile |
| Game started with the wrong settings | Make sure Steam was not already running; `stormgate steam stop`, then `game run` |
| Anti-cheat warning | Kernel-level anti-cheat cannot work under Wine; Storm Gate will not bypass it |

## Debug options

```bash
stormgate run --debug-graphics game.exe      # Wine +loaddll,+seh; DXVK/DXMT/VKD3D logs
stormgate run --vulkan-validation game.exe   # Khronos validation layer (slow)
stormgate run --metal-validation game.exe    # Metal API validation (slow)
stormgate --dry-run run game.exe             # print the exact command and environment
```

Logs live in `~/Library/Application Support/StormGate/logs/<date>/<label>-<time>/`.
`stormgate logs bundle` creates a zip with tokens, cookies, e-mail addresses,
SteamIDs, your user name and home directory redacted. Review it before
sharing.
