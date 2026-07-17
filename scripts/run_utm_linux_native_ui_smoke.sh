#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "UTM Linux orchestration must run on the macOS host" >&2
    exit 1
fi

utmctl="${GPUI_UTMCTL:-/Applications/UTM.app/Contents/MacOS/utmctl}"
vm="${GPUI_UTM_LINUX_VM:-Ubuntu 24.04 ARM}"
ssh_target="${GPUI_UTM_LINUX_SSH:-pierre@192.168.64.4}"
guest_root="${GPUI_UTM_LINUX_ROOT:-/home/pierre/gpui-toolkit-qa}"
artifact="${1:-target/qa/native-ui/linux/gpui-builder-smoke.json}"
screenshot="${2:-target/qa/native-ui/linux/gpui-builder.png}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
wait_attempts="${GPUI_UTM_WAIT_ATTEMPTS:-60}"

if [[ ! -x "$utmctl" ]]; then
    echo "UTM CLI is missing: $utmctl" >&2
    exit 1
fi
case "$guest_root" in
    /home/*/gpui-toolkit-qa) ;;
    *)
        echo "GPUI_UTM_LINUX_ROOT must be a dedicated /home/*/gpui-toolkit-qa path" >&2
        exit 1
        ;;
esac
for command in ssh rsync scp python3; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "required UTM Linux host command is missing: $command" >&2
        exit 1
    fi
done

mkdir -p "$(dirname "$artifact")" "$(dirname "$screenshot")"
original_status="$($utmctl status "$vm")"
restore_vm=true
cleanup() {
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

ssh_options=(
    -o BatchMode=yes
    -o ConnectTimeout=3
    -o ServerAliveInterval=5
    -o ServerAliveCountMax=3
)
ssh_ready=false
for _ in $(seq 1 "$wait_attempts"); do
    if ssh "${ssh_options[@]}" "$ssh_target" true >/dev/null 2>&1; then
        ssh_ready=true
        break
    fi
    sleep 2
done
if [[ "$ssh_ready" != true ]]; then
    echo "Ubuntu VM is running, but key-based SSH is unavailable at $ssh_target." >&2
    echo "Log in once and authorize this Mac's SSH public key, then rerun." >&2
    exit 1
fi

ssh "${ssh_options[@]}" "$ssh_target" "mkdir -p '$guest_root'"
rsync -az --delete \
    --exclude .git/ \
    --exclude .tokensave/ \
    --exclude target/ \
    -e "ssh ${ssh_options[*]}" \
    "$repo_root/" "$ssh_target:$guest_root/"

guest_artifact="$guest_root/target/qa/native-ui/linux/gpui-builder-smoke.json"
guest_screenshot="$guest_root/target/qa/native-ui/linux/gpui-builder.png"
ssh "${ssh_options[@]}" "$ssh_target" \
    "bash '$guest_root/scripts/run_utm_linux_guest_native_ui_smoke.sh' \
        '$guest_root' '$guest_artifact' '$guest_screenshot'"

scp "${ssh_options[@]}" "$ssh_target:$guest_artifact" "$artifact"
scp "${ssh_options[@]}" "$ssh_target:$guest_screenshot" "$screenshot"
python3 "$repo_root/scripts/qa_native_ui_evidence.py" \
    --artifact "$artifact" \
    --screenshot "$screenshot" \
    --platform linux \
    --verify

cleanup
restore_vm=false
trap - EXIT
echo "UTM Linux native UI evidence: $artifact $screenshot"
