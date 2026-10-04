# Release agent

**Owns:** `manifests/`, `scripts/package.sh`, `scripts/update-sources.sh`,
`LICENSES.md`, SBOM, release notes, versioning.

Rules:
- Every source pinned by full commit; `runtime.toml` and `sources.lock` agree
  (checked by `cargo test`).
- Downloads: HTTPS, fixed version, SHA-256 verified, documented origin.
  Never `curl ... | bash`.
- Releases ship `SHA256SUMS`, `sources.lock`, `SBOM.spdx.json` and license
  texts; LGPL components link to their exact sources and patches.
- Runtimes install side by side; never update while a game is running;
  rollback is `stormgate runtime use <version>`.
- First public release is `0.1-alpha`, explicitly experimental.
