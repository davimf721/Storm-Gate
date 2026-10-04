# Game testing procedure

1. `stormgate doctor` must be READY. Note the chip, macOS and runtime version.
2. Install the game through `stormgate steam start` (or use the standalone exe).
3. `stormgate game add <appid>` — review the suggested profile.
4. `stormgate game run <appid>`; play at least 15 minutes.
5. Record each area: boot, menu, gameplay, audio, input, graphics,
   performance, online, anti-cheat (`pass` / `partial` / `fail` / `untested`).
6. `stormgate logs bundle <appid>` and attach it to the game report issue.
7. Open a PR adding `compat/steam/<appid>/{profile.toml,status.json,notes.md}`.

Start with simple games: a light D3D11 title, then D3D9, then D3D12. Never
mark a game as working without completing this procedure on real hardware.
