# Continuous integration

```text
Pull request → formatting → static analysis → unit tests → build components
            → synthetic Windows tests → GPU regression runner → artifacts
```

## GitHub-hosted (`.github/workflows/stormgate.yml`)

- `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test` (Linux + macOS arm64)
- `shellcheck`, patch queue check, profile and compatibility DB validation
- Synthetic Windows programs built with mingw-w64 and `hello` run against
  distro Wine on Linux (launcher smoke test)

## Self-hosted Mac (planned, issue #27)

Needed for Metal, Rosetta, the Wine runtime, DXMT, VKD3D, real GPUs and
games. The job is gated on the `STORMGATE_SELF_HOSTED` repository variable
and only runs for pushes to the main branch and trusted PRs.

Never put Steam (or any personal) credentials on a runner.
