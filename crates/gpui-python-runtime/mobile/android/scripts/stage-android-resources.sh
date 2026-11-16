#!/bin/bash
# Stage everything an Android Python host project bundles into its APK:
#   <crate>/vendor/prefix/                 pinned CPython Android build
#   <project>/gradle/app/src/main/assets/python/   CPython home
#   <project>/gradle/app/src/main/assets/app/      toolkit package + app entry
#   <project>/gradle/app/src/main/assets/gpui_bundle_manifest.txt
#   <project>/gradle/app/src/main/jniLibs/arm64-v8a/ libpython + support libs
#
# Usage: stage-android-resources.sh [android-project-dir] [app-source-dir]
#   android-project-dir  directory containing gradle/ (default: this crate)
#   app-source-dir       directory holding main_app.py (+ sibling modules);
#                        defaults to staging the tutorial demo app
# Requires ANDROID_NDK_HOME. CARGO_TARGET_DIR is honored.
set -euo pipefail

PYTHON_VERSION="3.15.2"
PYTHON_SHA256="cfd4f0fdaff465ba62ebf3c3df2c8c410d0802fef3f98897082460df704cfef6"
PYTHON_URL="https://www.python.org/ftp/python/${PYTHON_VERSION}/python-${PYTHON_VERSION}-aarch64-linux-android.tar.gz"
RUST_TARGET="aarch64-linux-android"

CRATE_ANDROID_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ANDROID_DIR="${1:-$CRATE_ANDROID_DIR}"
APP_SOURCE_DIR="${2:-}"
# Later steps cd into the workspace, so resolve caller-relative paths now.
case "$ANDROID_DIR" in /*) ;; *) ANDROID_DIR="$PWD/$ANDROID_DIR" ;; esac
if [ -n "$APP_SOURCE_DIR" ]; then
    case "$APP_SOURCE_DIR" in /*) ;; *) APP_SOURCE_DIR="$PWD/$APP_SOURCE_DIR" ;; esac
fi
WORKSPACE_ROOT="$(cd "$CRATE_ANDROID_DIR/../../../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$WORKSPACE_ROOT/target}"
PREFIX="$CRATE_ANDROID_DIR/vendor/prefix"
ASSETS="$ANDROID_DIR/gradle/app/src/main/assets"
JNILIBS="$ANDROID_DIR/gradle/app/src/main/jniLibs/arm64-v8a"

if [ ! -d "$PREFIX" ]; then
    mkdir -p "$CRATE_ANDROID_DIR/vendor"
    TARBALL="$CRATE_ANDROID_DIR/vendor/python-${PYTHON_VERSION}-aarch64-linux-android.tar.gz"
    echo "Downloading CPython ${PYTHON_VERSION} Android build..."
    curl -fsSL "$PYTHON_URL" -o "$TARBALL"
    echo "${PYTHON_SHA256}  ${TARBALL}" | shasum -a 256 -c -
    tar -xzf "$TARBALL" -C "$CRATE_ANDROID_DIR/vendor" prefix
    rm -f "$TARBALL"
fi

echo "Building _native extension for $RUST_TARGET..."
cd "$WORKSPACE_ROOT"
PYO3_CROSS=1 \
PYO3_CROSS_PYTHON_VERSION=3.15 \
PYO3_CROSS_LIB_DIR="$PREFIX/lib" \
    cargo ndk -t arm64-v8a -P 26 build -p gpui-python-runtime \
        --features python-extension --lib --release

NATIVE_DYLIB="$TARGET_DIR/$RUST_TARGET/release/libgpui_python_runtime.so"
if [ ! -f "$NATIVE_DYLIB" ]; then
    echo "missing built extension: $NATIVE_DYLIB" >&2
    exit 1
fi

echo "Staging assets in $ASSETS..."
rm -rf "$ASSETS/python" "$ASSETS/app" "$ASSETS/gpui_bundle_manifest.txt"
mkdir -p "$ASSETS/python/lib" "$JNILIBS"
rsync -a \
    --exclude '/python3.15/test' \
    --exclude '/python3.15/idlelib' \
    --exclude '/python3.15/turtledemo' \
    --exclude '__pycache__' \
    --exclude '/python3.15/ensurepip/_bundled' \
    "$PREFIX/lib/python3.15" "$ASSETS/python/lib/"
cp "$PREFIX/lib/libpython3.15.so" "$JNILIBS/"
cp "$PREFIX"/lib/lib*_python.so "$JNILIBS/"

mkdir -p "$ASSETS/app"
if [ -n "$APP_SOURCE_DIR" ]; then
    rsync -a --exclude '__pycache__' --exclude 'gpui_toolkit' \
        "$APP_SOURCE_DIR/" "$ASSETS/app/"
else
    cp "$WORKSPACE_ROOT/crates/gpui-python-runtime/python/examples/mobile_signal_lab.py" \
        "$ASSETS/app/main_app.py"
fi
if [ ! -f "$ASSETS/app/main_app.py" ]; then
    echo "app staging produced no main_app.py (source: ${APP_SOURCE_DIR:-tutorial demo})" >&2
    exit 1
fi
rsync -a \
    --exclude '__pycache__' \
    --exclude '_native.abi3.so' \
    --exclude 'bin' \
    "$WORKSPACE_ROOT/crates/gpui-python-runtime/python/gpui_toolkit" \
    "$ASSETS/app/"
cp "$NATIVE_DYLIB" "$ASSETS/app/gpui_toolkit/_native.abi3.so"

# Launcher icons: copy the toolkit defaults when the project has none.
RES_DIR="$ANDROID_DIR/gradle/app/src/main/res"
if ! ls "$RES_DIR"/mipmap-*/ic_launcher.png >/dev/null 2>&1; then
    mkdir -p "$RES_DIR"
    cp -R "$CRATE_ANDROID_DIR/../assets/android-res/." "$RES_DIR/"
fi

# One relative path per line: the Rust extractor's worklist.
(cd "$ASSETS" && find python app -type f | sort) > "$ASSETS/gpui_bundle_manifest.txt"

echo "Staged: $(wc -l < "$ASSETS/gpui_bundle_manifest.txt") bundled files, jniLibs in $JNILIBS"
