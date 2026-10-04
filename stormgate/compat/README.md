# Compatibility database

```text
compat/steam/<appid>/
├── profile.toml   game profile (open stack only), game.appid must match
├── status.json    rating + results, see below
└── notes.md       free-form notes, known issues, workarounds
```

`status.json`:

```json
{
  "schema": 1,
  "rating": "GOLD",
  "areas": {
    "boot": "pass", "menu": "pass", "gameplay": "pass", "audio": "partial",
    "input": "pass", "graphics": "pass", "performance": "partial",
    "online": "untested", "anti_cheat": "untested"
  },
  "tested_on": [
    { "chip": "M1", "macos": "15.6", "runtime": "0.1.0", "date": "2026-10-04" }
  ],
  "known_issues": ["Intro video is black"]
}
```

- Ratings: `PLATINUM`, `GOLD`, `SILVER`, `BRONZE`, `BROKEN`, `BLOCKED`.
- Area results: `pass`, `partial`, `fail`, `untested`.
- A playable rating needs `boot = "pass"` and at least one `tested_on`.
- `BLOCKED` is for games that cannot work by design (e.g. kernel anti-cheat).

CI runs `stormgate profile check-db compat`. The database is empty until
the first game is tested on real hardware.
