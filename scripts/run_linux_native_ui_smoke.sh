#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/debug/layout-showcase}"
artifact="${2:-target/qa/native-ui/linux/gpui-builder-smoke.json}"
screenshot="${3:-target/qa/native-ui/linux/gpui-builder.png}"
capture_transport="${4:-linux-x11-window}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

mkdir -p "$(dirname "$artifact")" "$(dirname "$screenshot")"

export GPUI_NATIVE_SMOKE_HOLD_MS="${GPUI_NATIVE_SMOKE_HOLD_MS:-65000}"

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
unique_colors=0
for _ in $(seq 1 13); do
    import -window root "$screenshot"
    unique_colors="$(identify -format '%k' "$screenshot")"
    if (( unique_colors >= 16 )); then
        break
    fi
    sleep 5
done
import -window root "$screenshot"

unique_colors="$(identify -format '%k' "$screenshot")"
if (( unique_colors < 16 )); then
    echo "Native UI screenshot is blank or near-uniform: ${unique_colors} colors" >&2
    exit 1
fi

wait "$app_pid"

python3 "$script_dir/qa_native_ui_evidence.py" \
    --artifact "$artifact" \
    --screenshot "$screenshot" \
    --platform linux \
    --unique-colors "$unique_colors" \
    --capture-transport "$capture_transport"

trap - EXIT
