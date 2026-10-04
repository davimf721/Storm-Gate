<div align="center">

# ⚡ Storm Gate

**Run Windows games on your Mac — free, open source, no subscription.**

An open-source Windows game compatibility runtime for macOS (Apple Silicon),
built using Wine and technologies from the Proton ecosystem.

[Português](#-português) · [Documentation](stormgate/docs) · [Roadmap](stormgate/docs/roadmap.md) · [Contributing](stormgate/CONTRIBUTING.md)

![status](https://img.shields.io/badge/status-pre--alpha-orange)
![platform](https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-black)
![license](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue)

</div>

---

> **Status: pre-alpha (milestone BOOT).** The CLI, orchestration core, build
> scripts and test programs exist and pass CI; the runtime has **not** yet
> been validated on real Apple Silicon hardware and no game is marked as
> supported. Expect breakage.

## What it is

Storm Gate wants to be what "Proton for Mac" should mean: one command to run
a Windows game, without knowing about Wine prefixes, DLL overrides, Vulkan
drivers or environment variables.

```bash
stormgate doctor                    # is this Mac ready?
stormgate steam install             # Windows Steam in its own prefix
stormgate steam start               # log in inside the Steam window
stormgate game run 1091500          # launch a Steam game by AppID
stormgate run ~/Games/MyGame/game.exe
```

For every launch Storm Gate inspects the executable, picks the graphics
backend, creates an isolated prefix, deploys the right DLLs, controls the
whole Wine environment and records logs you can attach to a bug report.

## How it works

```text
Windows game (x86_64)
      │
Wine (Proton branch) ── Rosetta 2 ── Apple Silicon
      │
      ├── D3D10/11 ── DXMT ─────────────────────────────┐
      ├── D3D9 ────── DXVK ──────────── Vulkan ──┐      │
      └── D3D12 ───── VKD3D-Proton ──── Vulkan ──┤      │
                          MoltenVK / KosmicKrisp ┘      │
                                             └──────── Metal
```

| API | Backend | Status |
|---|---|---|
| D3D10 / D3D11 | DXMT → Metal | next milestone (METAL) |
| D3D9 | DXVK → MoltenVK → Metal | planned |
| D3D12 | VKD3D-Proton → MoltenVK → Metal | experimental |
| Fallback | WineD3D | available |

## Principles

- **Free and open** — nothing essential depends on CrossOver, a subscription,
  a cloud service or a proprietary backend. Apple's D3DMetal can only ever be
  an optional backend you install yourself.
- **Reproducible** — every component pinned by commit; releases ship
  checksums, `sources.lock` and an SBOM.
- **Small fork** — reuse Wine, DXMT, DXVK, VKD3D-Proton and MoltenVK, send
  fixes upstream.
- **Legitimate** — Storm Gate never bypasses DRM or anti-cheat.

## Getting started (developers)

You need an Apple Silicon Mac (M1 or newer), macOS 14+, Rosetta 2, the Xcode
Command Line Tools and Homebrew. An M1 MacBook Air with 8 GB is enough;
keep ~40 GB free or point `STORMGATE_BUILD_DIR` at an external SSD.

```bash
git clone https://github.com/davimf721/Storm-Gate
cd Storm-Gate/stormgate
make bootstrap         # checks the toolchain and prints what is missing
make runtime           # builds Wine, DXMT, DXVK and MoltenVK, then packages them
make install-runtime
make test
```

No runtime yet? Try the launcher with any existing Wine:

```bash
STORMGATE_WINE=/opt/homebrew/bin/wine cargo run -p stormgate-cli -- run notepad.exe
```

More: [getting started](stormgate/docs/getting-started.md) ·
[building](stormgate/docs/building.md) ·
[architecture](stormgate/docs/architecture.md) ·
[troubleshooting](stormgate/docs/troubleshooting.md)

## Roadmap

```text
BOOT → METAL → STEAM → FIRST GAME → D3D9 → D3D12 → COMPATIBILITY DB → GUI → PUBLIC ALPHA
 ▲ we are here
```

See the [roadmap](stormgate/docs/roadmap.md) for the 20 starting issues and
their acceptance criteria.

## Contributing

You don't need to know C or Rust to help: testing games and sending
[game reports](.github/ISSUE_TEMPLATE/stormgate-game-report.yml) or
[profiles](stormgate/docs/profiles.md) is just as valuable. Read
[CONTRIBUTING.md](stormgate/CONTRIBUTING.md) to get started.

## Repository layout

| Path | Contents |
|---|---|
| [`stormgate/`](stormgate/) | Everything Storm Gate: Rust CLI and crates, build scripts, manifests, patches, profiles, tests, docs |
| everything else | Valve's [Proton](https://github.com/ValveSoftware/Proton) tree, kept unchanged so it can be rebased ([original README](README.proton.md)) |

---

## 🇧🇷 Português

**Rode jogos de Windows no seu Mac — gratuito, open source e sem assinatura.**

O Storm Gate é um runtime de compatibilidade para jogos Windows no macOS
(Apple Silicon), construído com Wine e tecnologias do ecossistema Proton.
Um comando roda o jogo; o Storm Gate cuida de prefixos, DLLs, backend
gráfico (DXMT, DXVK, VKD3D-Proton sobre Metal) e logs.

> **Estado: pré-alfa.** A CLI, o núcleo e os scripts existem e passam no CI,
> mas o runtime ainda não foi validado em um Mac real e nenhum jogo está
> marcado como suportado.

```bash
cd stormgate
make bootstrap && make runtime && make install-runtime
stormgate doctor
stormgate run ~/Games/MeuJogo/game.exe
```

Leia o [README em português](stormgate/README.pt-BR.md) e o
[guia de contribuição](stormgate/CONTRIBUTING.md). Issues e discussões em
português são bem-vindas.

---

## License

Storm Gate's own code is dual-licensed under [Apache-2.0](stormgate/LICENSE-APACHE)
or [MIT](stormgate/LICENSE-MIT). Third-party components keep their own
licenses — see [LICENSES.md](stormgate/LICENSES.md). The Proton tree is
governed by [LICENSE](LICENSE) and [LICENSE.proton](LICENSE.proton).

Storm Gate is not affiliated with or endorsed by Valve, Apple or
CodeWeavers. "Proton" and "Steam" are trademarks of Valve Corporation.
