# Agents guide

Instructions for AI coding agents (and humans) working on Storm Gate. The
rules in [`CLAUDE.md`](CLAUDE.md) always apply.

## Work in small issues

Never take tasks like "build Proton for macOS". Every change maps to an issue
with acceptance criteria, for example:

```text
Implement isolated Wine prefix creation.

Acceptance criteria:
- command: stormgate prefix create NAME
- creates directory
- runs wineboot
- writes stormgate.toml
- validates wineserver exit
- integration test
- no global state
```

## Standard implementation prompt

```text
You are working on the Storm Gate runtime.

Task:
Implement <TASK>.

Before changing code:
1. inspect relevant architecture docs (docs/architecture.md, docs/adr/);
2. identify the smallest subsystem (crate or script) that needs changes;
3. inspect upstream behavior (Wine, DXMT, DXVK, VKD3D-Proton, MoltenVK);
4. propose the minimal change.

Constraints:
- Apple Silicon is Tier 1.
- Do not introduce proprietary runtime dependencies.
- Preserve third-party licenses.
- Do not modify unrelated modules.
- Do not bypass anti-cheat or DRM.
- Add or update tests.
- Run `make lint test`.
- Report remaining risks.

Acceptance criteria:
<CRITERIA>
```

## Engineering loop

```text
issue -> analysis -> small patch -> build -> automated test
      -> test on a real Mac -> logs -> analyse regression -> next patch
```

Never generate thousands of lines and merge them directly. A change that
cannot be tested on Linux CI must say so and list the manual steps for a Mac.

## Specialised agents

| File | Owns |
|---|---|
| [`.ai/wine.md`](.ai/wine.md) | Wine, winemac.drv, prefixes, processes, filesystem, MSync |
| [`.ai/graphics.md`](.ai/graphics.md) | DXMT, DXVK, VKD3D-Proton, MoltenVK, shaders, Metal |
| [`.ai/steam.md`](.ai/steam.md) | Steam bootstrap, steamwebhelper, AppIDs, libraries, launch |
| [`.ai/macos.md`](.ai/macos.md) | Cocoa, Rosetta, GameController, signing, notarization |
| [`.ai/tests.md`](.ai/tests.md) | Reproductions, integration and regression tests, benchmarks, logs |
| [`.ai/release.md`](.ai/release.md) | Packaging, pins, SBOM, licenses, releases |

An agent stays inside its area; cross-area changes need a note in the PR.
