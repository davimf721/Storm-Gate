#!/usr/bin/env bash
# Assembles dist/* into a side-by-side runtime archive.
#
# Output (in dist/):
#   stormgate-runtime-<version>-macos-arm64.tar.gz
#   SHA256SUMS
#   SBOM.spdx.json
#   sources.lock
#
# Missing optional components (VKD3D-Proton, MSync) are reported, not fatal.
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"

VERSION="$(sg_runtime_version)"
[[ -n "$VERSION" ]] || sg_die "cannot read runtime version"
STAGE="${SG_BUILD}/package/${VERSION}"
ARCHIVE="stormgate-runtime-${VERSION}-macos-arm64.tar.gz"

[[ -x "${SG_DIST}/wine/bin/wine" ]] || sg_die "dist/wine missing; run scripts/build-wine.sh"

rm -rf "${SG_BUILD}/package"
mkdir -p "$STAGE"
features=()
for comp in wine dxmt dxvk vkd3d-proton moltenvk; do
	if [[ -d "${SG_DIST}/${comp}" ]]; then
		cp -R "${SG_DIST}/${comp}" "$STAGE/"
	else
		sg_warn "$comp not built; runtime will not include it"
	fi
done
if [[ -f "${SG_DIST}/wine/.msync" ]]; then
	features+=("\"msync\"")
fi

# runtime.toml = manifest + features actually built in.
{
	sed -n '/^\[runtime\]/,/^$/p' "${SG_ROOT}/manifests/runtime.toml" | sed '/^$/d'
	printf 'features = [%s]\n\n' "$(
		IFS=,
		echo "${features[*]:-}"
	)"
	sed -n '/^\[sources\./,$p' "${SG_ROOT}/manifests/runtime.toml"
} >"$STAGE/runtime.toml"

cp -R "${SG_ROOT}/compat" "$STAGE/compat"
mkdir -p "$STAGE/licenses"
cp "${SG_ROOT}/LICENSES.md" "${SG_ROOT}/LICENSE-APACHE" "${SG_ROOT}/LICENSE-MIT" "$STAGE/licenses/"
for comp in wine dxmt dxvk-macos dxvk vkd3d-proton moltenvk; do
	src="${SG_SOURCES}/${comp}"
	for f in COPYING COPYING.LIB LICENSE LICENSE.md LICENSE.txt; do
		[[ -f "$src/$f" ]] && cp "$src/$f" "$STAGE/licenses/${comp}-${f}"
	done
done

sg_log "creating $ARCHIVE"
mkdir -p "$SG_DIST"
# Reproducible ordering and ownership.
tar -C "${SG_BUILD}/package" --uid 0 --gid 0 --uname root --gname wheel \
	-czf "${SG_DIST}/${ARCHIVE}" "$VERSION" 2>/dev/null ||
	tar -C "${SG_BUILD}/package" -czf "${SG_DIST}/${ARCHIVE}" "$VERSION"

cp "$SG_LOCK" "${SG_DIST}/sources.lock"
(cd "$SG_DIST" && for f in "$ARCHIVE" sources.lock; do printf '%s  %s\n' "$(sg_sha256 "$f")" "$f"; done) >"${SG_DIST}/SHA256SUMS"

# Minimal SPDX 2.3 SBOM generated from sources.lock.
{
	printf '{\n  "spdxVersion": "SPDX-2.3",\n  "dataLicense": "CC0-1.0",\n'
	printf '  "SPDXID": "SPDXRef-DOCUMENT",\n  "name": "stormgate-runtime-%s",\n' "$VERSION"
	printf '  "documentNamespace": "https://github.com/davimf721/Storm-Gate/spdx/%s",\n' "$VERSION"
	printf '  "creationInfo": { "created": "%s", "creators": ["Tool: stormgate-package.sh"] },\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf '  "packages": [\n'
	first=1
	while read -r name repo commit; do
		[[ -z "$name" || "$name" == \#* ]] && continue
		[[ $first -eq 1 ]] || printf ',\n'
		first=0
		printf '    { "SPDXID": "SPDXRef-%s", "name": "%s", "versionInfo": "%s", "downloadLocation": "git+%s@%s", "licenseConcluded": "%s", "licenseDeclared": "%s", "copyrightText": "NOASSERTION" }' \
			"$name" "$name" "$commit" "$repo" "$commit" "$(sg_license_of "$name")" "$(sg_license_of "$name")"
	done <"$SG_LOCK"
	printf '\n  ]\n}\n'
} >"${SG_DIST}/SBOM.spdx.json"

sg_log "done:"
cat "${SG_DIST}/SHA256SUMS"
echo "Install with: stormgate runtime install ${SG_DIST}/${ARCHIVE} --sha256 $(sg_sha256 "${SG_DIST}/${ARCHIVE}") --use"
