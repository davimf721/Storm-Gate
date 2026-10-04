#!/usr/bin/env bash
# Variables defined here are used by the scripts that source this file.
# shellcheck disable=SC2034
# Shared helpers for Storm Gate build scripts. Source, do not execute.
#
# Rules (docs/development/build-security.md):
#   - sources are fetched by pinned commit from manifests/sources.lock;
#   - downloads are HTTPS only and verified by SHA-256;
#   - nothing is ever piped into a shell.

set -euo pipefail


SG_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SG_LOCK="${SG_ROOT}/manifests/sources.lock"
SG_BUILD="${STORMGATE_BUILD_DIR:-${SG_ROOT}/build}"
SG_DIST="${STORMGATE_DIST_DIR:-${SG_ROOT}/dist}"
SG_SOURCES="${SG_BUILD}/sources"

sg_log() { printf '\033[1;34m==>\033[0m %s\n' "$*" >&2; }
sg_warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
sg_die() {
	printf '\033[1;31merror:\033[0m %s\n' "$*" >&2
	exit 1
}

sg_is_macos() { [[ "$(uname -s)" == "Darwin" ]]; }

sg_require_macos() {
	sg_is_macos || sg_die "this step must run on macOS (found $(uname -s))"
}

sg_require_cmd() {
	local cmd
	for cmd in "$@"; do
		command -v "$cmd" >/dev/null 2>&1 || sg_die "missing '$cmd'; run scripts/bootstrap.sh"
	done
}

sg_jobs() {
	if sg_is_macos; then
		sysctl -n hw.ncpu
	else
		nproc
	fi
}

# Prints "<repository> <commit>" for a component in sources.lock.
sg_lock_get() {
	local name="$1" line
	line="$(awk -v n="$name" '$1 == n { print $2, $3 }' "$SG_LOCK")"
	[[ -n "$line" ]] || sg_die "'$name' is not listed in $SG_LOCK"
	printf '%s\n' "$line"
}

# Clones a component at its pinned commit into $SG_SOURCES/<name> and prints
# the path. Re-uses an existing checkout when it is already at that commit.
sg_fetch_source() {
	local name="$1" repo commit dest
	read -r repo commit < <(sg_lock_get "$name")
	[[ "$repo" == https://* ]] || sg_die "$name: repository must use https"
	[[ "$commit" =~ ^[0-9a-f]{40}$ ]] || sg_die "$name: commit must be a full hash"
	dest="${SG_SOURCES}/${name}"
	if [[ -d "$dest/.git" ]] && [[ "$(git -C "$dest" rev-parse HEAD)" == "$commit" ]]; then
		sg_log "$name already at ${commit:0:12}"
	else
		sg_log "fetching $name ${commit:0:12} from $repo"
		rm -rf "$dest"
		mkdir -p "$dest"
		git -C "$dest" init -q
		git -C "$dest" remote add origin "$repo"
		git -C "$dest" fetch -q --depth 1 origin "$commit"
		git -C "$dest" -c advice.detachedHead=false checkout -q FETCH_HEAD
	fi
	[[ "$(git -C "$dest" rev-parse HEAD)" == "$commit" ]] || sg_die "$name: checkout does not match pinned commit"
	if [[ -f "$dest/.gitmodules" ]]; then
		git -C "$dest" submodule update -q --init --recursive --depth 1
	fi
	printf '%s\n' "$dest"
}

# Applies patches/<component>/series (one file name per line, '#' comments)
# on top of a clean checkout. Every patch must apply cleanly.
sg_apply_patches() {
	local component="$1" src="$2" dir series patch
	dir="${SG_ROOT}/patches/${component}"
	series="${dir}/series"
	[[ -f "$series" ]] || return 0
	git -C "$src" reset -q --hard
	git -C "$src" clean -q -fdx
	while IFS= read -r patch || [[ -n "$patch" ]]; do
		patch="${patch%%#*}"
		patch="${patch//[[:space:]]/}"
		[[ -z "$patch" ]] && continue
		[[ -f "${dir}/${patch}" ]] || sg_die "$component: series lists missing patch $patch"
		sg_log "$component: applying $patch"
		git -C "$src" apply --index --whitespace=nowarn "${dir}/${patch}" ||
			sg_die "$component: $patch does not apply; rebase it (docs/development/patches.md)"
	done <"$series"
}

sg_sha256() {
	if command -v shasum >/dev/null 2>&1; then
		shasum -a 256 "$1" | awk '{ print $1 }'
	else
		sha256sum "$1" | awk '{ print $1 }'
	fi
}

# Downloads a file over HTTPS and verifies its SHA-256 before keeping it.
sg_download() {
	local url="$1" sha="$2" dest="$3"
	[[ "$url" == https://* ]] || sg_die "refusing non-https download: $url"
	[[ "$sha" =~ ^[0-9a-f]{64}$ ]] || sg_die "a pinned sha256 is required for $url"
	if [[ -f "$dest" ]] && [[ "$(sg_sha256 "$dest")" == "$sha" ]]; then
		return 0
	fi
	mkdir -p "$(dirname "$dest")"
	curl --fail --location --proto '=https' --proto-redir '=https' --tlsv1.2 \
		--silent --show-error --output "${dest}.part" "$url"
	local got
	got="$(sg_sha256 "${dest}.part")"
	if [[ "$got" != "$sha" ]]; then
		rm -f "${dest}.part"
		sg_die "checksum mismatch for $url: expected $sha, got $got"
	fi
	mv "${dest}.part" "$dest"
}

# Version from manifests/runtime.toml ([runtime] version = "...").
sg_runtime_version() {
	awk -F'"' '/^\[runtime\]/ { in_rt = 1; next } /^\[/ { in_rt = 0 } in_rt && /^version/ { print $2; exit }' \
		"${SG_ROOT}/manifests/runtime.toml"
}

# License (SPDX) of a component from manifests/runtime.toml.
sg_license_of() {
	awk -F'"' -v sec="[sources.$1]" '$0 == sec { found = 1; next } /^\[/ { found = 0 } found && /^license/ { print $2; exit }' \
		"${SG_ROOT}/manifests/runtime.toml"
}
