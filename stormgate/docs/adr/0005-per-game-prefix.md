# ADR 0005: One prefix per game by default

## Context
Shared prefixes let one game's DLL overrides, registry tweaks or
redistributables break another.

## Decision
`stormgate run` creates a prefix per game folder (`prefixes/<slug>`), each
with its own `stormgate.toml` metadata. Steam games run in the shared `steam`
prefix because the Steam client must be in the same prefix as the game;
they still get per-game graphics, sync, display and environment settings
from their profile, applied at launch.

## Consequences
- More disk use (~700 MB per fresh prefix); `prefix clone` and later a
  prefix cache mitigate it.
- Storm Gate only deletes prefixes that contain `stormgate.toml`.
