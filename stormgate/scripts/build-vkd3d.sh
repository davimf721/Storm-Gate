#!/usr/bin/env bash
# Builds VKD3D-Proton (D3D12 -> Vulkan). EXPERIMENTAL on macOS.
#
# macOS specific changes come from the VKD3D-Proton-MacOS research project
# and are kept as reviewed patches in patches/vkd3d-proton (never vendored
# wholesale). Start with the D3D12 feature probe, then the triangle test.
#
# Output: dist/vkd3d-proton/{x64,x86}/{d3d12,d3d12core}.dll
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
sg_require_cmd meson ninja x86_64-w64-mingw32-gcc i686-w64-mingw32-gcc glslangValidator

OUT="${SG_DIST}/vkd3d-proton"
src="$(sg_fetch_source vkd3d-proton)"
sg_apply_patches vkd3d-proton "$src"
rm -rf "$OUT"

for arch in x64:64 x86:32; do
	name="${arch%%:*}"
	bits="${arch##*:}"
	build="${SG_BUILD}/vkd3d-build.$name"
	rm -rf "$build"
	sg_log "building VKD3D-Proton $name"
	meson setup "$build" "$src" \
		--cross-file "$src/build-win$bits.txt" \
		--buildtype release \
		--prefix "$build/install" \
		--bindir "" --libdir "" \
		-Denable_tests=false -Denable_extras=false
	ninja -C "$build" install
	mkdir -p "$OUT/$name"
	cp "$build"/install/d3d12*.dll "$OUT/$name/"
done
sg_log "VKD3D-Proton installed to $OUT (experimental)"
