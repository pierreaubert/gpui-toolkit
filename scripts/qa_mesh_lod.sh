#!/usr/bin/env bash
# Capture adapter-backed MeshPlot drag-time LOD evidence.
# Developer runs record an explicit skip when no WGPU adapter exists; strict
# release callers require captured visual-quality and frame-budget evidence.
set -euo pipefail

cd "$(dirname "$0")/.."

output_dir="${QA_MESH_LOD_OUTPUT_DIR:-target/qa/perf/mesh-lod}"
required="${QA_LOD_REQUIRED:-0}"
manifest="$output_dir/mesh-lod-evidence.json"

cargo_bin="${MESH_PLOT_CARGO_BIN:-}"
if [[ -z "$cargo_bin" ]]; then
    cargo_bin="$(command -v cargo 2>/dev/null || true)"
fi
if [[ -z "$cargo_bin" && -x "/Users/pierre/.cargo/bin/cargo" ]]; then
    cargo_bin="/Users/pierre/.cargo/bin/cargo"
fi
python_bin="${MESH_PLOT_PYTHON_BIN:-}"
if [[ -z "$python_bin" ]]; then
    python_bin="$(command -v python3 2>/dev/null || true)"
fi
if [[ -z "$python_bin" && -x "/opt/homebrew/bin/python3" ]]; then
    python_bin="/opt/homebrew/bin/python3"
fi
if [[ -z "$python_bin" && -x "/usr/bin/python3" ]]; then
    python_bin="/usr/bin/python3"
fi
if [[ -z "$cargo_bin" || -z "$python_bin" ]]; then
    echo "MeshPlot LOD QA requires cargo and python3" >&2
    exit 2
fi

mkdir -p "$output_dir"
rm -f "$manifest"
source_revision="$(git rev-parse HEAD)"
source_dirty=false
if [[ -n "$(git status --porcelain=v1 --untracked-files=all)" ]]; then
    source_dirty=true
fi

write_skip_manifest() {
    local reason="$1"
    "$python_bin" - "$manifest" "$reason" "$source_revision" "$source_dirty" <<'PY'
import json
import sys
from pathlib import Path

path, reason, revision, dirty = sys.argv[1:]
Path(path).write_text(
    json.dumps(
        {
            "schema_version": 1,
            "report_type": "gpui-mesh-lod-evidence",
            "status": "skipped",
            "reason": reason,
            "source_revision": revision,
            "source_dirty": dirty == "true",
        },
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY
}

if [[ -z "${CLANG_MODULE_CACHE_PATH:-}" ]]; then
    export CLANG_MODULE_CACHE_PATH="${MESH_PLOT_CLANG_MODULE_CACHE_PATH:-${TMPDIR:-/tmp}/gpui-toolkit-clang-cache}"
fi

"$cargo_bin" test -p gpui-d3rs --test mesh_lod_evidence \
    --features gpu-3d --no-run
test_binary="$($python_bin - <<'PY'
from pathlib import Path

candidates = [
    path
    for path in Path("target/debug/deps").glob("mesh_lod_evidence-*")
    if path.is_file() and path.stat().st_mode & 0o111
]
if candidates:
    print(max(candidates, key=lambda path: path.stat().st_mtime_ns))
PY
)"
if [[ -z "$test_binary" ]]; then
    echo "could not locate the built MeshPlot LOD evidence test binary" >&2
    exit 1
fi

SOTF_MESH_LOD_EVIDENCE_DIR="$output_dir" \
SOTF_MESH_LOD_SOURCE_REVISION="$source_revision" \
SOTF_MESH_LOD_SOURCE_DIRTY="$source_dirty" \
QA_WGPU_REQUIRED="$required" \
    "$test_binary" --exact \
    adapter_lod_release_evidence_covers_drag_budget_and_visual_restore \
    --test-threads=1

if [[ ! -f "$manifest" ]]; then
    write_skip_manifest "LOD evidence test produced no manifest"
fi

PYTHONPATH=scripts \
    QA_LOD_REQUIRED="$required" \
    QA_LOD_SOURCE_REVISION="$source_revision" \
    "$python_bin" - <<'PY'
import os
from pathlib import Path

from qa_release_evidence import validate_mesh_lod_evidence

validate_mesh_lod_evidence(
    Path.cwd(),
    require_capture=os.environ.get("QA_LOD_REQUIRED") == "1",
    source_revision=os.environ["QA_LOD_SOURCE_REVISION"],
)
print("MeshPlot LOD evidence validated")
PY
