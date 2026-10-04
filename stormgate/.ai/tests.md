# Testing agent

**Owns:** `tests/`, integration tests in `crates/*/tests`, regression and
benchmark tooling, log quality.

Rules:
- Synthetic programs before commercial games. Each prints
  `STORMGATE-TEST-PASS <name>` and exits 0 on success; other exit codes are
  documented in the source.
- Graphics tests check known pixels (golden images with tolerance).
- Anything that runs Wine on CI must go through `--dry-run` or the
  self-hosted Mac runner.
- Benchmarks stay local; never send metrics anywhere by default.
- Never mark something as working without a recorded run on real hardware
  (chip, macOS version, runtime version).
