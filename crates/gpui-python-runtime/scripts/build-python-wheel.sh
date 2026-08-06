#!/usr/bin/env bash
# Stage the native host inside the Python package before building a wheel.
# CI/release jobs invoke this from the workspace checkout; end users never
# need Cargo because App.run resolves this bundled executable at runtime.
set -euo pipefail

crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
workspace_dir="$(cd "$crate_dir/../.." && pwd)"
host_name="gpui-python-host"
if [[ "${OS:-}" == "Windows_NT" ]]; then host_name+=".exe"; fi

cd "$workspace_dir"
cargo build --release -p gpui-python-runtime --features showcase --bin gpui-python-host

install_dir="$crate_dir/python/gpui_toolkit/bin"
mkdir -p "$install_dir"
install -m 755 "$workspace_dir/target/release/$host_name" "$install_dir/$host_name"

cd "$crate_dir"
python3 -m pip wheel --no-deps --no-build-isolation --wheel-dir dist .
