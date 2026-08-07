#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 7 || $# -gt 8 ]]; then
    echo "usage: $0 <apk> <package> <activity> <artifact.json> <before.png> <after.png> <accessibility.xml> [serial]" >&2
    exit 2
fi

apk="$1"
package="$2"
activity="$3"
artifact="$4"
before="$5"
after="$6"
accessibility="$7"
requested_serial="${8:-}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
adb="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-/opt/homebrew/share/android-commandlinetools}}/platform-tools/adb"

if [[ ! -f "$apk" ]]; then
    echo "Android APK is missing: $apk" >&2
    exit 1
fi
for command in git magick python3 rustc java; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "required Android evidence command is missing: $command" >&2
        exit 1
    fi
done
if [[ ! -x "$adb" ]]; then
    echo "adb is missing or not executable: $adb" >&2
    exit 1
fi

if [[ -n "$requested_serial" ]]; then
    serial="$requested_serial"
else
    device_list="$("$adb" devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
    device_count="$(awk 'NF { count++ } END { print count + 0 }' <<<"$device_list")"
    if [[ "$device_count" -ne 1 ]]; then
        echo "expected exactly one connected Android device; pass an explicit serial" >&2
        exit 1
    fi
    serial="$device_list"
fi

temporary_dir="$(mktemp -d "${TMPDIR:-/private/tmp}/gpui-android.XXXXXX")"
temporary_before="$temporary_dir/before.png"
temporary_after="$temporary_dir/after.png"
temporary_xml="$temporary_dir/accessibility.xml"
cleanup() {
    "$adb" -s "$serial" shell am force-stop "$package" >/dev/null 2>&1 || true
    rm -rf "$temporary_dir"
}
trap cleanup EXIT

"$adb" -s "$serial" install -r "$apk" >/dev/null
"$adb" -s "$serial" shell am force-stop "$package"
launch_output="$("$adb" -s "$serial" shell am start -W "$package/$activity")"
launch_time_ms="$(awk -F: '/TotalTime:/ { gsub(/ /, "", $2); print $2; exit }' <<<"$launch_output")"
launch_pid="$("$adb" -s "$serial" shell pidof "$package" | tr -d '\r')"
if [[ ! "$launch_pid" =~ ^[0-9]+$ || ! "$launch_time_ms" =~ ^[0-9]+$ ]]; then
    echo "Android launch did not return a valid PID/time: $launch_output" >&2
    exit 1
fi

sleep 2
"$adb" -s "$serial" exec-out screencap -p >"$temporary_before"
pixel_width="$(magick identify -format '%w' "$temporary_before")"
pixel_height="$(magick identify -format '%h' "$temporary_before")"
tap_x="$((pixel_width * 28 / 100))"
tap_y="$((pixel_height * 5 / 200))"
"$adb" -s "$serial" shell input tap "$tap_x" "$tap_y"
sleep 1
"$adb" -s "$serial" exec-out screencap -p >"$temporary_after"

# The first pass warms Android's virtual-node IDs; the second is the retained tree.
"$adb" -s "$serial" shell uiautomator dump /sdcard/gpui-accessibility.xml >/dev/null
"$adb" -s "$serial" shell uiautomator dump /sdcard/gpui-accessibility.xml >/dev/null
"$adb" -s "$serial" exec-out cat /sdcard/gpui-accessibility.xml >"$temporary_xml"

mkdir -p "$(dirname "$artifact")" "$(dirname "$before")" \
    "$(dirname "$after")" "$(dirname "$accessibility")"
cp "$temporary_before" "$before"
cp "$temporary_after" "$after"
cp "$temporary_xml" "$accessibility"

before_unique_colors="$(magick identify -format '%k' "$before")"
after_unique_colors="$(magick identify -format '%k' "$after")"
read -r accessibility_node_count accessible_named_node_count <<<"$(python3 - "$accessibility" <<'PY'
import sys
import xml.etree.ElementTree as ET

root = ET.parse(sys.argv[1]).getroot()
nodes = root.findall(".//node")
named = [node for node in nodes if node.attrib.get("content-desc", "").strip()]
print(len(nodes), len(named))
PY
)"

device_name="$("$adb" -s "$serial" shell getprop ro.product.model | tr -d '\r')"
api_level="$("$adb" -s "$serial" shell getprop ro.build.version.sdk | tr -d '\r')"
abi="$("$adb" -s "$serial" shell getprop ro.product.cpu.abi | tr -d '\r')"
source_revision="$(git rev-parse HEAD)"
if [[ -n "$(git status --porcelain --untracked-files=no)" ]]; then
    source_dirty_flag="--source-dirty"
else
    source_dirty_flag="--no-source-dirty"
fi
adb_version="$("$adb" version | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
java_version="$(java -version 2>&1 | tr '\n' ' ' | sed 's/[[:space:]]*$//')"
rustc_version="$(rustc -Vv | tr '\n' ' ' | sed 's/[[:space:]]*$//')"

python3 "$script_dir/qa_android_emulator_evidence.py" \
    --artifact "$artifact" \
    --before "$before" \
    --after "$after" \
    --accessibility "$accessibility" \
    --device-name "$device_name" \
    --serial "$serial" \
    --api-level "$api_level" \
    --abi "$abi" \
    --package "$package" \
    --activity "$activity" \
    --launch-pid "$launch_pid" \
    --launch-time-ms "$launch_time_ms" \
    --accessibility-node-count "$accessibility_node_count" \
    --accessible-named-node-count "$accessible_named_node_count" \
    --before-unique-colors "$before_unique_colors" \
    --after-unique-colors "$after_unique_colors" \
    --pixel-width "$pixel_width" \
    --pixel-height "$pixel_height" \
    --source-revision "$source_revision" \
    "$source_dirty_flag" \
    --adb "$adb_version" \
    --java "$java_version" \
    --rustc "$rustc_version"

echo "Android emulator evidence: $artifact $before $after $accessibility"
