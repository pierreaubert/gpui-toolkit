#!/usr/bin/env bash
# Compile every checked-in cargo-fuzz target and replay its seed corpus
# under libFuzzer for a bounded time. QA_FUZZ_SECONDS bounds each target
# (default 10s); CI sets QA_REQUIRE_FUZZ=1 so a missing cargo-fuzz fails
# instead of silently skipping.
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo-fuzz >/dev/null 2>&1; then
    if [[ "${QA_REQUIRE_FUZZ:-0}" == "1" ]]; then
        echo "cargo-fuzz is required but is not installed" >&2
        exit 1
    fi
    echo "cargo-fuzz not installed; fuzz target compile check skipped"
    exit 0
fi

seconds="${QA_FUZZ_SECONDS:-10}"
for target in pretext-layout vtu-parse vtk-parse; do
    cargo fuzz check "$target"
    if [ -d "fuzz/corpus/$target" ] && [ -n "$(ls -A "fuzz/corpus/$target")" ]; then
        cargo fuzz run "$target" "fuzz/corpus/$target" -- -max_total_time="$seconds"
    else
        cargo fuzz run "$target" -- -max_total_time="$seconds"
    fi
done
