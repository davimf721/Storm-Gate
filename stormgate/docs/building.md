# Building the runtime

All builds are pinned: [`manifests/sources.lock`](../manifests/sources.lock)
lists the exact commit of every component and the scripts refuse anything
else. Outputs go to `build/` (sources, objects) and `dist/` (installed
components and release archives). Set `STORMGATE_BUILD_DIR` /
`STORMGATE_DIST_DIR` to move them to an external SSD.

> **Status:** the scripts encode the intended build; they have not yet been
> run end-to-end on Apple Silicon. Phase 1 (issue #3) is exactly that. Expect
> to add patches in `patches/wine` for Proton-Wine code that assumes Linux.

## Prerequisites

```bash
make bootstrap            # report
scripts/bootstrap.sh --install   # also brew-install missing formulae
```

- Xcode Command Line Tools, Rosetta 2
- arm64 Homebrew: `git cmake meson ninja pkgconf mingw-w64 glslang bison flex rustup`
- **x86_64 Homebrew** in `/usr/local` for the libraries Wine links against
  (Wine is an x86_64 binary that runs under Rosetta):

### x86_64 Homebrew

Download and **review** Homebrew's installer before running it under Rosetta:

```bash
curl -fsSLo /tmp/brew-install.sh https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh
less /tmp/brew-install.sh
arch -x86_64 /bin/bash /tmp/brew-install.sh
arch -x86_64 /usr/local/bin/brew install freetype gnutls sdl2
```

## Steps

```bash
make wine        # scripts/build-wine.sh     -> dist/wine
make dxmt        # scripts/build-dxmt.sh     -> dist/dxmt (+ winemetal.so into dist/wine)
make dxvk        # scripts/build-dxvk.sh     -> dist/dxvk (DXVK-macOS flavour by default)
make moltenvk    # scripts/build-moltenvk.sh -> dist/moltenvk
make vkd3d       # scripts/build-vkd3d.sh    -> dist/vkd3d-proton (experimental)
make package     # scripts/package.sh        -> dist/stormgate-runtime-<v>-macos-arm64.tar.gz
make install-runtime
```

`make runtime` runs the default set (no VKD3D-Proton);
`make runtime-experimental` includes it.

### Wine options

| Variable | Effect |
|---|---|
| `WITH_MSYNC=1` | Apply wine-msync (`msync-devel.patch`), mark runtime with the `msync` feature |
| `MSYNC_PATCH=...` | Use another patch from wine-msync |
| `WINE_CONFIGURE_EXTRA="..."` | Extra `./configure` flags |
| `WINE_COMPONENT=...` | Build another `sources.lock` entry (e.g. an upstream Wine pin for comparison) |
| `X86_BREW=/usr/local` | Prefix of the x86_64 Homebrew |

### DXVK flavour

Upstream DXVK 2.x needs Vulkan features MoltenVK does not expose yet, so the
default is `DXVK_FLAVOR=macos` (Gcenx/DXVK-macOS). `DXVK_FLAVOR=upstream`
builds Proton's pin for experiments with KosmicKrisp.

## Updating a pin

```bash
scripts/update-sources.sh dxmt --ref v0.81
cargo test -p stormgate-runtime    # runtime.toml and sources.lock must agree
```

Rebuild, run the synthetic tests and the regression set before committing.

## Smoke tests

```bash
make windows-tests                 # needs mingw-w64
stormgate test hello
stormgate test win32-window
stormgate test d3d11-triangle      # renders through DXMT and checks pixels
```

## Without a runtime

`STORMGATE_WINE=/path/to/wine` makes the CLI use an existing Wine (Homebrew,
CrossOver-free builds, distro packages on Linux). Graphics backends are
disabled in that mode (WineD3D only). This is how CI-style smoke tests of the
launcher run on Linux.
