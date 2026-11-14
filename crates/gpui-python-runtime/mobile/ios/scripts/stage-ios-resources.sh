#!/bin/bash
# Stage everything an iOS Python host project bundles:
#   <ios-project>/vendor/Python.xcframework   pinned CPython (symlink to
#                                             the toolkit crate's vendor copy when
#                                             staging for another project)
#   <ios-project>/app/gpui_toolkit/           toolkit Python package
#   <ios-project>/app/gpui_toolkit/_native.abi3.so
#   <ios-project>/app/main_app.py (+ the rest of the app sources)
#
# Usage: stage-ios-resources.sh [rust-target] [ios-project-dir] [app-source-dir]
#   rust-target      aarch64-apple-ios-sim (default) or aarch64-apple-ios
#   ios-project-dir  defaults to this crate's ios/ directory
#   app-source-dir   directory holding main_app.py (+ sibling modules);
#                    defaults to staging the tutorial demo app
# CARGO_TARGET_DIR is honored for artifact lookup.
set -euo pipefail

RUST_TARGET="${1:-aarch64-apple-ios-sim}"
PYTHON_VERSION="3.15.2"
PYTHON_SHA256="0f9fc7f61ceb8278a69fc7ec6089aad81789a96fa42f31cfe6d2f5f860d20a62"
PYTHON_URL="https://www.python.org/ftp/python/${PYTHON_VERSION}/python-${PYTHON_VERSION}-iOS-XCframework.tar.gz"

CRATE_IOS_DIR="$(cd "$(dirname "$0")/.." && pwd)"
IOS_DIR="${2:-$CRATE_IOS_DIR}"
APP_SOURCE_DIR="${3:-}"
# Later steps cd into the workspace, so resolve caller-relative paths now.
case "$IOS_DIR" in /*) ;; *) IOS_DIR="$PWD/$IOS_DIR" ;; esac
if [ -n "$APP_SOURCE_DIR" ]; then
    case "$APP_SOURCE_DIR" in /*) ;; *) APP_SOURCE_DIR="$PWD/$APP_SOURCE_DIR" ;; esac
fi
WORKSPACE_ROOT="$(cd "$CRATE_IOS_DIR/../../../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$WORKSPACE_ROOT/target}"
VENDOR_DIR="$CRATE_IOS_DIR/vendor"
XCFRAMEWORK="$VENDOR_DIR/Python.xcframework"

case "$RUST_TARGET" in
    aarch64-apple-ios) SLICE="ios-arm64" ;;
    aarch64-apple-ios-sim) SLICE="ios-arm64_x86_64-simulator" ;;
    *) echo "unsupported iOS Rust target: $RUST_TARGET" >&2; exit 1 ;;
esac

if [ ! -d "$XCFRAMEWORK" ]; then
    mkdir -p "$VENDOR_DIR"
    TARBALL="$VENDOR_DIR/python-${PYTHON_VERSION}-iOS-XCframework.tar.gz"
    echo "Downloading CPython ${PYTHON_VERSION} iOS XCframework..."
    curl -fsSL "$PYTHON_URL" -o "$TARBALL"
    echo "${PYTHON_SHA256}  ${TARBALL}" | shasum -a 256 -c -
    tar -xzf "$TARBALL" -C "$VENDOR_DIR" Python.xcframework
    rm -f "$TARBALL"
fi

echo "Building _native extension for $RUST_TARGET..."
cd "$WORKSPACE_ROOT"
PYO3_CROSS=1 \
PYO3_CROSS_PYTHON_VERSION=3.15 \
PYO3_CROSS_LIB_DIR="$XCFRAMEWORK/$SLICE/lib-arm64" \
    cargo build -p gpui-python-runtime --features python-extension \
        --lib --release --target "$RUST_TARGET"

NATIVE_DYLIB="$TARGET_DIR/$RUST_TARGET/release/libgpui_python_runtime.dylib"
if [ ! -f "$NATIVE_DYLIB" ]; then
    echo "missing built extension: $NATIVE_DYLIB" >&2
    exit 1
fi

echo "Staging app resources in $IOS_DIR/app..."
mkdir -p "$IOS_DIR/app"
if [ -n "$APP_SOURCE_DIR" ]; then
    rsync -a --delete --exclude '__pycache__' --exclude 'gpui_toolkit' \
        "$APP_SOURCE_DIR/" "$IOS_DIR/app/"
else
    rm -f "$IOS_DIR/app/main_app.py"
    cp "$WORKSPACE_ROOT/crates/gpui-python-runtime/python/examples/mobile_signal_lab.py" \
        "$IOS_DIR/app/main_app.py"
fi
if [ ! -f "$IOS_DIR/app/main_app.py" ]; then
    echo "app staging produced no main_app.py (source: ${APP_SOURCE_DIR:-tutorial demo})" >&2
    exit 1
fi
rm -rf "$IOS_DIR/app/gpui_toolkit"
rsync -a \
    --exclude '__pycache__' \
    --exclude '_native.abi3.so' \
    --exclude 'bin' \
    "$WORKSPACE_ROOT/crates/gpui-python-runtime/python/gpui_toolkit" \
    "$IOS_DIR/app/"
cp "$NATIVE_DYLIB" "$IOS_DIR/app/gpui_toolkit/_native.abi3.so"

# App icon: an app-provided icon-1024.png wins over the toolkit default.
ICON_SRC="$WORKSPACE_ROOT/crates/gpui-python-runtime/mobile/assets/icon-1024.png"
if [ -n "$APP_SOURCE_DIR" ] && [ -f "$APP_SOURCE_DIR/icon-1024.png" ]; then
    ICON_SRC="$APP_SOURCE_DIR/icon-1024.png"
elif [ -f "$IOS_DIR/../app/icon-1024.png" ]; then
    ICON_SRC="$IOS_DIR/../app/icon-1024.png"
fi
for app_target_dir in "$IOS_DIR"/*App; do
    [ -d "$app_target_dir" ] || continue
    icon_dest="$app_target_dir/Assets.xcassets/AppIcon.appiconset"
    mkdir -p "$icon_dest"
    cp "$WORKSPACE_ROOT/crates/gpui-python-runtime/mobile/assets/AppIcon.appiconset/Contents.json" \
        "$icon_dest/Contents.json"
    cp "$ICON_SRC" "$icon_dest/icon-1024.png"
done

# The Xcode project links and embeds vendor/Python.xcframework relative
# to itself; link the toolkit's verified copy into external projects.
if [ "$IOS_DIR" != "$CRATE_IOS_DIR" ]; then
    mkdir -p "$IOS_DIR/vendor"
    ln -sfn "$XCFRAMEWORK" "$IOS_DIR/vendor/Python.xcframework"
fi

echo "Staged: Python.xcframework ($SLICE), app/main_app.py, app/gpui_toolkit/_native.abi3.so"
