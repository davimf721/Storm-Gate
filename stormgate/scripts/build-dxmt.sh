#!/usr/bin/env bash
# Builds DXMT (Direct3D 10/11 -> Metal), pinned in sources.lock.
#
# Output: dist/dxmt/{x86_64-windows,i386-windows}/*.dll and the unix side
# (winemetal.so) installed into dist/wine/lib/wine/x86_64-unix.
#
# Requires dist/wine from build-wine.sh (DXMT links its unix side against
# the Wine build). Follow upstream's README for toolchain requirements;
# DXMT_MESON_ARGS passes extra options through.
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
sg_require_macos
sg_require_cmd meson ninja x86_64-w64-mingw32-gcc

WINE="${SG_DIST}/wine"
[[ -x "$WINE/bin/wine" ]] || sg_die "build Wine first (scripts/build-wine.sh)"
OUT="${SG_DIST}/dxmt"

src="$(sg_fetch_source dxmt)"
sg_apply_patches dxmt "$src"

rm -rf "$OUT" "${SG_BUILD}/dxmt-build"
sg_log "configuring DXMT"
# shellcheck disable=SC2086
meson setup "${SG_BUILD}/dxmt-build" "$src" \
	--cross-file "$src/build-win64.txt" \
	--native-file "$src/build-osx.txt" \
	--buildtype release \
	-Dwine_install_path="$WINE" \
	${DXMT_MESON_ARGS:-}
ninja -C "${SG_BUILD}/dxmt-build"

mkdir -p "$OUT/x86_64-windows"
find "${SG_BUILD}/dxmt-build" -name '*.dll' -exec cp {} "$OUT/x86_64-windows/" \;
unix_so="$(find "${SG_BUILD}/dxmt-build" -name 'winemetal.so' | head -1)"
[[ -n "$unix_so" ]] || sg_die "winemetal.so not produced"
cp "$unix_so" "$WINE/lib/wine/x86_64-unix/"
for dll in d3d11 dxgi d3d10core winemetal; do
	[[ -f "$OUT/x86_64-windows/$dll.dll" ]] || sg_die "missing $dll.dll"
done
sg_log "DXMT installed to $OUT"
