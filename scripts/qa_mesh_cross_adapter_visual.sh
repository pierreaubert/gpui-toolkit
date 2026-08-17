#!/usr/bin/env bash
# Compare paired canonical Metal/WGPU MeshPlot captures.
#
# This wrapper does not manufacture reference evidence.  It consumes two
# already-captured six-case manifests, writes the deterministic comparator
# report, and validates the report with the release-evidence contract.  The
# ordinary developer lane skips when the pair is unavailable; release
# callers set QA_CROSS_ADAPTER_REQUIRED=1 and get a hard failure instead.
set -euo pipefail

cd "$(dirname "$0")/.."

metal_manifest="${QA_MESH_METAL_MANIFEST:-target/qa/visual/mesh-plot-metal/actual/manifest.json}"
wgpu_manifest="${QA_MESH_WGPU_MANIFEST:-target/qa/visual/mesh-plot-wgpu/actual/manifest.json}"
report_path="${QA_MESH_CROSS_ADAPTER_REPORT:-target/qa/visual/mesh-plot-cross-adapter.json}"
required="${QA_CROSS_ADAPTER_REQUIRED:-0}"
max_channel_delta="${QA_CROSS_ADAPTER_MAX_CHANNEL_DELTA:-0}"
max_changed_fraction="${QA_CROSS_ADAPTER_MAX_CHANGED_FRACTION:-0}"

python_bin="${MESH_PLOT_PYTHON_BIN:-}"
if [[ -z "$python_bin" ]]; then
    python_bin="$(command -v python3 2>/dev/null || true)"
fi
if [[ -z "$python_bin" && -x /opt/homebrew/bin/python3 ]]; then
    python_bin=/opt/homebrew/bin/python3
fi
if [[ -z "$python_bin" ]]; then
    echo "MeshPlot cross-adapter QA requires python3" >&2
    exit 2
fi

# This wrapper owns only the comparison report. Clear it before checking for
# a pair so an unavailable-adapter developer run cannot reuse stale evidence.
rm -f "$report_path"

manifest_status() {
    "$python_bin" - "$1" <<'PY'
import json
import sys
from pathlib import Path

try:
    value = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
except (OSError, UnicodeDecodeError, json.JSONDecodeError):
    print("invalid")
    raise SystemExit(0)
if not isinstance(value, dict):
    print("invalid")
else:
    print(value.get("status", "captured"))
PY
}

metal_exists=false
wgpu_exists=false
[[ -f "$metal_manifest" ]] && metal_exists=true
[[ -f "$wgpu_manifest" ]] && wgpu_exists=true

if [[ "$metal_exists" != true || "$wgpu_exists" != true ]]; then
    if [[ "$required" == "1" ]]; then
        echo "MeshPlot cross-adapter QA requires both capture manifests:" >&2
        echo "  Metal: $metal_manifest" >&2
        echo "  WGPU:  $wgpu_manifest" >&2
        exit 1
    fi
    echo "MeshPlot cross-adapter QA skipped: paired manifests are unavailable"
    exit 0
fi

metal_status="$(manifest_status "$metal_manifest")"
wgpu_status="$(manifest_status "$wgpu_manifest")"
if [[ "$metal_status" == "invalid" || "$wgpu_status" == "invalid" ]]; then
    echo "MeshPlot cross-adapter QA failed: capture manifest is not valid JSON" >&2
    exit 1
fi
if [[ "$metal_status" != "captured" && "$metal_status" != "skipped" ]] \
    || [[ "$wgpu_status" != "captured" && "$wgpu_status" != "skipped" ]]; then
    echo "MeshPlot cross-adapter QA failed: capture manifest has an unknown status" >&2
    exit 1
fi
if [[ "$metal_status" != "captured" || "$wgpu_status" != "captured" ]]; then
    if [[ "$required" == "1" ]]; then
        echo "MeshPlot cross-adapter QA requires captured Metal and WGPU manifests" >&2
        exit 1
    fi
    echo "MeshPlot cross-adapter QA skipped: one or both adapter captures were skipped"
    exit 0
fi

PYTHONPATH=scripts "$python_bin" scripts/mesh_plot_visual_compare.py \
    --left "$metal_manifest" \
    --right "$wgpu_manifest" \
    --repo-root "$PWD" \
    --max-channel-delta "$max_channel_delta" \
    --max-changed-fraction "$max_changed_fraction" \
    --output-report "$report_path"

PYTHONPATH=scripts "$python_bin" - "$report_path" <<'PY'
import sys
from pathlib import Path

from qa_release_evidence import validate_mesh_plot_cross_adapter_visual

validate_mesh_plot_cross_adapter_visual(
    Path.cwd(),
    require_report=True,
    report_path=Path(sys.argv[1]).resolve(),
)
PY

echo "MeshPlot cross-adapter release report validated: $report_path"
