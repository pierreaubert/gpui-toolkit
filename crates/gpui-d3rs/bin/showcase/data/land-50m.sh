#!/usr/bin/env bash
set -euo pipefail

readonly expected_sha256="619477ff690c086885e45cb91707d783805561bd75ae8e437b7d4694b0204e0f"
readonly output="land-50m.json"
readonly temporary="${output}.download"

curl --fail --location --silent --show-error \
  https://cdn.jsdelivr.net/npm/world-atlas@2/land-50m.json \
  --output "${temporary}"
echo "${expected_sha256}  ${temporary}" | shasum -a 256 --check
mv "${temporary}" "${output}"
