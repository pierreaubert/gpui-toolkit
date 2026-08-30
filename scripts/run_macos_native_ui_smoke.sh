#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "macOS native UI capture must run directly on macOS" >&2
    exit 1
fi

binary="${1:-target/debug/layout-showcase}"
artifact="${2:-target/qa/native-ui/macos/gpui-builder-smoke.json}"
screenshot="${3:-target/qa/native-ui/macos/gpui-builder.png}"
window_title="${GPUI_NATIVE_SMOKE_WINDOW_TITLE:-Layout Builder Showcase}"
window_owner="${GPUI_NATIVE_SMOKE_WINDOW_OWNER:-layout-showcase}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

for command in swift screencapture magick python3; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "required macOS capture command is missing: $command" >&2
        exit 1
    fi
done
if [[ ! -x "$binary" ]]; then
    echo "layout showcase binary is missing or not executable: $binary" >&2
    exit 1
fi

mkdir -p "$(dirname "$artifact")" "$(dirname "$screenshot")"
export GPUI_NATIVE_SMOKE_HOLD_MS="${GPUI_NATIVE_SMOKE_HOLD_MS:-8000}"
capture_pixels="${GPUI_NATIVE_SMOKE_CAPTURE_PIXELS:-1}"

"$binary" --smoke-test --smoke-artifact "$artifact" &
app_pid=$!

cleanup() {
    kill "$app_pid" 2>/dev/null || true
}
trap cleanup EXIT

if [[ "$capture_pixels" != "0" ]]; then
window_id="$(swift -e '
import CoreGraphics
import Darwin
import Foundation

let title = CommandLine.arguments[1]
let owner = CommandLine.arguments[2]
for _ in 0..<100 {
    let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID)
        as? [[String: Any]] ?? []
    if let window = windows.first(where: {
        ($0[kCGWindowName as String] as? String) == title
            || (($0[kCGWindowOwnerName as String] as? String) == owner
                && (($0[kCGWindowLayer as String] as? Int) ?? -1) == 0)
    }), let number = window[kCGWindowNumber as String] as? Int {
        print(number)
        exit(0)
    }
    Thread.sleep(forTimeInterval: 0.1)
}
fputs("native GPUI window was not discoverable: \(title)\n", stderr)
exit(1)
' "$window_title" "$window_owner")"

# The smoke transition occurs after the first paint. Capture after it has had
# enough time to produce and present the verified second frame.
sleep 1
if ! screencapture -x -l "$window_id" "$screenshot"; then
    echo "macOS window capture failed; grant Screen Recording permission to the terminal" >&2
    exit 1
fi

unique_colors="$(magick identify -format '%k' "$screenshot")"
wait "$app_pid"

python3 "$script_dir/qa_native_ui_evidence.py" \
  --artifact "$artifact" \
    --screenshot "$screenshot" \
    --platform macos \
  --unique-colors "$unique_colors" \
  --capture-transport macos-window
else
  # GitHub-hosted macOS runners have no Screen Recording permission. The
  # showcase still opens, renders and verifies its interaction contract; the
  # pixel capture remains required on an interactive desktop runner.
  wait "$app_pid"
  python3 "$script_dir/qa_native_ui_evidence.py" \
    --artifact "$artifact" \
    --platform macos \
    --smoke-only
fi

trap - EXIT
