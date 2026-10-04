#!/usr/bin/env bash
set -euo pipefail

[[ $(uname -s) == Darwin ]] || { echo 'Windows UTM QA requires macOS' >&2; exit 1; }
utmctl=${GPUI_UTMCTL:-/Applications/UTM.app/Contents/MacOS/utmctl}
vm=${GPUI_UTM_WINDOWS_VM:-Win11 ARM AutoEQ}
user=${GPUI_UTM_WINDOWS_USER:-pierre}
base=${GPUI_UTM_WINDOWS_ROOT:-'C:\gpui-toolkit-qa'}
toolkit=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
sources=${GPUI_UTM_SOURCE_ROOT:-$(cd "$toolkit/.." && pwd)}
artifact=${1:-target/qa/native-ui/windows/gpui-builder-smoke.json}
screenshot=${2:-target/qa/native-ui/windows/gpui-builder.png}
run_id="$(date -u +%Y%m%d%H%M%S)-$$-$RANDOM"
guest="${base}\\${run_id}"
guest_project="${guest}\src\gpui-toolkit"
guest_status="${guest}\status.json"
guest_artifacts="C:\Users\\${user}\AppData\Local\Temp\gpui-toolkit-qa-${run_id}"
evidence=$(dirname "$artifact")

[[ -x $utmctl && $base == 'C:\gpui-toolkit-qa' && $user =~ ^[A-Za-z0-9_.-]+$ ]] || {
    echo 'invalid UTM CLI, dedicated QA root, or guest user' >&2; exit 1;
}
for tool in tar magick python3 shasum git; do
    command -v "$tool" >/dev/null || { echo "missing host tool: $tool" >&2; exit 1; }
done
[[ $toolkit == "$sources/gpui-toolkit" ]] || { echo 'toolkit must be in pinned sibling checkout' >&2; exit 1; }
for sibling in gpui-toolkit math-audio; do
    [[ -f $sources/$sibling/Cargo.toml ]] || { echo "missing sibling: $sibling" >&2; exit 1; }
done
[[ -f $sources/scripts/release/sources.json ]] || { echo 'missing pinned sources.json' >&2; exit 1; }
bounded() {
    local seconds=$1
    shift
    python3 -c '
import os, signal, subprocess, sys
seconds = int(sys.argv[1])
process = subprocess.Popen(sys.argv[2:], stdin=sys.stdin.buffer, start_new_session=True)
def stop_child(signum, _frame):
    for pending in (signal.SIGINT, signal.SIGTERM):
        signal.signal(pending, signal.SIG_IGN)
    try:
        os.killpg(process.pid, signum)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
    raise SystemExit(128 + signum)
for signum in (signal.SIGINT, signal.SIGTERM):
    signal.signal(signum, stop_child)
try:
    status = process.wait(timeout=seconds)
except subprocess.TimeoutExpired:
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()
    raise SystemExit(f"UTM command timed out after {seconds}s")
raise SystemExit(status)
' "$seconds" "$@"
}

mkdir -p "$evidence" "$(dirname "$screenshot")"
evidence=$(cd "$evidence" && pwd)
cp "$sources/scripts/release/sources.json" "$evidence/sources.json"
cp "$toolkit/Cargo.lock" "$evidence/Cargo.lock.before"
for sibling in gpui-toolkit math-audio; do
    git -C "$sources/$sibling" rev-parse HEAD >"$evidence/$sibling-revision.txt"
    git -C "$sources/$sibling" status --porcelain >"$evidence/$sibling-status.before"
    [[ ! -s $evidence/$sibling-status.before ]] || { echo "dirty source: $sibling" >&2; exit 1; }
done
shasum -a 256 "$evidence/Cargo.lock.before" >"$evidence/lock-before.sha256"
python3 - "$evidence" <<'PY'
import json, pathlib, sys
directory = pathlib.Path(sys.argv[1])
manifest = json.loads((directory / 'sources.json').read_text())['sources']
for sibling in ('gpui-toolkit', 'math-audio'):
    actual = (directory / f'{sibling}-revision.txt').read_text().strip()
    expected = manifest[sibling]['revision']
    if actual != expected:
        raise SystemExit(f'{sibling}: checkout {actual} differs from pinned {expected}')
PY

