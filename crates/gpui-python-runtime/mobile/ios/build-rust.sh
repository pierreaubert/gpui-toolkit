#!/bin/bash
# Build the Rust static library (GPUI host + embedded CPython session)
# for the iOS simulator or device, staging Python resources first.
#
# Usage: build-rust.sh [ios-project-dir] [app-source-dir]
# Detects the target from PLATFORM_NAME (set by Xcode) or defaults to
# the simulator. Output: <ios-project-dir>/lib/libpython_mobile_ios.a
set -euo pipefail

CRATE_IOS_DIR="$(cd "$(dirname "$0")" && pwd)"
IOS_DIR="${1:-$CRATE_IOS_DIR}"
APP_SOURCE_DIR="${2:-}"
# The build cds into the workspace, so resolve caller-relative paths now.
case "$IOS_DIR" in /*) ;; *) IOS_DIR="$PWD/$IOS_DIR" ;; esac
if [ -n "$APP_SOURCE_DIR" ]; then
    case "$APP_SOURCE_DIR" in /*) ;; *) APP_SOURCE_DIR="$PWD/$APP_SOURCE_DIR" ;; esac
fi
WORKSPACE_ROOT="$(cd "$CRATE_IOS_DIR/../../../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$WORKSPACE_ROOT/target}"

if [ "${PLATFORM_NAME:-}" = "iphoneos" ]; then
    RUST_TARGET="aarch64-apple-ios"
    SLICE="ios-arm64"
else
    RUST_TARGET="aarch64-apple-ios-sim"
    SLICE="ios-arm64_x86_64-simulator"
fi

echo "=== Building Python mobile iOS host ($RUST_TARGET) ==="
"$CRATE_IOS_DIR/scripts/stage-ios-resources.sh" "$RUST_TARGET" "$IOS_DIR" "$APP_SOURCE_DIR"

export GPUI_PYTHON_XCFRAMEWORK="$CRATE_IOS_DIR/vendor/Python.xcframework"
export PYO3_CROSS=1
export PYO3_CROSS_PYTHON_VERSION=3.15
export PYO3_CROSS_LIB_DIR="$GPUI_PYTHON_XCFRAMEWORK/$SLICE/lib-arm64"

cd "$WORKSPACE_ROOT"
cargo build -p gpui-python-mobile-ios --target "$RUST_TARGET" --release

mkdir -p "$IOS_DIR/lib"
cp "$TARGET_DIR/$RUST_TARGET/release/libpython_mobile_ios.a" "$IOS_DIR/lib/"
echo "=== Built $IOS_DIR/lib/libpython_mobile_ios.a ==="
ls -lh "$IOS_DIR/lib/libpython_mobile_ios.a"
