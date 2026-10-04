#!/usr/bin/env bash
# Builds MoltenVK (Vulkan -> Metal), pinned in sources.lock.
#
# Wine runs as x86_64 under Rosetta, so the dylib must contain an x86_64
# slice; the build is checked with lipo.
#
# Output: dist/moltenvk/lib/libMoltenVK.dylib, dist/moltenvk/icd.d/MoltenVK_icd.json
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
sg_require_macos
sg_require_cmd xcodebuild cmake python3

OUT="${SG_DIST}/moltenvk"
src="$(sg_fetch_source moltenvk)"
sg_apply_patches moltenvk "$src"

sg_log "fetching MoltenVK dependencies (pinned by MoltenVK's ExternalRevisions)"
(cd "$src" && ./fetchDependencies --macos)
sg_log "building MoltenVK"
make -C "$src" macos

pkg="$src/Package/Release/MoltenVK"
dylib="$(find "$pkg" -name libMoltenVK.dylib -path '*macOS*' | head -1)"
icd="$(find "$pkg" -name MoltenVK_icd.json -path '*macOS*' | head -1)"
[[ -n "$dylib" && -n "$icd" ]] || sg_die "MoltenVK package layout not recognised under $pkg"
lipo -archs "$dylib" | grep -qw x86_64 || sg_die "libMoltenVK.dylib has no x86_64 slice"

rm -rf "$OUT"
mkdir -p "$OUT/lib" "$OUT/icd.d"
cp "$dylib" "$OUT/lib/"
# The ICD points at the library relative to itself.
sed 's#"library_path"[^,]*#"library_path": "../lib/libMoltenVK.dylib"#' "$icd" >"$OUT/icd.d/MoltenVK_icd.json"
sg_log "MoltenVK installed to $OUT"
