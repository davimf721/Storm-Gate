#!/usr/bin/env bash
# Builds DXVK (D3D9/10/11 -> Vulkan) for use over MoltenVK.
#
# DXVK_FLAVOR=macos (default) builds Gcenx/DXVK-macOS, which carries the
# workarounds MoltenVK needs; DXVK_FLAVOR=upstream builds the Proton pin
# (useful once KosmicKrisp or MoltenVK expose the required features).
#
# Output: dist/dxvk/{x64,x32}/*.dll
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
sg_require_cmd meson ninja x86_64-w64-mingw32-gcc i686-w64-mingw32-gcc glslangValidator

case "${DXVK_FLAVOR:-macos}" in
macos) component=dxvk-macos ;;
upstream) component=dxvk ;;
*) sg_die "DXVK_FLAVOR must be macos or upstream" ;;
esac
OUT="${SG_DIST}/dxvk"

src="$(sg_fetch_source "$component")"
sg_apply_patches "$component" "$src"
rm -rf "$OUT"

for bits in 64 32; do
	build="${SG_BUILD}/dxvk-build.$bits"
	rm -rf "$build"
	sg_log "building DXVK win$bits"
	meson setup "$build" "$src" \
		--cross-file "$src/build-win$bits.txt" \
		--buildtype release \
		--prefix "$build/install" \
		--bindir "" --libdir ""
	ninja -C "$build" install
	dest="$OUT/x$bits"
	mkdir -p "$dest"
	cp "$build"/install/*.dll "$dest/"
done
[[ -f "$OUT/x64/d3d9.dll" ]] || sg_die "d3d9.dll not produced"
sg_log "DXVK ($component) installed to $OUT"
