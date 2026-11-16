#!/bin/bash
# Build a debug APK: stage Python resources, build the Rust host library
# with cargo-ndk into jniLibs, then assembleDebug via Gradle.
#
# Usage: build-apk.sh [android-project-dir] [app-source-dir]
# Requires ANDROID_NDK_HOME (NDK 27.x) and ANDROID_HOME. JAVA_HOME
# defaults to Android Studio's bundled JBR.
set -euo pipefail

CRATE_ANDROID_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ANDROID_DIR="${1:-$CRATE_ANDROID_DIR}"
APP_SOURCE_DIR="${2:-}"
# The build cds into the workspace, so resolve caller-relative paths now.
case "$ANDROID_DIR" in /*) ;; *) ANDROID_DIR="$PWD/$ANDROID_DIR" ;; esac
if [ -n "$APP_SOURCE_DIR" ]; then
    case "$APP_SOURCE_DIR" in /*) ;; *) APP_SOURCE_DIR="$PWD/$APP_SOURCE_DIR" ;; esac
fi
WORKSPACE_ROOT="$(cd "$CRATE_ANDROID_DIR/../../../.." && pwd)"
export ANDROID_HOME="${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/27.2.12479018}"
export JAVA_HOME="${JAVA_HOME:-/Applications/Android Studio.app/Contents/jbr/Contents/Home}"
export GPUI_PYTHON_ANDROID_PREFIX="$CRATE_ANDROID_DIR/vendor/prefix"
export PYO3_CROSS=1
export PYO3_CROSS_PYTHON_VERSION=3.15
export PYO3_CROSS_LIB_DIR="$GPUI_PYTHON_ANDROID_PREFIX/lib"

"$CRATE_ANDROID_DIR/scripts/stage-android-resources.sh" "$ANDROID_DIR" "$APP_SOURCE_DIR"

echo "=== Building Rust host library (arm64-v8a) ==="
cd "$WORKSPACE_ROOT"
cargo ndk -t arm64-v8a -P 26 \
    -o "$ANDROID_DIR/gradle/app/src/main/jniLibs" \
    build -p gpui-python-mobile-android --release
# cargo-ndk copies every cdylib it built, including the dependency
# artifact libgpui_python_runtime.so (the _native extension ships inside
# assets instead, where CPython loads it as a module).
rm -f "$ANDROID_DIR/gradle/app/src/main/jniLibs/arm64-v8a/libgpui_python_runtime.so"

GRADLE_PROPS=()
[ -n "${GPUI_VERSION_NAME:-}" ] && GRADLE_PROPS+=("-PgpuiVersionName=$GPUI_VERSION_NAME")
[ -n "${GPUI_VERSION_CODE:-}" ] && GRADLE_PROPS+=("-PgpuiVersionCode=$GPUI_VERSION_CODE")

echo "=== Assembling debug APK ==="
cd "$ANDROID_DIR/gradle"
if [ -x ./gradlew ]; then
    ./gradlew :app:assembleDebug ${GRADLE_PROPS[@]+"${GRADLE_PROPS[@]}"}
else
    gradle :app:assembleDebug ${GRADLE_PROPS[@]+"${GRADLE_PROPS[@]}"}
fi

APK="$ANDROID_DIR/gradle/app/build/outputs/apk/debug/app-debug.apk"
echo "=== Built $APK ==="
ls -lh "$APK"
