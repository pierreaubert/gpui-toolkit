#!/usr/bin/env bash
# Run visual/non-regression checks that don't require pixel comparisons yet.
# Phase 0: manifest checks + existing "visual" tests + conformance runners.
# Phase 1+: add pixel capture/diff against qa/visual/golden/.
set -euo pipefail

cd "$(dirname "$0")/.."

FEATURES="--features autoeq,camera,gpu-2d,gpu-3d,reqwest,showcase,spinorama,tokio,urlencoding"

echo "=== gpui-builder visual tests ==="
cargo test -p gpui-builder visual --quiet

echo "=== gpui-d3rs visual/GPU tests ==="
cargo test -p gpui-d3rs --features gpui,gpu-2d,gpu-3d --tests --quiet

echo "=== design-token conformance ==="
cargo run -p gpui-design-tools --bin gpui-validate-design-tokens ${FEATURES} -- --report-json target/gpui-conformance/design-tokens.json --report-markdown target/gpui-conformance/design-tokens.md

echo "=== component-lab conformance ==="
cargo run -p gpui-component-lab --bin gpui-component-lab ${FEATURES} -- --conformance --report-json target/gpui-conformance/component-lab.json --report-markdown target/gpui-conformance/component-lab.md

echo "=== visual regression harness (stub) ==="
cargo build -p gpui-ui-kit --bin qa_visual ${FEATURES}

echo "Visual checks passed."
