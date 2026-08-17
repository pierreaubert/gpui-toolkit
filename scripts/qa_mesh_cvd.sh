#!/usr/bin/env bash
# Run the dependency-free rendered-stimulus CVD screen after product capture.
set -euo pipefail
cd "$(dirname "$0")/.."

required="${QA_CVD_REQUIRED:-0}"
python_bin="${MESH_PLOT_PYTHON_BIN:-}"
if [[ -z "$python_bin" ]]; then
  python_bin="$(command -v python3 2>/dev/null || true)"
fi
if [[ -z "$python_bin" && -x /opt/homebrew/bin/python3 ]]; then
  python_bin=/opt/homebrew/bin/python3
fi
if [[ -z "$python_bin" ]]; then
  echo "could not locate Python 3 for MeshPlot CVD QA" >&2
  exit 1
fi

source_revision="$(git rev-parse HEAD)"
source_dirty="$(git status --short --untracked-files=all | sed '/^[[:space:]]*$/d' | wc -l | tr -d ' ')"
if [[ "$source_dirty" -gt 0 ]]; then
  source_dirty=1
else
  source_dirty=0
fi

if [[ "$required" == "1" ]]; then
  QA_CVD_SOURCE_REVISION="$source_revision" \
  QA_CVD_SOURCE_DIRTY="$source_dirty" \
  PYTHONPATH=scripts "$python_bin" scripts/qa_mesh_cvd.py \
    --required --output target/qa/visual/mesh-plot-cvd.json
else
  QA_CVD_SOURCE_REVISION="$source_revision" \
  QA_CVD_SOURCE_DIRTY="$source_dirty" \
  PYTHONPATH=scripts "$python_bin" scripts/qa_mesh_cvd.py \
    --output target/qa/visual/mesh-plot-cvd.json
fi

PYTHONPATH=scripts QA_CVD_REQUIRED="$required" \
  QA_CVD_SOURCE_REVISION="$source_revision" \
  "$python_bin" - <<'PY'
import os
from pathlib import Path

from qa_release_evidence import validate_mesh_plot_cvd

validate_mesh_plot_cvd(
    Path.cwd(),
    require_capture=os.environ.get("QA_CVD_REQUIRED") == "1",
    source_revision=os.environ["QA_CVD_SOURCE_REVISION"],
)
print("MeshPlot CVD screen validated")
PY
