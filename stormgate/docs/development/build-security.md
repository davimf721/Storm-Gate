# Build chain security

- Sources: fetched by **full commit hash** from `manifests/sources.lock` over
  HTTPS; the checkout is verified against the pin.
- Downloads: HTTPS only (also for redirects), fixed version, **SHA-256
  verified before use**, origin documented (`sg_download` in
  `scripts/lib/common.sh`).
- Never `curl ... | bash` in scripts. Installers are downloaded, reviewed,
  then run.
- The one unpinnable download is Valve's `SteamSetup.exe` (updated in place);
  its hash is logged on every install and can be enforced with
  `stormgate steam install --sha256`.
- Runtime archives are verified before extraction (`runtime install --sha256`),
  extracted into a staging directory and moved into place only after their
  manifest validates.
- Releases publish `SHA256SUMS`, `sources.lock`, `SBOM.spdx.json` and
  license texts.
- CI has no credentials. Self-hosted Mac runners never run untrusted PRs and
  never hold Steam accounts.
