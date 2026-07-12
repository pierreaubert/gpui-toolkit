#!/usr/bin/env bash
# Run renderer-independent visual checks and emit the capture inventory consumed
# by renderer-specific CI/device jobs. This script does not claim that a
# manifest is a screenshot; the generated report is explicit about that split.
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

echo "=== showcase capture inventory ==="
cargo run -p gpui-showcase --bin gpui-showcase -- --visual-manifest --json > target/qa/visual/showcase-manifest.json
cargo run -p gpui-showcase --bin gpui-showcase -- --visual-manifest > target/qa/visual/showcase-manifest.md

cat > target/qa/visual/report.md <<'EOF'
# Visual QA execution report

- Renderer-independent golden/GPU tests: passed
- Design-token conformance: passed
- Component-lab conformance: passed
- Showcase capture inventory: generated
- Renderer screenshot capture/diff: pending platform jobs

The manifest defines required baseline, actual, and diff paths. It is not
evidence that those images were rendered. A release remains pending until the
platform jobs attach real captures and diffs with OS, renderer, scale factor,
font set, viewport, and color scheme metadata.
EOF

echo "Visual logic checks passed; renderer capture remains a separate release gate."
