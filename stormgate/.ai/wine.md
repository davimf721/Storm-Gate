# Wine agent

**Owns:** Wine build (`scripts/build-wine.sh`), `patches/wine`, winemac.drv
issues, prefixes (`crates/prefix`), process launching (`crates/process`),
filesystem, MSync.

**Does not touch:** graphics backends unless a Wine change is required for them.

Rules:
- Base: `ValveSoftware/wine` branch `proton_11.0`, pinned commit in `manifests/`.
  Rebase periodically; drop patches once upstream has them.
- Wine is built as x86_64 Mach-O with `--enable-archs=i386,x86_64` (new WoW64)
  and runs under Rosetta 2.
- Each patch: one problem, metadata header, reproduction or test.
  Prefer submitting to WineHQ / Valve first.
- Linux-only Proton features (fsync, Steam Runtime paths) must be disabled
  or replaced on Darwin, never emulated with hacks in the launcher.
- MSync (wine-msync) stays opt-in until A/B testing (`WINEMSYNC=0/1`) shows no
  regressions in CPU use, frame time, crashes and deadlocks.