initial=$(bounded 15 "$utmctl" status "$vm")
[[ $initial == started || $initial == stopped ]] || { echo "unsupported VM state: $initial" >&2; exit 1; }
temp=$(mktemp -d "${TMPDIR:-/tmp}/gpui-utm-windows.XXXXXX")
started=false
created=false
cleanup() {
    local result=$?
    trap - EXIT INT TERM
    set +e
    if [[ $created == true ]]; then
        bounded 20 "$utmctl" file pull "$vm" "$guest_project\Cargo.lock" >"$evidence/Cargo.lock.after-guest" 2>"$evidence/lock-pull.log"
        if [[ $? == 0 ]]; then
            shasum -a 256 "$evidence/Cargo.lock.after-guest" >"$evidence/lock-after-guest.sha256"
            cmp -s "$evidence/Cargo.lock.before" "$evidence/Cargo.lock.after-guest" || result=1
        else
            echo 'Failed to retrieve guest Cargo.lock' >&2
            result=1
        fi
        bounded 120 "$utmctl" exec "$vm" --cmd powershell.exe -NoProfile -NonInteractive -Command \
            "\$marker = '$guest_status.build-owner.json'; if (Test-Path -LiteralPath \$marker) { \$owned = Get-Content -LiteralPath \$marker -Raw | ConvertFrom-Json; if (\$owned.run_id -ne '$run_id' -or \$owned.repo_root -ne '$guest_project') { throw 'guest build ownership mismatch' }; \$owner = Get-CimInstance Win32_Process -Filter ('ProcessId = ' + [int]\$owned.pid); if (\$owner) { if (-not \$owner.CommandLine.Contains('$guest_project') -or -not \$owner.CommandLine.Contains('$run_id')) { throw 'guest build process identity mismatch' }; \$all = @(Get-CimInstance Win32_Process); \$ids = @([int]\$owned.pid); for (\$i = 0; \$i -lt \$ids.Count; \$i++) { \$parent = \$ids[\$i]; \$ids += @(\$all | Where-Object { \$_.ParentProcessId -eq \$parent } | ForEach-Object { [int]\$_.ProcessId }) }; [array]::Reverse(\$ids); foreach (\$processId in \$ids) { Stop-Process -Id \$processId -Force -ErrorAction SilentlyContinue }; \$deadline = (Get-Date).AddSeconds(10); do { \$live = @(\$ids | Where-Object { Get-Process -Id \$_ -ErrorAction SilentlyContinue }); if (\$live.Count -eq 0) { break }; Start-Sleep -Milliseconds 200 } while ((Get-Date) -lt \$deadline); if (\$live.Count -ne 0) { throw 'owned guest build process did not stop' } } }; Stop-ScheduledTask -TaskName 'GpuiToolkitNativeUiSmoke-$run_id' -ErrorAction SilentlyContinue; Unregister-ScheduledTask -TaskName 'GpuiToolkitNativeUiSmoke-$run_id' -Confirm:\$false -ErrorAction SilentlyContinue; Remove-Item -LiteralPath '$guest_artifacts' -Recurse -Force -ErrorAction SilentlyContinue; Remove-Item -LiteralPath '$guest' -Recurse -Force -ErrorAction Stop" \
            >"$evidence/guest-cleanup.log" 2>&1 || result=1
    fi
    if [[ $started == true ]]; then
        bounded 45 "$utmctl" stop "$vm" >"$evidence/vm-restore.log" 2>&1 || result=1
    fi
    final=$(bounded 15 "$utmctl" status "$vm" 2>&1) || result=1
    printf '%s\n' "$final" >"$evidence/vm-final-state.txt"
    [[ $final == "$initial" ]] || result=1
    for sibling in gpui-toolkit math-audio; do
        git -C "$sources/$sibling" status --porcelain >"$evidence/$sibling-status.after" || result=1
        cmp -s "$evidence/$sibling-status.before" "$evidence/$sibling-status.after" || result=1
    done
    shasum -a 256 "$toolkit/Cargo.lock" >"$evidence/lock-after-host.sha256" || result=1
    cmp -s "$evidence/Cargo.lock.before" "$toolkit/Cargo.lock" || result=1
    rm -rf "$temp" || result=1
    exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if [[ $initial == stopped ]]; then
    started=true
    bounded 45 "$utmctl" start "$vm" >"$evidence/vm-start.log" 2>&1
fi
ready=false
for ((attempt=0; attempt<${GPUI_UTM_WAIT_ATTEMPTS:-60}; attempt++)); do
    ip=$(bounded 10 "$utmctl" ip-address "$vm" 2>&1 || true)
    if [[ $ip =~ ([0-9]{1,3}\.){3}[0-9]{1,3} ]]; then ready=true; break; fi
    sleep 2
done
[[ $ready == true ]] || { echo 'UTM guest agent unavailable' >&2; exit 1; }

existing=$(bounded 20 "$utmctl" exec "$vm" --cmd powershell.exe -NoProfile -NonInteractive -Command \
    "if (Test-Path -LiteralPath '$guest') { 'EXISTS' } else { 'ABSENT' }")
[[ $existing == *ABSENT* && $existing != *EXISTS* ]] || { echo "guest QA root already exists: $guest" >&2; exit 1; }
created=true
bounded 20 "$utmctl" exec "$vm" --cmd powershell.exe -NoProfile -NonInteractive -Command \
    "New-Item -ItemType Directory -Path '$guest' -ErrorAction Stop | Out-Null" \
    >"$evidence/guest-stage.log" 2>&1
bounded 30 "$utmctl" file push "$vm" "$guest\prepare-native-ui.ps1" <"$toolkit/scripts/prepare_utm_windows_native_ui.ps1"
bounded 90 "$utmctl" exec "$vm" --cmd powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
    -File "$guest\prepare-native-ui.ps1" -RepoRoot "$guest_project" -UserName "$user" \
    -StatusPath "$guest_status" -RunId "$run_id" -CheckOnly \
    >"$evidence/guest-preflight.log" 2>&1 || true
bounded 30 "$utmctl" file pull "$vm" "$guest_status" >"$evidence/guest-preflight.json"
python3 - "$evidence/guest-preflight.json" <<'PY'
import json, sys
state = json.load(open(sys.argv[1], encoding='utf-8-sig'))
if state.get('state') != 'desktop-ready':
    raise SystemExit(state.get('message', 'Windows preflight failed'))
PY

mkdir -p "$temp/stage/gpui-toolkit" "$temp/stage/math-audio"
for sibling in gpui-toolkit math-audio; do
    pinned_revision=$(cat "$evidence/$sibling-revision.txt")
    git -C "$sources/$sibling" archive --format=tar "$pinned_revision" |
        tar -xf - -C "$temp/stage/$sibling"
done
COPYFILE_DISABLE=1 tar -czf "$temp/sources.tar.gz" -C "$temp/stage" gpui-toolkit math-audio
shasum -a 256 "$temp/sources.tar.gz" >"$evidence/sources-archive.sha256"
bounded 300 "$utmctl" file push "$vm" "$guest\sources.tar.gz" <"$temp/sources.tar.gz"
bounded 30 "$utmctl" file push "$vm" "$guest\prepare-workspace.ps1" <"$toolkit/scripts/prepare_utm_windows_workspace.ps1"
bounded 120 "$utmctl" exec "$vm" --cmd powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
    -File "$guest\prepare-workspace.ps1" -Root "$guest" -Archive "$guest\sources.tar.gz" \
    >"$evidence/guest-workspace.log" 2>&1
bounded 3600 "$utmctl" exec "$vm" --cmd powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass \
    -File "$guest_project\scripts\prepare_utm_windows_native_ui.ps1" -RepoRoot "$guest_project" \
    -UserName "$user" -StatusPath "$guest_status" -RunId "$run_id" \
    >"$evidence/guest-build.log" 2>&1 || true
bounded 30 "$utmctl" file pull "$vm" "$guest_status" >"$evidence/guest-build.json"
python3 - "$evidence/guest-build.json" <<'PY'
import json, sys
state = json.load(open(sys.argv[1], encoding='utf-8-sig'))
if state.get('state') != 'scheduled':
    raise SystemExit(state.get('message', 'Windows build or smoke scheduling failed'))
PY

found=false
for ((attempt=0; attempt<${GPUI_UTM_CAPTURE_ATTEMPTS:-90}; attempt++)); do
    bounded 20 "$utmctl" file pull "$vm" "$guest_artifacts\gpui-builder-smoke.json" >"$temp/report.json" 2>/dev/null || true
    if python3 - "$temp/report.json" <<'PY' >/dev/null 2>&1
import json, sys
assert json.load(open(sys.argv[1], encoding='utf-8-sig')).get('report_type') == 'gpui-native-smoke'
PY
    then
        cp "$temp/report.json" "$artifact"
        found=true
        break
    fi
    bounded 20 "$utmctl" file pull "$vm" "$guest_artifacts\gpui-builder-smoke.json.error.txt" >"$evidence/guest-smoke-error.txt" 2>/dev/null || true
    if [[ -s $evidence/guest-smoke-error.txt ]]; then
        cat "$evidence/guest-smoke-error.txt" >&2
        exit 1
    fi
    sleep 2
done
[[ $found == true ]] || { echo 'Windows interactive smoke evidence unavailable' >&2; exit 1; }
bounded 60 "$utmctl" file pull "$vm" "$guest_artifacts\gpui-builder.png" >"$screenshot"
colors=$(magick identify -format '%k' "$screenshot")
python3 "$toolkit/scripts/qa_native_ui_evidence.py" --artifact "$artifact" --screenshot "$screenshot" \
    --platform windows --unique-colors "$colors" --capture-transport utm-windows-interactive-window
echo "UTM Windows native UI evidence: $artifact $screenshot"
