#!/usr/bin/env bash
# Run renderer-independent visual checks plus deterministic renderer-backed
# component snapshots. Supported renderer lanes must provide a versioned
# baseline archive; missing, blank, or wrong-sized pixels are hard failures.
set -euo pipefail

cd "$(dirname "$0")/.."

# macOS shader compilation commonly inherits a protected global Clang module
# cache in restricted developer shells. Allow an explicit caller override,
# otherwise keep this QA run's modules in a task-scoped writable directory.
if [[ "$(uname -s)" == "Darwin" && -z "${CLANG_MODULE_CACHE_PATH:-}" ]]; then
    CLANG_MODULE_CACHE_PATH="${GPUI_TOOLKIT_CLANG_MODULE_CACHE_PATH:-${TMPDIR:-/tmp}/gpui-toolkit-clang-cache}"
    export CLANG_MODULE_CACHE_PATH
fi

# A versioned baseline represents a source revision, not whichever partially
# edited files happened to be open on a developer machine. Keep capture output
# in target/ available for iteration, but refuse baseline promotion until the
# source tree (including untracked release inputs) is clean. This guard is
# deliberately before any build or capture work.
if [[ "${QA_VISUAL_UPDATE_BASELINES:-0}" == "1" ]]; then
    if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
        echo "QA_VISUAL_UPDATE_BASELINES=1 requires a clean source tree; commit or remove all tracked/untracked changes before promoting baselines." >&2
        exit 1
    fi
    baseline_revision="$(git rev-parse --verify HEAD)"
    echo "=== Promoting visual baselines from clean revision ${baseline_revision} ==="
fi

mkdir -p target/qa/visual target/gpui-conformance

# macOS can return a non-zero status while tearing down Text Input Services
# after the component-lab renderer has already written a complete capture.
# Accept that narrow case only when the JSON contract and every actual image
# prove that all requested cases completed; genuine partial/failed captures
# remain hard failures.
visual_capture_artifact_is_complete() {
    python3 - "$1" "$2" "$3" "$4" "$5" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
expected_type = sys.argv[2]
expected_count = int(sys.argv[3])
expected_root = Path(sys.argv[4]).resolve()
started_at = float(sys.argv[5])
try:
    if path.stat().st_mtime < started_at:
        raise SystemExit(1)
    data = json.loads(path.read_text(encoding="utf-8"))
except (OSError, UnicodeDecodeError, json.JSONDecodeError):
    raise SystemExit(1)
cases = data.get("cases")
complete = (
    data.get("report_type") == expected_type
    and data.get("passed") is True
    and data.get("requested_count") == expected_count
    and data.get("captured_count") == expected_count
    and data.get("failed_count") == 0
    and isinstance(cases, list)
    and len(cases) == expected_count
    and all(
        isinstance(case, dict)
        and case.get("status") == "Captured"
        and isinstance(case.get("actual_path"), str)
        and Path(case["actual_path"]).is_file()
        and Path(case["actual_path"]).resolve().is_relative_to(expected_root)
        for case in cases
    )
)
raise SystemExit(0 if complete else 1)
PY
}

visual_diff_artifact_is_complete() {
    python3 - "$1" "$2" "$3" "$4" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
expected_count = int(sys.argv[2])
expected_root = Path(sys.argv[3]).resolve()
started_at = float(sys.argv[4])
try:
    if path.stat().st_mtime < started_at:
        raise SystemExit(1)
    data = json.loads(path.read_text(encoding="utf-8"))
except (OSError, UnicodeDecodeError, json.JSONDecodeError):
    raise SystemExit(1)
cases = data.get("cases")
complete = (
    data.get("report_type") == "gpui-component-lab-visual-diff"
    and data.get("passed") is True
    and data.get("compared_count") == expected_count
    and data.get("failed_count") == 0
    and data.get("max_changed_pixels") == 0
    and isinstance(cases, list)
    and len(cases) == expected_count
    and all(
        isinstance(case, dict)
        and case.get("status") == "Passed"
        and case.get("changed_pixels") == 0
        and case.get("max_channel_delta") == 0
        and isinstance(case.get("actual_path"), str)
        and Path(case["actual_path"]).resolve().is_relative_to(expected_root)
        and Path(case["actual_path"]).is_file()
        for case in cases
    )
)
raise SystemExit(0 if complete else 1)
PY
}

