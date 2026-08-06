#!/usr/bin/env bash
# Compile the checked-in cargo-fuzz target. Running fuzzing itself remains an
# explicit, time-bounded developer/release action (for example, `cargo fuzz
# run pretext-layout -- -max_total_time=60`).
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

cargo fuzz check pretext-layout
