#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
OUT_DIR="${ROOT}/target/gpui-ios-hot-reload"
GENERATION_FILE="${OUT_DIR}/generation"
MANIFEST="${OUT_DIR}/showcase.reload"
DYLIB="${OUT_DIR}/libshowcase_ios_reload.dylib"

mkdir -p "${OUT_DIR}"
generation=1
if [[ -f "${GENERATION_FILE}" ]]; then
  generation="$(( $(cat "${GENERATION_FILE}") + 1 ))"
fi
echo "${generation}" > "${GENERATION_FILE}"

cargo build \
  -p gpui-showcase-ios \
  --target aarch64-apple-ios-sim \
  --features hot-reload

cp "${ROOT}/target/aarch64-apple-ios-sim/debug/libshowcase_ios.dylib" "${DYLIB}"
{
  echo "dylib_path=${DYLIB}"
  echo "entry_symbol=showcase_ios_start"
  echo "generation=${generation}"
} > "${MANIFEST}"

echo "Wrote ${MANIFEST}"
