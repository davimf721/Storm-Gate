#!/usr/bin/env bash
# Verifies the patch queues: every series entry exists, every patch file is
# listed, and every patch carries the required metadata header.
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"

FIELDS=(Storm-Gate-Patch Problem Justification Test Upstream-Issue Upstream-Status)
STATUSES="not-submitted submitted accepted rejected not-applicable"
errors=0
err() {
	echo "FAIL $*"
	errors=$((errors + 1))
}

for dir in "${SG_ROOT}"/patches/*/; do
	component="$(basename "$dir")"
	series="${dir}series"
	[[ -f "$series" ]] || {
		err "$component: missing series file"
		continue
	}
	listed=()
	while IFS= read -r line || [[ -n "$line" ]]; do
		line="${line%%#*}"
		line="${line//[[:space:]]/}"
		[[ -n "$line" ]] && listed+=("$line")
	done <"$series"
	for p in "${listed[@]+"${listed[@]}"}"; do
		file="${dir}${p}"
		[[ -f "$file" ]] || {
			err "$component: series lists missing $p"
			continue
		}
		for field in "${FIELDS[@]}"; do
			grep -q "^${field}: ." "$file" || err "$component/$p: missing '${field}:' header"
		done
		status="$(sed -n 's/^Upstream-Status: *//p' "$file" | head -1)"
		[[ " $STATUSES " == *" ${status} "* ]] || err "$component/$p: invalid Upstream-Status '$status'"
	done
	for file in "$dir"*.patch; do
		[[ -e "$file" ]] || continue
		name="$(basename "$file")"
		printf '%s\n' "${listed[@]+"${listed[@]}"}" | grep -qxF "$name" || err "$component: $name is not listed in series"
	done
done

if [[ "$errors" -eq 0 ]]; then
	echo "ok   patch queues"
fi
exit "$errors"
