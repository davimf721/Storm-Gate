#!/usr/bin/env bash
# Builds Proton's Wine (ValveSoftware/wine, pinned in sources.lock) for macOS
# as an x86_64 Mach-O that runs under Rosetta 2, using the new WoW64 mode
# (32-bit Windows code runs inside the 64-bit Wine process).
#
# Output: dist/wine/  (bin/wine, bin/wineserver, lib/wine/...)
#
# Environment:
#   WINE_COMPONENT         sources.lock entry to build (default: wine)
#   WITH_MSYNC=1           apply the wine-msync patch set (experimental, Phase 4)
#   MSYNC_PATCH            patch file from wine-msync (default: msync-devel.patch)
#   WINE_CONFIGURE_EXTRA   extra ./configure flags
#   X86_BREW               x86_64 Homebrew prefix (default: /usr/local)
#
# Status: Phase 1 script. Proton's Wine branch is Linux-first; macOS build
# breakages are fixed with patches in patches/wine (see docs/development/patches.md).
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
sg_require_macos
sg_require_cmd git clang x86_64-w64-mingw32-gcc pkg-config

COMPONENT="${WINE_COMPONENT:-wine}"
X86_BREW="${X86_BREW:-/usr/local}"
ARM_BREW="$(brew --prefix 2>/dev/null || echo /opt/homebrew)"
OUT="${SG_DIST}/wine"
BUILD="${SG_BUILD}/wine-build"

src="$(sg_fetch_source "$COMPONENT")"
sg_apply_patches wine "$src"
if [[ "${WITH_MSYNC:-0}" == 1 ]]; then
	msync="$(sg_fetch_source wine-msync)"
	patch="${msync}/${MSYNC_PATCH:-msync-devel.patch}"
	[[ -f "$patch" ]] || sg_die "wine-msync has no $(basename "$patch")"
	sg_log "applying $(basename "$patch") from wine-msync"
	git -C "$src" apply --index "$patch" ||
		sg_die "msync does not apply to this Wine; it needs a rebase onto Proton Wine (docs/roadmap.md, issue #8)"
fi

export PATH="${ARM_BREW}/opt/bison/bin:${ARM_BREW}/opt/flex/bin:${PATH}"
export CC="clang -arch x86_64"
export CXX="clang++ -arch x86_64"
export PKG_CONFIG_PATH="${X86_BREW}/lib/pkgconfig:${X86_BREW}/opt/freetype/lib/pkgconfig:${X86_BREW}/opt/gnutls/lib/pkgconfig"
export CPPFLAGS="-I${X86_BREW}/include"
export LDFLAGS="-L${X86_BREW}/lib -Wl,-headerpad_max_install_names"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-14.0}"

rm -rf "$BUILD" "$OUT"
mkdir -p "$BUILD"
cd "$BUILD" || exit 1

sg_log "configuring Wine (x86_64, new WoW64)"
# configure runs x86_64 test programs, which needs Rosetta.
# shellcheck disable=SC2086
arch -x86_64 "$src/configure" \
	--prefix="$OUT" \
	--enable-archs=i386,x86_64 \
	--disable-tests \
	--disable-winedbg \
	--without-x \
	--without-alsa --without-oss --without-pulse --without-udev \
	--without-dbus --without-v4l2 --without-sane --without-cups \
	--without-krb5 --without-pcap --without-usb --without-capi \
	--without-gphoto --without-netapi \
	--with-freetype --with-gnutls --with-sdl --with-vulkan \
	--with-mingw \
	${WINE_CONFIGURE_EXTRA:-}

sg_log "building Wine with $(sg_jobs) jobs"
arch -x86_64 make -j"$(sg_jobs)"
arch -x86_64 make install-lib

for bin in wine wineserver; do
	[[ -x "$OUT/bin/$bin" ]] || sg_die "build did not produce bin/$bin"
done
file "$OUT/bin/wine" | grep -q x86_64 || sg_die "bin/wine is not x86_64"
# Marker read by package.sh to advertise the msync feature.
if [[ "${WITH_MSYNC:-0}" == 1 ]]; then
	touch "$OUT/.msync"
fi
sg_log "Wine installed to $OUT"
"$OUT/bin/wine" --version
