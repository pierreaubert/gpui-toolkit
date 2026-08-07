#!/usr/bin/env bash
# Run renderer-independent visual checks plus deterministic renderer-backed
# component snapshots. Supported renderer lanes must provide a versioned
# baseline archive; missing, blank, or wrong-sized pixels are hard failures.
set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p target/qa/visual target/gpui-conformance

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
capture_limit="${QA_VISUAL_CAPTURE_LIMIT:-200}"
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

component_capture_status="not supported on $(uname -s)"
visual_diff_status="not run"
if [[ "$(uname -s)" == "Darwin" ]]; then
    if [[ -f "$baseline_archive" && "${QA_VISUAL_UPDATE_BASELINES:-0}" != "1" ]]; then
        tar -xf "$baseline_archive" -C "$visual_root"
    fi
    capture_args=(
        --visual-capture
        --visual-capture-limit "$capture_limit"
        --visual-gallery
        --visual-output-root "$visual_root"
        --visual-renderer "$visual_renderer"
        --visual-pixel-scale "$visual_scale"
        --visual-capture-json target/qa/visual/component-lab-capture.json
        --visual-capture-markdown target/qa/visual/component-lab-capture.md
    )
    if [[ "${QA_VISUAL_UPDATE_BASELINES:-0}" == "1" ]]; then
        capture_args+=(--visual-update-baselines)
    fi
    cargo run -p gpui-component-lab --bin gpui-component-lab \
        --features autoeq,gpu-2d,gpu-3d,reqwest,showcase,spinorama,tokio,urlencoding,visual-capture \
        -- "${capture_args[@]}"
    component_capture_status="passed (${capture_limit} representative Metal captures)"

    if ! find "$visual_root/$visual_renderer/baseline" -type f -name '*.png' -print -quit 2>/dev/null | grep -q .; then
        echo "renderer baseline set is missing: $baseline_archive" >&2
        echo "run with QA_VISUAL_UPDATE_BASELINES=1 to approve local captures, then package the baseline directory" >&2
        exit 1
    fi
    echo "=== component-lab pixel diff ==="
    cargo run -p gpui-component-lab --bin gpui-component-lab ${FEATURES} -- \
        --visual-output-root "$visual_root" \
        --visual-renderer "$visual_renderer" \
        --visual-pixel-scale "$visual_scale" \
        --visual-diff \
        --visual-diff-limit "$capture_limit" \
        --visual-diff-json target/qa/visual/component-lab-diff.json \
        --visual-diff-markdown target/qa/visual/component-lab-diff.md
    visual_diff_status="passed (${capture_limit} renderer comparisons)"
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
