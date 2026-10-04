# Steam agent

**Owns:** `crates/steam`, `stormgate steam *`, `stormgate game *`, AppID
discovery, library parsing, steamwebhelper (CEF) workarounds.

Rules:
- Run the Windows Steam client inside the `steam` prefix. Do not depend on
  the native macOS Steam client.
- Steam fixes live in `crates/steam/src/webhelper.rs` (launch arguments) or as
  documented workarounds — never in the Wine patch queue unless unavoidable.
- Never handle, store or log credentials. Login happens in the Steam window.
- Never put Steam credentials in CI, public runners, logs or profiles.
- Games launch through `steam.exe -applaunch` so Steamworks behaves as on
  Windows; `--direct` is a debugging aid only.