FEATURES="--features autoeq,gpu-2d,gpu-3d,reqwest,showcase,spinorama,tokio,urlencoding"

echo "=== gpui-builder visual tests ==="
cargo test -p gpui-builder visual --quiet
echo "=== gpui-d3rs visual/GPU tests ==="
cargo test -p gpui-d3rs --features gpui,gpu-2d,gpu-3d --tests --quiet
echo "=== design-token conformance ==="
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens ${FEATURES} -- --report-json target/gpui-conformance/design-tokens.json --report-markdown target/gpui-conformance/design-tokens.md
echo "=== component-lab conformance ==="
cargo run -p gpui-component-lab --bin gpui-component-lab ${FEATURES} -- --conformance --report-json target/gpui-conformance/component-lab.json --report-markdown target/gpui-conformance/component-lab.md

echo "=== component-lab capture inventory ==="
visual_root="${QA_VISUAL_OUTPUT_ROOT:-target/qa/visual/component-lab}"
baseline_archive="${QA_VISUAL_BASELINE_ARCHIVE:-qa/visual/baselines/component-lab-metal-pr-v1.tar.zst}"

case "$(uname -s)" in
    Darwin)
        visual_renderer="metal"
        visual_scale="2"
        ;;
    Linux)
        visual_renderer="wgpu-linux"
        visual_scale="1"
        ;;
    *)
        visual_renderer="directx"
        visual_scale="1"
        ;;
esac
mkdir -p "$visual_root"
cargo run -p gpui-component-lab --bin gpui-component-lab ${FEATURES} -- \
    --visual-output-root "$visual_root" \
    --visual-renderer "$visual_renderer" \
    --visual-pixel-scale "$visual_scale" \
    --visual-manifest-json target/qa/visual/component-lab-manifest.json \
    --visual-manifest-markdown target/qa/visual/component-lab-manifest.md

if ! command -v jq >/dev/null 2>&1; then
    echo "MeshPlot visual QA requires jq to select the registered 99 MeshPlot cases" >&2
    exit 1
fi
mesh_plot_case_args=()
mesh_plot_case_count=0
while IFS= read -r capture_id; do
    [[ -n "$capture_id" ]] || continue
    mesh_plot_case_args+=(--visual-case "$capture_id")
    mesh_plot_case_count=$((mesh_plot_case_count + 1))
done < <(jq -r '.cases[] | select(.story_id | startswith("px.mesh_plot")) | .capture_id' \
    target/qa/visual/component-lab-manifest.json)
if [[ "$mesh_plot_case_count" -ne 99 ]]; then
    echo "component-lab manifest must expose exactly 99 MeshPlot cases; found $mesh_plot_case_count" >&2
    exit 1
fi

