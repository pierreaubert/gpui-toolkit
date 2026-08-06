#!/usr/bin/env bash
# Run visual checks, generate the component-lab capture inventory, and run the
# existing pixel diff whenever a complete baseline/actual set is available.
# The manifest itself is not a screenshot; the report remains explicit about
# that distinction when renderer capture or baselines are unavailable.
set -euo pipefail

cd "$(dirname "$0")/.."

mkdir -p target/qa/visual target/gpui-conformance

FEATURES="--features autoeq,camera,gpu-2d,gpu-3d,reqwest,showcase,spinorama,tokio,urlencoding"

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
mkdir -p "$visual_root"
cargo run -p gpui-component-lab --bin gpui-component-lab ${FEATURES} -- \
    --visual-output-root "$visual_root" \
    --visual-manifest-json target/qa/visual/component-lab-manifest.json \
    --visual-manifest-markdown target/qa/visual/component-lab-manifest.md

visual_diff_status="pending (no complete baseline/actual set)"
if find "$visual_root/baseline" -type f -name '*.png' -print -quit 2>/dev/null | grep -q . \
    && find "$visual_root/actual" -type f -name '*.png' -print -quit 2>/dev/null | grep -q .; then
    echo "=== component-lab pixel diff ==="
    cargo run -p gpui-component-lab --bin gpui-component-lab ${FEATURES} -- \
        --visual-output-root "$visual_root" \
        --visual-diff-json target/qa/visual/component-lab-diff.json \
        --visual-diff-markdown target/qa/visual/component-lab-diff.md
    visual_diff_status="passed"
else
    echo "Component-lab pixel diff pending: supply matching baseline and actual PNG captures under $visual_root."
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
- Component-lab capture manifest: generated
- Component-lab pixel diff: ${visual_diff_status}
- Showcase capture inventory: generated
- Native screenshot capture: ${native_capture_status}

The component-lab manifest defines required baseline, actual, and diff paths.
Pixel diff is a strict gate when a complete image set is present. Without
matching renderer captures and baselines, this report remains pending and does
not claim visual-regression coverage.
EOF

echo "Visual QA checks passed; component-lab diff status: ${visual_diff_status}."
