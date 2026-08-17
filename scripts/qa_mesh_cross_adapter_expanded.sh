#!/usr/bin/env bash
# Compare expanded adapter-state MeshPlot captures.
#
# The canonical six-case release lane remains intentionally separate. This
# lane covers camera, displayed range, and NaN masking so state changes can be
# compared without changing the stable baseline contract.
set -euo pipefail

cd "$(dirname "$0")/.."

required="${QA_EXPANDED_REQUIRED:-0}"
root="${QA_MESH_EXPANDED_OUTPUT_ROOT:-target/qa/visual/mesh-plot-expanded}"
metal_dir="$root/metal/actual"
wgpu_dir="$root/wgpu/actual"
report_path="${QA_MESH_EXPANDED_REPORT:-target/qa/visual/mesh-plot-cross-adapter-expanded.json}"

cargo_bin="${MESH_PLOT_CARGO_BIN:-}"
if [[ -z "$cargo_bin" ]]; then
    cargo_bin="$(command -v cargo 2>/dev/null || true)"
fi
if [[ -z "$cargo_bin" && -x "$HOME/.cargo/bin/cargo" ]]; then
    cargo_bin="$HOME/.cargo/bin/cargo"
fi
python_bin="${MESH_PLOT_PYTHON_BIN:-}"
if [[ -z "$python_bin" ]]; then
    python_bin="$(command -v python3 2>/dev/null || true)"
fi
if [[ -z "$python_bin" && -x /opt/homebrew/bin/python3 ]]; then
    python_bin=/opt/homebrew/bin/python3
fi

if [[ -z "$cargo_bin" || -z "$python_bin" ]]; then
    echo "expanded MeshPlot QA requires cargo and python3" >&2
    exit 2
fi

mkdir -p "$metal_dir" "$wgpu_dir"
# Never let an adapter-unavailable developer run reuse a prior capture or
# comparison report. The release validator must see either a fresh report or
# no expanded evidence at all.
rm -f "$metal_dir/manifest.json" "$wgpu_dir/manifest.json" "$report_path"

if [[ "$(uname -s)" != "Darwin" ]]; then
    if [[ "$required" == "1" ]]; then
        echo "expanded MeshPlot QA requires macOS Metal and WGPU adapters" >&2
        exit 1
    fi
    echo "expanded MeshPlot QA skipped: macOS Metal adapter is unavailable"
    exit 0
fi

if [[ -z "${CLANG_MODULE_CACHE_PATH:-}" ]]; then
    export CLANG_MODULE_CACHE_PATH="${MESH_PLOT_CLANG_MODULE_CACHE_PATH:-${TMPDIR:-/tmp}/gpui-toolkit-clang-cache}"
fi

capture_or_skip() {
    local label="$1"
    shift
    local log
    log="$(mktemp -t mesh-plot-expanded.XXXXXX)"
    if "$@" >"$log" 2>&1; then
        /bin/cat "$log"
        rm -f "$log"
        return 0
    fi
    /bin/cat "$log" >&2
    if [[ "$required" != "1" ]] && /usr/bin/grep -Eqi 'adapter unavailable|no usable .*adapter|requires macOS' "$log"; then
        echo "expanded MeshPlot QA skipped: $label adapter is unavailable"
        rm -f "$log"
        exit 0
    fi
    rm -f "$log"
    return 1
}

capture_or_skip Metal \
    "$cargo_bin" run -p gpui-d3rs --example mesh_metal_visual_capture \
    --features metal-qa -- --case-set expanded --output-dir "$metal_dir"
capture_or_skip WGPU \
    "$cargo_bin" run -p gpui-d3rs --example mesh_wgpu_visual_capture \
    --features gpu-3d -- --case-set expanded --output-dir "$wgpu_dir"

PYTHONPATH=scripts "$python_bin" scripts/mesh_plot_visual_compare.py \
    --left "$metal_dir/manifest.json" \
    --right "$wgpu_dir/manifest.json" \
    --repo-root "$PWD" \
    --max-channel-delta 0 \
    --max-changed-fraction 0 \
    --output-report "$report_path"

PYTHONPATH=scripts "$python_bin" - "$report_path" <<'PY'
import sys
from pathlib import Path

from mesh_plot_expanded_visual import ExpandedVisualError, validate_expanded_report

try:
    validate_expanded_report(Path(sys.argv[1]))
except ExpandedVisualError as error:
    raise SystemExit(str(error)) from error
print("expanded MeshPlot cross-adapter release report validated (3 cases)")
PY
