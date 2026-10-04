# Patch policy

Goal: **the smallest possible fork.**

```text
bug → minimal reproduction → local patch → test → upstream issue → upstream PR
    → drop the local patch once upstream ships it
```

## Layout

```text
patches/<component>/
├── series          # application order, one file per line
└── NNNN-short-name.patch
```

`<component>` matches a `sources.lock` entry (`wine`, `dxmt`, `dxvk-macos`,
`dxvk`, `vkd3d-proton`, `moltenvk`). The build scripts start from a clean
checkout of the pinned commit and apply `series` with `git apply`; a patch
that does not apply fails the build.

## Required header

Every patch starts with (before the diff, `git apply` ignores it):

```text
Storm-Gate-Patch: SG-WINE-0001
Problem: wineserver fails to start on Darwin because ...
Justification: Linux-only code path in Proton's branch; no macOS equivalent.
Test: stormgate test hello (fails without the patch)
Upstream-Issue: https://bugs.winehq.org/show_bug.cgi?id=NNNNN
Upstream-Status: submitted
```

`Upstream-Status` is one of `not-submitted`, `submitted`, `accepted`,
`rejected`, `not-applicable`. `scripts/check-patches.sh` (run by `make test`
and CI) enforces the header and that every patch is listed in `series`.

## Rebasing

When a pin moves (`scripts/update-sources.sh`), rebuild; patches that no
longer apply are rebased or dropped if upstream absorbed them. See also the
Proton tree's [REBASING_TIPS](../../../docs/REBASING_TIPS.md) for the Wine
cherry-pick conventions Valve uses.

## Before writing a patch

Check whether it is already solved in: Wine, Proton, DXMT, DXVK,
VKD3D-Proton, MoltenVK, Mesa, Sikarugir, wine-msync. Reuse and contribute
instead of duplicating.
