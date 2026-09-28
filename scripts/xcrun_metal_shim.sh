#!/bin/sh
# xcrun shim for hosts where Xcode ships a non-functional `metal` stub and no
# `metallib`, while the real Metal toolchain is only available as a cryptex
# MobileAsset mount (plain `xcrun -f metal` resolves it, but
# `xcrun -sdk macosx ...` does not).
#
# Intercepts exactly the two SDK-scoped invocations issued by
# crates/3rdparties/gpui_macos/build.rs and runs them with the cryptex
# binaries. Every other invocation passes through to the real xcrun, so
# existing consumers keep working unchanged:
#   - crates/3rdparties/media/build.rs (`xcrun --sdk macosx --show-sdk-path`)
#   - scripts/run_apple_simulator_smoke.sh (`xcrun simctl ...`)
#
# Usage (the file must be reachable as `xcrun` on PATH; symlink it):
#   mkdir -p target/shim
#   ln -sf ../../scripts/xcrun_metal_shim.sh target/shim/xcrun
#   export PATH="$PWD/target/shim:$PATH"
#
# Environment:
#   REAL_XCRUN  path to the real xcrun (default: /usr/bin/xcrun).
#               Must be absolute so this shim never recurses into itself.
set -u

REAL_XCRUN="${REAL_XCRUN:-/usr/bin/xcrun}"
# Cryptex mounts come and go and `xcrun -f metal` does not always resolve to
# them, so probe the mount point directly instead of trusting the registry.
MNT_GLOB="/var/run/com.apple.security.cryptexd/mnt/com.apple.MobileAsset.MetalToolchain-*/Metal.xctoolchain/usr/bin"

resolve_tool_bin() {
    # $1 = tool name; prints the bin dir holding an executable copy.
    for dir in $MNT_GLOB; do
        if [ -x "$dir/$1" ]; then
            printf '%s' "$dir"
            return 0
        fi
    done
    # Fall back to whatever plain `xcrun -f metal` resolves (covers a
    # properly installed toolchain where the stub is replaced).
    dir=$(dirname "$("$REAL_XCRUN" -f metal)" 2>/dev/null) || return 1
    if [ -x "$dir/$1" ]; then
        printf '%s' "$dir"
        return 0
    fi
    return 1
}

if [ "$#" -ge 3 ] && [ "$1" = "-sdk" ] && [ "$2" = "macosx" ] \
    && { [ "$3" = "metal" ] || [ "$3" = "metallib" ]; }; then
    tool="$3"
    shift 3
    if ! mnt_bin=$(resolve_tool_bin "$tool"); then
        echo "xcrun_metal_shim: no executable '$tool' in the Metal toolchain mount ($MNT_GLOB)" >&2
        echo "xcrun_metal_shim: install the Metal toolchain component (xcodebuild -downloadComponent MetalToolchain)" >&2
        exit 1
    fi
    exec "$mnt_bin/$tool" "$@"
fi

exec "$REAL_XCRUN" "$@"
