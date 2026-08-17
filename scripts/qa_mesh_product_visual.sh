#!/usr/bin/env bash
# Capture the high-level GPUI MeshPlot composition through both native
# headless adapters.  The test is intentionally product-level: it builds the
# same axes/colorbar/selection tree for Metal and WGPU before reading pixels.
# Developer runs write an explicit skipped manifest when adapters are absent;
# release callers set QA_PRODUCT_REQUIRED=1 and reject that result.
set -euo pipefail

cd "$(dirname "$0")/.."

output_root="${QA_MESH_PRODUCT_OUTPUT_ROOT:-target/qa/visual/mesh-plot-product}"
required="${QA_PRODUCT_REQUIRED:-0}"
actual_dir="$output_root"

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
    echo "high-level MeshPlot product QA requires cargo and python3" >&2
    exit 2
fi

mkdir -p "$actual_dir"
# Remove only the exact generated files owned by this lane so a skipped run
# cannot accidentally reuse a prior adapter capture.
rm -f \
    "$actual_dir/manifest.json" \
    "$actual_dir/metal/plain.png" \
    "$actual_dir/metal/selected.png" \
    "$actual_dir/wgpu/plain.png" \
    "$actual_dir/wgpu/selected.png"

source_revision="$(git rev-parse HEAD)"
source_dirty=false
if [[ -n "$(git status --porcelain=v1 --untracked-files=all)" ]]; then
    source_dirty=true
fi

write_skip_manifest() {
    local reason="$1"
    "$python_bin" - "$actual_dir/manifest.json" "$reason" "$source_revision" "$source_dirty" <<'PY'
import json
import sys
from pathlib import Path

path, reason, revision, dirty = sys.argv[1:]
Path(path).write_text(
    json.dumps(
        {
            "schema_version": 1,
            "report_type": "gpui-mesh-plot-product-capture",
            "status": "skipped",
            "reason": reason,
            "source_revision": revision,
            "source_dirty": dirty == "true",
            "cases": [],
        },
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY
}

if [[ "$(uname -s)" != "Darwin" ]]; then
    if [[ "$required" == "1" ]]; then
        echo "high-level MeshPlot product QA requires macOS Metal and WGPU adapters" >&2
        exit 1
    fi
    write_skip_manifest "native Metal/WGPU adapters require macOS"
else
    if [[ -z "${CLANG_MODULE_CACHE_PATH:-}" ]]; then
        export CLANG_MODULE_CACHE_PATH="${MESH_PLOT_CLANG_MODULE_CACHE_PATH:-${TMPDIR:-/tmp}/gpui-toolkit-clang-cache}"
    fi
    "$cargo_bin" test -p gpui-px --test mesh_plot_wgpu_product \
        --features native-qa,headless-qa --no-run
    test_binary="$($python_bin - <<'PY'
from pathlib import Path

candidates = [
    path
    for path in Path("target/debug/deps").glob("mesh_plot_wgpu_product-*")
    if path.is_file() and path.stat().st_mode & 0o111
]
if candidates:
    print(max(candidates, key=lambda path: path.stat().st_mtime_ns))
PY
)"
    if [[ -z "$test_binary" ]]; then
        echo "could not locate the built MeshPlot product test binary" >&2
        exit 1
    fi
    GPUI_MESH_PLOT_PRODUCT_CAPTURE_DIR="$actual_dir" \
    GPUI_MESH_PLOT_PRODUCT_SOURCE_REVISION="$source_revision" \
    GPUI_MESH_PLOT_PRODUCT_SOURCE_DIRTY="$source_dirty" \
        "$test_binary" --exact \
        product_mesh_plot_axes_and_selection_render_through_metal_and_wgpu \
        --test-threads=1
    if [[ ! -f "$actual_dir/manifest.json" ]]; then
        write_skip_manifest "product capture test produced no adapter manifest"
    fi
fi

PYTHONPATH=scripts \
    QA_PRODUCT_REQUIRED="$required" \
    QA_PRODUCT_SOURCE_REVISION="$source_revision" \
    "$python_bin" - <<'PY'
import os
from pathlib import Path

from qa_release_evidence import validate_mesh_plot_product_visual

validate_mesh_plot_product_visual(
    Path.cwd(),
    require_capture=os.environ.get("QA_PRODUCT_REQUIRED") == "1",
    source_revision=os.environ["QA_PRODUCT_SOURCE_REVISION"],
)
print("high-level MeshPlot product visual evidence validated")
PY
