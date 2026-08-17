#!/usr/bin/env bash
# Capture the retained MeshPlot scenes through the native Metal adapter.
#
# Developer runs skip cleanly when Metal is unavailable. Release callers set
# QA_METAL_REQUIRED=1 and get a hard failure instead of silently accepting a
# missing renderer-backed reference.
set -euo pipefail

cd "$(dirname "$0")/.."

output_root="${QA_METAL_VISUAL_OUTPUT_ROOT:-target/qa/visual/mesh-plot-metal}"
actual_dir="$output_root/actual"
required="${QA_METAL_REQUIRED:-0}"
mkdir -p "$actual_dir"
# Clear only this lane's generated manifest before platform/adapter probing.
rm -f "$actual_dir/manifest.json"
capture_log="$(mktemp -t mesh-plot-metal.XXXXXX)"
trap 'rm -f "$capture_log"' EXIT

if [[ "$(uname -s)" != "Darwin" ]]; then
    if [[ "$required" == "1" ]]; then
        echo "MeshPlot Metal QA requires macOS" >&2
        exit 1
    fi
    echo "MeshPlot Metal QA skipped: native Metal is unavailable on this platform"
    exit 0
fi

# Keep macOS shader-module compilation out of a developer's protected global
# Clang cache, matching the WGPU adapter-backed lane.
if [[ -z "${CLANG_MODULE_CACHE_PATH:-}" ]]; then
    CLANG_MODULE_CACHE_PATH="${MESH_PLOT_CLANG_MODULE_CACHE_PATH:-${TMPDIR:-/tmp}/gpui-toolkit-clang-cache}"
    export CLANG_MODULE_CACHE_PATH
fi

cargo_bin="${MESH_PLOT_CARGO_BIN:-}"
if [[ -z "$cargo_bin" ]]; then
    cargo_bin="$(command -v cargo 2>/dev/null || true)"
fi
if [[ -z "$cargo_bin" && -x "${HOME}/.cargo/bin/cargo" ]]; then
    cargo_bin="${HOME}/.cargo/bin/cargo"
fi
python_bin="${MESH_PLOT_PYTHON_BIN:-}"
if [[ -z "$python_bin" ]]; then
    python_bin="$(command -v python3 2>/dev/null || true)"
fi
if [[ -z "$python_bin" && -x /opt/homebrew/bin/python3 ]]; then
    python_bin=/opt/homebrew/bin/python3
fi
if [[ -z "$cargo_bin" || -z "$python_bin" ]]; then
    echo "MeshPlot Metal QA requires cargo and python3" >&2
    exit 2
fi

if "$cargo_bin" run -p gpui-d3rs --example mesh_metal_visual_capture --features metal-qa -- \
    --output-dir "$actual_dir" >"$capture_log" 2>&1; then
    /bin/cat "$capture_log"
else
    /bin/cat "$capture_log" >&2
    if ! /usr/bin/grep -Eq "Metal adapter unavailable|requires macOS" "$capture_log"; then
        exit 1
    fi
    "$python_bin" scripts/mesh_wgpu_manifest.py \
        --write-skip "$actual_dir/manifest.json" \
        --renderer metal-headless \
        --reason "no usable Metal adapter"
fi

manifest_args=(
    --actual "$actual_dir/manifest.json"
    --renderer metal-headless
    --repo-root "$PWD"
)
if [[ "$required" == "1" ]]; then
    manifest_args+=(--required --capture-only)
fi
"$python_bin" scripts/mesh_wgpu_manifest.py "${manifest_args[@]}"
