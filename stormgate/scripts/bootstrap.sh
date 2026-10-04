#!/usr/bin/env bash
# Checks (and optionally installs) everything needed to build Storm Gate.
#
#   scripts/bootstrap.sh            report only
#   scripts/bootstrap.sh --install  also `brew install` missing formulae
#
# Never installs Rosetta or accepts licenses on your behalf.
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"

INSTALL=0
[[ "${1:-}" == "--install" ]] && INSTALL=1

failures=0
missing_formulae=()
ok() { printf '  \033[32m✓\033[0m %s\n' "$*"; }
warn() { printf '  \033[33m!\033[0m %s\n' "$*"; }
fail() {
	printf '  \033[31m✗\033[0m %s\n' "$*"
	failures=$((failures + 1))
}

echo "Storm Gate bootstrap"
echo
echo "System"
if ! sg_is_macos; then
	warn "not macOS ($(uname -s)): only the Rust workspace can be built and tested here"
	if command -v cargo >/dev/null; then
		ok "Rust $(cargo --version | awk '{print $2}')"
	else
		fail "Rust (https://rustup.rs)"
	fi
	exit "$failures"
fi

if [[ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" == "1" ]]; then
	ok "Apple Silicon: $(sysctl -n machdep.cpu.brand_string), $(($(sysctl -n hw.memsize) / 1073741824)) GB"
else
	warn "Intel Mac: Tier 3, not tested"
fi
macos="$(sw_vers -productVersion)"
if [[ "${macos%%.*}" -ge 14 ]]; then ok "macOS $macos"; else warn "macOS $macos is older than 14"; fi

if [[ -e /Library/Apple/usr/libexec/oah/libRosettaRuntime ]] || /usr/bin/pgrep -q oahd; then
	ok "Rosetta 2"
else
	fail "Rosetta 2 missing. Install it yourself (accepts Apple's license):"
	echo "      /usr/sbin/softwareupdate --install-rosetta --agree-to-license"
fi

if xcode-select -p >/dev/null 2>&1; then
	ok "Xcode Command Line Tools ($(xcode-select -p))"
else
	fail "Xcode Command Line Tools: run xcode-select --install"
fi

free_gb="$(df -g "$SG_ROOT" | awk 'NR == 2 { print $4 }')"
if [[ "$free_gb" -ge 40 ]]; then
	ok "${free_gb} GB free"
else
	warn "only ${free_gb} GB free; a full runtime build needs ~40 GB (set STORMGATE_BUILD_DIR to an external SSD)"
fi

echo
echo "Tools (arm64 Homebrew)"
BREW="${BREW:-$(command -v brew || true)}"
if [[ -z "$BREW" ]]; then
	fail "Homebrew not found: https://brew.sh (download and review the installer before running it)"
else
	ok "Homebrew $("$BREW" --version | head -1 | awk '{print $2}')"
fi

# command -> formula
check_tool() {
	local cmd="$1" formula="$2"
	if command -v "$cmd" >/dev/null 2>&1; then
		ok "$cmd"
	else
		fail "$cmd (brew install $formula)"
		missing_formulae+=("$formula")
	fi
}
check_tool git git
check_tool cmake cmake
check_tool meson meson
check_tool ninja ninja
check_tool pkg-config pkgconf
check_tool x86_64-w64-mingw32-gcc mingw-w64
check_tool glslangValidator glslang
check_tool python3 python
check_tool cargo rustup
# macOS ships bison 2.3 and an old flex; Wine needs newer ones.
for kegonly in bison flex; do
	if [[ -n "$BREW" ]] && [[ -x "$("$BREW" --prefix)/opt/$kegonly/bin/$kegonly" ]]; then
		ok "$kegonly (Homebrew, keg-only)"
	else
		fail "$kegonly (brew install $kegonly)"
		missing_formulae+=("$kegonly")
	fi
done

echo
echo "x86_64 libraries for Wine (Intel Homebrew in /usr/local, run under Rosetta)"
if [[ -x /usr/local/bin/brew ]]; then
	ok "x86_64 Homebrew"
	for f in freetype gnutls sdl2; do
		if [[ -d "/usr/local/opt/$f" ]]; then ok "$f (x86_64)"; else fail "$f (arch -x86_64 /usr/local/bin/brew install $f)"; fi
	done
else
	fail "x86_64 Homebrew missing; see docs/building.md#x86_64-homebrew"
fi

if [[ "$INSTALL" == 1 ]] && [[ ${#missing_formulae[@]} -gt 0 ]] && [[ -n "$BREW" ]]; then
	echo
	sg_log "installing: ${missing_formulae[*]}"
	"$BREW" install "${missing_formulae[@]}"
	exec "$0"
fi

echo
if [[ "$failures" -eq 0 ]]; then
	echo "Result: READY (next: make runtime)"
else
	echo "Result: $failures problem(s); fix them and re-run (or use --install for Homebrew formulae)"
fi
exit "$failures"
