#!/usr/bin/env bash
# Run property-based tests across the workspace.
# If no proptest tests exist, cargo exits 0 with zero matched tests.
set -euo pipefail

cd "$(dirname "$0")/.."

FEATURES="--features autoeq,camera,gpu-2d,gpu-3d,reqwest,showcase,spinorama,tokio,urlencoding"

cargo test --workspace --all-targets ${FEATURES} proptest --quiet -- --nocapture
