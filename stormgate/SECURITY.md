# Security Policy

## Reporting a vulnerability

Please **do not** open a public issue. Use GitHub's
[private vulnerability reporting](https://github.com/davimf721/Storm-Gate/security/advisories/new)
with steps to reproduce and the affected version. You should get a response
within 7 days.

## Scope

In scope:
- The `stormgate` CLI and crates (e.g. path traversal through prefix names,
  profiles or Steam manifests; unsafe archive extraction; log redaction gaps).
- Build and release scripts (pinning, checksum verification, supply chain).
- Runtime packaging (permissions, library search paths).

Out of scope:
- Vulnerabilities in Wine, DXMT, DXVK, VKD3D-Proton or MoltenVK themselves —
  report them upstream (we will help coordinate).
- Windows programs being able to access your files: Wine is a compatibility
  layer, **not a sandbox**. Only run software you trust.

## Supply chain rules

- Sources are fetched by pinned commit over HTTPS; downloads are verified
  with SHA-256. Release scripts never pipe downloads into a shell.
- Runtime archives are verified before installation
  (`stormgate runtime install --sha256`).
- CI never has access to Steam or other personal credentials; self-hosted
  Mac runners must not be used for untrusted pull requests.
