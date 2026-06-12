#!/bin/bash
# Build the Rust static library for iOS simulator or device.
#
# This script is called from Xcode's pre-build phase and also works standalone.
# It detects the target architecture from PLATFORM_NAME/ARCHS (set by Xcode)
# or defaults to the iOS simulator.
#
# Output: lib/libshowcase_ios.a

set -euo pipefail

# Navigate to the workspace root (4 levels up from ios/)
WORKSPACE_ROOT="$(cd "$(dirname "$0")/../../../.." && pwd)"
IOS_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== Building Rust static library for iOS ==="
echo "  Workspace: $WORKSPACE_ROOT"
echo "  iOS dir:   $IOS_DIR"

# Determine Rust target from Xcode environment or default to simulator
if [ "${PLATFORM_NAME:-}" = "iphoneos" ]; then
    RUST_TARGET="aarch64-apple-ios"
    PROFILE="release"
elif [ "${PLATFORM_NAME:-}" = "iphonesimulator" ]; then
    RUST_TARGET="aarch64-apple-ios-sim"
    PROFILE="release"
else
    # Default: simulator (for standalone builds)
    RUST_TARGET="aarch64-apple-ios-sim"
    PROFILE="release"
fi

echo "  Target:    $RUST_TARGET"
echo "  Profile:   $PROFILE"

# Build the static library
cd "$WORKSPACE_ROOT"

if [ "$PROFILE" = "release" ]; then
    cargo build -p gpui-ui-kit-ios-showcase --target "$RUST_TARGET" --release
    LIB_DIR="target/$RUST_TARGET/release"
else
    cargo build -p gpui-ui-kit-ios-showcase --target "$RUST_TARGET"
    LIB_DIR="target/$RUST_TARGET/debug"
fi

# Copy the .a file to the Xcode project's lib/ directory
mkdir -p "$IOS_DIR/lib"
cp "$LIB_DIR/libshowcase_ios.a" "$IOS_DIR/lib/"

echo "=== Rust static library built: $IOS_DIR/lib/libshowcase_ios.a ==="
ls -lh "$IOS_DIR/lib/libshowcase_ios.a"
