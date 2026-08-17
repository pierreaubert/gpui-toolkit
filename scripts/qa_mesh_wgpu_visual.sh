#!/usr/bin/env bash
# Capture and compare the retained MeshPlot WGPU renderer contract.
#
# This lane is deliberately independent of component-lab's Metal-only
# screenshot harness. It produces real adapter-backed PNGs for representative
# MeshPlot modes and compares their manifest/checksum to a versioned WGPU
# baseline. A missing adapter is a skip for ordinary developer QA and a hard
# failure when QA_WGPU_REQUIRED=1 (as used by release CI).
set -euo pipefail

cd "$(dirname "$0")/.."

# Keep macOS shader-module compilation out of a developer's protected global
# Clang cache. CI and callers may provide their own cache explicitly; the
# default is task-scoped and disposable so the wrapper can reach the adapter
# probe instead of failing before WGPU QA starts.
if [[ "$(uname -s)" == "Darwin" && -z "${CLANG_MODULE_CACHE_PATH:-}" ]]; then
    CLANG_MODULE_CACHE_PATH="${MESH_PLOT_CLANG_MODULE_CACHE_PATH:-${TMPDIR:-/tmp}/gpui-toolkit-clang-cache}"
    export CLANG_MODULE_CACHE_PATH
fi

output_root="${QA_WGPU_VISUAL_OUTPUT_ROOT:-target/qa/visual/mesh-plot-wgpu}"
actual_dir="$output_root/actual"
baseline_dir="${QA_WGPU_VISUAL_BASELINE_DIR:-qa/visual/baselines/mesh-plot-wgpu-v1}"
required="${QA_WGPU_REQUIRED:-0}"
mkdir -p "$actual_dir"
# The capture lane owns this manifest; do not retain a prior captured manifest
# if compilation or adapter discovery takes a skip path.
rm -f "$actual_dir/manifest.json"
capture_log="$(mktemp -t mesh-plot-wgpu.XXXXXX)"
trap 'rm -f "$capture_log"' EXIT

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
    echo "MeshPlot WGPU QA requires cargo and python3" >&2
    exit 2
fi

if "$cargo_bin" run -p gpui-d3rs --example mesh_wgpu_visual_capture --features gpu-3d -- \
    --output-dir "$actual_dir" >"$capture_log" 2>&1; then
    /bin/cat "$capture_log"
else
    /bin/cat "$capture_log" >&2
    if ! /usr/bin/grep -Eq "WGPU adapter unavailable|No suitable graphics adapter" "$capture_log"; then
        exit 1
    fi
    "$python_bin" scripts/mesh_wgpu_manifest.py \
        --write-skip "$actual_dir/manifest.json" \
        --reason "no usable WGPU adapter"
fi

manifest_args=(
    --actual "$actual_dir/manifest.json"
    --baseline "$baseline_dir/manifest.json"
    --repo-root "$PWD"
)
if [[ "$required" == "1" ]]; then
    manifest_args+=(--required)
fi
"$python_bin" scripts/mesh_wgpu_manifest.py "${manifest_args[@]}"
