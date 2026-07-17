#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "UTM Windows orchestration must run on the macOS host" >&2
    exit 1
fi

utmctl="${GPUI_UTMCTL:-/Applications/UTM.app/Contents/MacOS/utmctl}"
vm="${GPUI_UTM_WINDOWS_VM:-Win11 ARM AutoEQ}"
guest_user="${GPUI_UTM_WINDOWS_USER:-pierre}"
guest_root="${GPUI_UTM_WINDOWS_ROOT:-C:\\gpui-toolkit-qa}"
artifact="${1:-target/qa/native-ui/windows/gpui-builder-smoke.json}"
screenshot="${2:-target/qa/native-ui/windows/gpui-builder.png}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
wait_attempts="${GPUI_UTM_WAIT_ATTEMPTS:-60}"
capture_attempts="${GPUI_UTM_CAPTURE_ATTEMPTS:-90}"

if [[ ! -x "$utmctl" ]]; then
    echo "UTM CLI is missing: $utmctl" >&2
    exit 1
fi
if [[ ! "$guest_root" =~ ^[A-Za-z]:\\gpui-toolkit-qa$ ]]; then
    echo "GPUI_UTM_WINDOWS_ROOT must be a dedicated drive:\\gpui-toolkit-qa path" >&2
    exit 1
fi
for command in tar magick python3; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "required UTM Windows host command is missing: $command" >&2
        exit 1
    fi
done

mkdir -p "$(dirname "$artifact")" "$(dirname "$screenshot")"
temporary="$(mktemp -d "${TMPDIR:-/tmp}/gpui-utm-windows.XXXXXX")"
original_status="$($utmctl status "$vm")"
restore_vm=true
cleanup() {
    rm -rf "$temporary"
    if [[ "${GPUI_UTM_KEEP_RUNNING:-0}" == "1" || "$restore_vm" != true ]]; then
        return
    fi
    case "$original_status" in
        stopped) "$utmctl" stop "$vm" >/dev/null 2>&1 || true ;;
        suspended) "$utmctl" suspend "$vm" >/dev/null 2>&1 || true ;;
    esac
}
trap cleanup EXIT

if [[ "$original_status" != "started" ]]; then
    "$utmctl" start "$vm"
fi

agent_ready=false
for _ in $(seq 1 "$wait_attempts"); do
    agent_output="$("$utmctl" ip-address "$vm" 2>&1 || true)"
    if [[ "$agent_output" =~ ([0-9]{1,3}\.){3}[0-9]{1,3} ]]; then
        agent_ready=true
        break
    fi
    sleep 2
done
if [[ "$agent_ready" != true ]]; then
    echo "Windows VM started, but its UTM QEMU guest agent did not become ready." >&2
    exit 1
fi

guest_status="$guest_root\\host-status.json"
host_status="$temporary/host-status.json"
guest_probe="C:\\Windows\\Temp\\gpui-prepare-native-ui.ps1"
guest_powershell="C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe"
"$utmctl" file push "$vm" "$guest_probe" \
    < "$repo_root/scripts/prepare_utm_windows_native_ui.ps1"
probe_command="$guest_powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $guest_probe -RepoRoot $guest_root\\src -UserName $guest_user -StatusPath $guest_status -CheckOnly"
"$utmctl" exec "$vm" --cmd cmd.exe /d /c "$probe_command" || true
"$utmctl" file pull "$vm" "$guest_status" > "$host_status"
if ! status_state="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8-sig"))["state"])' "$host_status" 2>/dev/null)"; then
    echo "Windows guest readiness probe did not return a valid status:" >&2
    cat "$host_status" >&2
    exit 1
fi
if [[ "$status_state" != "desktop-ready" ]]; then
    status_message="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8-sig"))["message"])' "$host_status")"
    echo "$status_message" >&2
    echo "The VM lock screen is capturable, but it is not valid GPUI pixel evidence." >&2
    exit 1
fi

archive="$temporary/gpui-toolkit.tar.gz"
COPYFILE_DISABLE=1 tar -czf "$archive" \
    --exclude .git \
    --exclude .tokensave \
    --exclude target \
    -C "$repo_root" .
guest_archive="C:\\Windows\\Temp\\gpui-toolkit-qa.tar.gz"
guest_workspace_prep="C:\\Windows\\Temp\\gpui-prepare-workspace.ps1"
"$utmctl" file push "$vm" "$guest_archive" < "$archive"
"$utmctl" file push "$vm" "$guest_workspace_prep" \
    < "$repo_root/scripts/prepare_utm_windows_workspace.ps1"
workspace_command="$guest_powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $guest_workspace_prep -Root $guest_root -Archive $guest_archive"
"$utmctl" exec "$vm" --cmd cmd.exe /d /c "$workspace_command"

prepare_command="$guest_powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $guest_root\\src\\scripts\\prepare_utm_windows_native_ui.ps1 -RepoRoot $guest_root\\src -UserName $guest_user -StatusPath $guest_status"
"$utmctl" exec "$vm" --cmd cmd.exe /d /c "$prepare_command" || true
"$utmctl" file pull "$vm" "$guest_status" > "$host_status"
if ! status_state="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8-sig"))["state"])' "$host_status" 2>/dev/null)"; then
    echo "Windows guest preparation did not return a valid status:" >&2
    cat "$host_status" >&2
    exit 1
fi
if [[ "$status_state" != "scheduled" ]]; then
    python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8-sig"))["message"])' "$host_status" >&2
    exit 1
fi

guest_artifact="$guest_root\\src\\target\\qa\\native-ui\\windows\\gpui-builder-smoke.json"
guest_screenshot="$guest_root\\src\\target\\qa\\native-ui\\windows\\gpui-builder.png"
guest_error="$guest_artifact.error.txt"
artifact_ready=false
for _ in $(seq 1 "$capture_attempts"); do
    candidate="$temporary/gpui-builder-smoke.json"
    "$utmctl" file pull "$vm" "$guest_artifact" > "$candidate" 2>/dev/null || true
    if python3 -c 'import json,sys; data=json.load(open(sys.argv[1], encoding="utf-8-sig")); assert data.get("report_type") == "gpui-native-smoke"' "$candidate" >/dev/null 2>&1; then
        cp "$candidate" "$artifact"
        artifact_ready=true
        break
    fi
    error_candidate="$temporary/gpui-builder-smoke.error.txt"
    "$utmctl" file pull "$vm" "$guest_error" > "$error_candidate" 2>/dev/null || true
    error_text="$(<"$error_candidate")"
    if [[ -n "$error_text" && "$error_text" != *"failed to open file"* && "$error_text" != *"Error from event"* ]]; then
        echo "Windows interactive capture task failed:" >&2
        cat "$error_candidate" >&2
        exit 1
    fi
    sleep 2
done
if [[ "$artifact_ready" != true ]]; then
    echo "Windows interactive capture task did not produce smoke evidence." >&2
    exit 1
fi
"$utmctl" file pull "$vm" "$guest_screenshot" > "$screenshot"
unique_colors="$(magick identify -format '%k' "$screenshot")"
python3 "$repo_root/scripts/qa_native_ui_evidence.py" \
    --artifact "$artifact" \
    --screenshot "$screenshot" \
    --platform windows \
    --unique-colors "$unique_colors" \
    --capture-transport utm-windows-interactive-window

cleanup
restore_vm=false
trap - EXIT
echo "UTM Windows native UI evidence: $artifact $screenshot"
