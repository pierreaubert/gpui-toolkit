#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/debug/layout-showcase}"
artifact="${2:-target/qa/native-ui/linux/gpui-builder-smoke.json}"
screenshot="${3:-target/qa/native-ui/linux/gpui-builder.png}"

mkdir -p "$(dirname "$artifact")" "$(dirname "$screenshot")"

export GPUI_NATIVE_SMOKE_HOLD_MS="${GPUI_NATIVE_SMOKE_HOLD_MS:-5000}"

"$binary" \
    --smoke-test \
    --smoke-artifact "$artifact" &
app_pid=$!

cleanup() {
    kill "$app_pid" 2>/dev/null || true
}
trap cleanup EXIT

window_id=""
for _ in $(seq 1 40); do
    window_id="$(xdotool search --name "Layout Builder Showcase" 2>/dev/null | head -n 1 || true)"
    if [[ -n "$window_id" ]]; then
        break
    fi
    sleep 0.25
done

if [[ -z "$window_id" ]]; then
    echo "Layout Builder Showcase window was not discoverable in Xvfb" >&2
    exit 1
fi

# Let the smoke transition collapse the sidebar and paint the second frame.
sleep 1
import -window "$window_id" "$screenshot"

unique_colors="$(identify -format '%k' "$screenshot")"
if (( unique_colors < 16 )); then
    echo "Native UI screenshot is blank or near-uniform: ${unique_colors} colors" >&2
    exit 1
fi

wait "$app_pid"

python3 - "$artifact" "$screenshot" "$unique_colors" <<'PY'
import json
import pathlib
import sys

artifact = pathlib.Path(sys.argv[1])
screenshot = pathlib.Path(sys.argv[2])
report = json.loads(artifact.read_text(encoding="utf-8"))
report["pixel_capture"] = True
report["pixel_artifact"] = screenshot.name
report["pixel_unique_colors"] = int(sys.argv[3])
artifact.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
PY

trap - EXIT
