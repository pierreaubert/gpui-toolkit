#!/usr/bin/env bash
# Run explicit property-test targets so an accidentally removed suite fails
# instead of cargo silently succeeding with zero matched tests.
set -euo pipefail

cd "$(dirname "$0")/.."

cargo test -p gpui-builder --test proptests --quiet -- --nocapture
