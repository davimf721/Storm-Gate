# Contributing to Storm Gate

Thanks for helping! Storm Gate is built by Wine developers, Metal developers,
Rust developers, game testers and macOS users working together. You do not
need to understand the whole project to help.

## Ways to contribute

| You know... | You can... |
|---|---|
| Nothing technical | Test games and file a **game report** with `stormgate logs bundle <appid>` |
| TOML | Contribute a **profile** in `compat/steam/<appid>/` |
| Rust | Work on the CLI and crates in `crates/` |
| C / C++ | Fix Wine, DXMT, DXVK, VKD3D-Proton or MoltenVK (prefer upstream) |
| Objective-C / Swift | macOS integrations, GameController, future GUI |

## Development setup

```bash
cd stormgate
make test          # works on Linux and macOS
make lint
make bootstrap     # macOS: check the runtime toolchain
```

The Rust workspace builds and tests on any platform. Wine and the graphics
backends only build on macOS. Use `--dry-run` and `STORMGATE_HOME=$(mktemp -d)`
to exercise the CLI without touching your real data.

## Pull requests

- One logical change per PR, linked to an issue with acceptance criteria.
- Add or update tests. Run `make lint test` before pushing.
- Changes that need a Mac to verify must list the manual steps you ran
  (chip, macOS version, runtime version) in the PR description.
- Architecture changes need an ADR in `docs/adr/`.
- Do not modify the Proton tree outside `stormgate/`.

## Runtime patches

Patches to third-party components go in `patches/<component>/` with the
metadata header from [`patches/README.md`](patches/README.md): ID, problem,
justification, test, upstream issue and status. Send the fix upstream too.

## Profiles and the compatibility database

```text
compat/steam/<appid>/
├── profile.toml   # validated by CI (`stormgate profile check-db compat`)
├── status.json    # rating + per-area results + hardware it was tested on
└── notes.md
```

- Ratings: PLATINUM, GOLD, SILVER, BRONZE, BROKEN, BLOCKED.
- A playable rating requires a `tested_on` entry from real hardware.
- Shared profiles must use only the open stack (no `d3dmetal`).
- Never include credentials, user names or personal paths.

## Reports

- **Game report:** game, Steam AppID, Mac, macOS, runtime, backend, result,
  logs, steps to reproduce.
- **Regression:** working version, broken version, expected, actual, logs.

Use the issue templates; attach `stormgate logs bundle` output (it redacts
tokens, cookies, user names and home paths, but review it before sharing).

## What we will not accept

- Anything that bypasses DRM, anti-cheat, signatures or other protections.
- Proprietary components in the core runtime, or redistribution of D3DMetal.
- Marking games/subsystems as supported without a real test.

## License

By contributing you agree that your contributions are dual-licensed under
Apache-2.0 OR MIT (patches to third-party code use that component's
license). See [LICENSES.md](LICENSES.md). Please follow the
[Code of Conduct](CODE_OF_CONDUCT.md).
