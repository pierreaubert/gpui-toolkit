#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "Apple simulator smoke tests must run directly on macOS" >&2
    exit 1
fi
if [[ $# -lt 5 || $# -gt 6 ]]; then
    echo "usage: $0 <ios|tvos> <app-path> <bundle-id> <artifact.json> <screenshot.png> [device-udid]" >&2
    exit 2
fi

platform="$1"
app_path="$2"
bundle_id="$3"
artifact="$4"
screenshot="$5"
requested_udid="${6:-}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ "$platform" != "ios" && "$platform" != "tvos" ]]; then
    echo "platform must be ios or tvos" >&2
    exit 2
fi
if [[ ! -d "$app_path" ]]; then
    echo "simulator app bundle is missing: $app_path" >&2
    exit 1
fi
for command in git magick python3 rustc xcodebuild xcrun; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "required simulator evidence command is missing: $command" >&2
        exit 1
    fi
done

device_record="$(python3 - "$platform" "$requested_udid" <<'PY'
import json
import subprocess
import sys

platform, requested = sys.argv[1:]
payload = json.loads(subprocess.check_output(
    ["xcrun", "simctl", "list", "devices", "available", "-j"], text=True
))
needle = ".iOS-" if platform == "ios" else ".tvOS-"
candidates = []
for runtime, devices in payload["devices"].items():
    if needle not in runtime:
        continue
    for device in devices:
        if requested and device["udid"] != requested:
            continue
        candidates.append((runtime, device))
if not candidates:
    qualifier = f" with UDID {requested}" if requested else ""
    raise SystemExit(f"no available {platform} simulator{qualifier}")
runtime, device = candidates[-1]
runtime_name = runtime.rsplit(".", 1)[-1].replace("-", " ")
print("|".join((device["udid"], device["name"], runtime_name, device["state"])))
PY
)"
IFS='|' read -r device_udid device_name runtime_name original_state <<<"$device_record"

temporary_dir="$(mktemp -d "${TMPDIR:-/private/tmp}/gpui-apple-sim.XXXXXX")"
temporary_screenshot="$temporary_dir/$(basename "$screenshot")"
booted_by_script=false
launched=false

cleanup() {
    if [[ "$launched" == true ]]; then
        xcrun simctl terminate "$device_udid" "$bundle_id" >/dev/null 2>&1 || true
    fi
    if [[ "$booted_by_script" == true ]]; then
        xcrun simctl shutdown "$device_udid" >/dev/null 2>&1 || true
    fi
    rm -rf "$temporary_dir"
}
trap cleanup EXIT

if [[ "$original_state" != "Booted" ]]; then
    xcrun simctl boot "$device_udid"
    booted_by_script=true
fi
xcrun simctl bootstatus "$device_udid" -b
xcrun simctl install "$device_udid" "$app_path"
launch_output="$(xcrun simctl launch "$device_udid" "$bundle_id")"
launch_pid="${launch_output##*: }"
if [[ ! "$launch_pid" =~ ^[0-9]+$ ]]; then
    echo "simctl did not return a launch PID: $launch_output" >&2
    exit 1
fi
launched=true
sleep 2
xcrun simctl io "$device_udid" screenshot "$temporary_screenshot"

mkdir -p "$(dirname "$artifact")" "$(dirname "$screenshot")"
cp "$temporary_screenshot" "$screenshot"
unique_colors="$(magick identify -format '%k' "$screenshot")"
pixel_width="$(magick identify -format '%w' "$screenshot")"
pixel_height="$(magick identify -format '%h' "$screenshot")"
source_revision="$(git rev-parse HEAD)"
if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
    source_dirty_flag="--source-dirty"
else
    source_dirty_flag="--no-source-dirty"
fi
xcode_version="$(xcodebuild -version | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
rustc_version="$(rustc -Vv | tr '\n' ' ' | sed 's/[[:space:]]*$//')"

python3 "$script_dir/qa_apple_simulator_evidence.py" \
    --artifact "$artifact" \
    --screenshot "$screenshot" \
    --platform "$platform" \
    --device-name "$device_name" \
    --runtime "$runtime_name" \
    --device-udid "$device_udid" \
    --bundle-id "$bundle_id" \
    --launch-pid "$launch_pid" \
    --unique-colors "$unique_colors" \
    --pixel-width "$pixel_width" \
    --pixel-height "$pixel_height" \
    --source-revision "$source_revision" \
    "$source_dirty_flag" \
    --xcode "$xcode_version" \
    --rustc "$rustc_version"

echo "Apple simulator evidence: $artifact $screenshot"