component_capture_status="not supported on $(uname -s)"
visual_diff_status="not run"
if [[ "$(uname -s)" == "Darwin" ]]; then
    if [[ -f "$baseline_archive" && "${QA_VISUAL_UPDATE_BASELINES:-0}" != "1" ]]; then
        tar -xf "$baseline_archive" -C "$visual_root"
    fi
    capture_args=(
        --visual-capture
        --visual-capture-limit 0
        --visual-gallery
        --visual-output-root "$visual_root"
        --visual-renderer "$visual_renderer"
        --visual-pixel-scale "$visual_scale"
        --visual-capture-json target/qa/visual/component-lab-capture.json
        --visual-capture-markdown target/qa/visual/component-lab-capture.md
        "${mesh_plot_case_args[@]}"
    )
    if [[ "${QA_VISUAL_UPDATE_BASELINES:-0}" == "1" ]]; then
        capture_args+=(--visual-update-baselines)
    fi
    capture_started_at="$(date +%s)"
    if ! cargo run -p gpui-component-lab --bin gpui-component-lab \
        --features autoeq,gpu-2d,gpu-3d,reqwest,showcase,spinorama,tokio,urlencoding,visual-capture \
        -- "${capture_args[@]}"; then
        if visual_capture_artifact_is_complete \
            target/qa/visual/component-lab-capture.json \
            gpui-component-lab-render-capture 99 \
            "$visual_root" "$capture_started_at"; then
            echo "renderer teardown returned non-zero after a complete 99-case capture; accepting validated artifact"
        else
            exit 1
        fi
    fi
    component_capture_status="passed (99 MeshPlot Metal captures)"

    if ! find "$visual_root/$visual_renderer/baseline" -type f -name '*.png' -print -quit 2>/dev/null | grep -q .; then
        echo "renderer baseline set is missing: $baseline_archive" >&2
        echo "run with QA_VISUAL_UPDATE_BASELINES=1 to approve local captures, then package the baseline directory" >&2
        exit 1
    fi
    echo "=== component-lab pixel diff ==="
    diff_started_at="$(date +%s)"
    if ! cargo run -p gpui-component-lab --bin gpui-component-lab ${FEATURES} -- \
        --visual-output-root "$visual_root" \
        --visual-renderer "$visual_renderer" \
        --visual-pixel-scale "$visual_scale" \
        --visual-diff \
        --visual-diff-limit 0 \
        --visual-diff-json target/qa/visual/component-lab-diff.json \
        --visual-diff-markdown target/qa/visual/component-lab-diff.md \
        "${mesh_plot_case_args[@]}"; then
        if visual_diff_artifact_is_complete target/qa/visual/component-lab-diff.json 99 \
            "$visual_root" "$diff_started_at"; then
            echo "renderer teardown returned non-zero after a complete zero-diff report; accepting validated artifact"
        else
            exit 1
        fi
    fi
    visual_diff_status="passed (99 MeshPlot renderer comparisons)"
fi

echo "=== showcase capture inventory ==="
cargo run -p gpui-showcase --bin gpui-showcase -- --visual-manifest --json > target/qa/visual/showcase-manifest.json
cargo run -p gpui-showcase --bin gpui-showcase -- --visual-manifest > target/qa/visual/showcase-manifest.md

native_capture_status="not-run (native capture tools unavailable or not requested)"
native_capture_mode="${QA_NATIVE_UI_CAPTURE:-auto}"
native_tools_available=true
for command in xvfb-run xdotool import identify; do
    if ! command -v "$command" >/dev/null 2>&1; then
        native_tools_available=false
    fi
done
if [[ "$native_capture_mode" == "1" \
    || ( "$native_capture_mode" == "auto" && "$(uname -s)" == "Linux" \
        && "$native_tools_available" == "true" ) ]]; then
    for command in xvfb-run xdotool import identify; do
        if ! command -v "$command" >/dev/null 2>&1; then
            echo "native visual capture requested but required command is missing: $command" >&2
            exit 1
        fi
    done
    echo "=== Linux native screenshot capture ==="
    cargo build -p gpui-builder --features showcase --bin layout-showcase
    xvfb-run -a env LIBGL_ALWAYS_SOFTWARE=1 bash scripts/run_linux_native_ui_smoke.sh \
        target/debug/layout-showcase \
        target/qa/visual/native-ui/gpui-builder-smoke.json \
        target/qa/visual/native-ui/gpui-builder.png \
        linux-x11-window
    native_capture_status="passed (Linux native smoke screenshot)"
elif [[ "$native_capture_mode" == "1" ]]; then
    echo "native visual capture requested but the Linux capture environment is unavailable" >&2
    exit 1
fi

cat > target/qa/visual/report.md <<EOF
# Visual QA execution report

- Renderer-independent golden/GPU tests: passed
- Design-token conformance: passed
- Component-lab conformance: passed
- Component-lab capture manifest: generated (${visual_renderer}, ${visual_scale}x)
- Component-lab renderer capture: ${component_capture_status}
- Component-lab pixel diff: ${visual_diff_status}
- Component-lab contact-sheet gallery: generated on supported renderer lanes
- Showcase capture inventory: generated
- Native screenshot capture: ${native_capture_status}

The component-lab manifest defines renderer-specific baseline, actual, and diff
paths. Supported capture lanes fail on missing baselines, blank images,
dimension drift, render/write failures, or pixel differences above policy.
EOF

echo "Visual QA checks passed; component-lab diff status: ${visual_diff_status}."
