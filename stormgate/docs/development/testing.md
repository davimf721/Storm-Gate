# Testing

| Layer | Where | Runs on |
|---|---|---|
| Unit tests | `crates/*/src` (`#[cfg(test)]`) | any OS, CI |
| CLI integration | `crates/cli/tests/cli.rs` (fake runtime + `--dry-run`) | any OS, CI |
| Script lint | `shellcheck`, `scripts/check-patches.sh` | CI |
| Profiles / DB | `stormgate profile validate`, `profile check-db` | CI |
| Synthetic Windows programs | `tests/windows/` + `stormgate test <name>` | Wine (Linux smoke) / Mac runner |
| Golden images | pixel checks inside the synthetic programs | Mac runner |
| Games | `tests/games/` procedure + compat DB | real Macs |

```bash
make test             # everything that does not need a Mac
make windows-tests    # build the .exe files (mingw-w64)
stormgate test hello
```

Every synthetic program prints `STORMGATE-TEST-PASS <name>` and exits 0 on
success; other exit codes are documented at the top of its source.

### Linux smoke test of the launcher

The launch pipeline can be exercised against distro Wine:

```bash
STORMGATE_HOME=$(mktemp -d) STORMGATE_WINE=/usr/lib/wine/wine64 \
  cargo run -p stormgate-cli -- test hello
```

This validates prefix creation, environment control, logging and exit-code
handling — not macOS behaviour.

## Order of real-world tests

1. hello.exe 2. notepad.exe 3. D3D9 sample 4. D3D11 sample 5. Vulkan sample
6. D3D12 sample 7. Steam 8. light D3D11 game 9. D3D9 game 10. D3D12 game
