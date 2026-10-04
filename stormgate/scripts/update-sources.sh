#!/usr/bin/env bash
# Pins a component to a new commit in both manifests.
#
#   scripts/update-sources.sh <name> <commit>
#   scripts/update-sources.sh <name> --ref <branch-or-tag>   (resolve via git ls-remote)
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"

name="${1:?usage: update-sources.sh <name> <commit>|--ref <ref>}"
read -r repo old < <(sg_lock_get "$name")
if [[ "${2:-}" == "--ref" ]]; then
	ref="${3:?missing ref}"
	commit="$(git ls-remote "$repo" "refs/tags/${ref}^{}" "refs/tags/${ref}" "refs/heads/${ref}" | awk 'NR == 1 { print $1 }')"
	[[ -n "$commit" ]] || sg_die "cannot resolve $ref in $repo"
else
	commit="${2:?missing commit}"
fi
[[ "$commit" =~ ^[0-9a-f]{40}$ ]] || sg_die "commit must be a full 40 character hash"

sed -i.bak "s/${old}/${commit}/" "$SG_LOCK" "${SG_ROOT}/manifests/runtime.toml"
rm -f "${SG_LOCK}.bak" "${SG_ROOT}/manifests/runtime.toml.bak"
sg_log "$name: ${old:0:12} -> ${commit:0:12}"
echo "Rebuild and run the regression tests before committing (docs/development/patches.md)."
