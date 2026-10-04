# ADR 0007: Storm Gate lives in `stormgate/` inside the Proton fork

## Context
The repository is a fork of ValveSoftware/Proton. Porting the Proton
monorepo (Makefile.in, Steam Runtime containers, Linux toolchains) to Darwin
would be enormous work that does not increase game compatibility.

## Decision
Storm Gate's runtime, CLI, scripts and docs live in `stormgate/`. The Proton
tree is left untouched so it can be rebased onto Valve's releases; it serves
as the reference for Wine/DXVK/VKD3D-Proton pins and Proton's launcher
behaviour. Components are built from pinned upstream sources listed in
`stormgate/manifests/`. Larger parts of Proton's launcher may be reused once
the macOS runtime works.

The only additions outside `stormgate/` are a pointer at the top of the
root `README.md` and Storm Gate's CI workflow, issue templates and PR
template under `.github/`.

## Consequences
- Rebasing on Proton never conflicts with Storm Gate code (at most the
  README banner).
- CI for Storm Gate is path-filtered to `stormgate/**`.
