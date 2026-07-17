#!/usr/bin/env bash
set -euo pipefail

repo_root="${1:-$PWD}"
artifact="${2:-$repo_root/target/qa/native-ui/linux/gpui-builder-smoke.json}"
screenshot="${3:-$repo_root/target/qa/native-ui/linux/gpui-builder.png}"

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "UTM Linux guest capture must run inside the Linux guest" >&2
    exit 1
fi

uid="$(id -u)"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$uid}"
export DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-unix:path=$XDG_RUNTIME_DIR/bus}"

# SSH sessions do not inherit the graphical login environment. Read only the
# display-related variables from the same user's desktop process.
session_pid=""
for process_name in gnome-shell plasmashell xfce4-session cinnamon; do
    session_pid="$(pgrep -u "$uid" -n "$process_name" 2>/dev/null || true)"
    if [[ -n "$session_pid" ]]; then
        break
    fi
done
if [[ -n "$session_pid" && -r "/proc/$session_pid/environ" ]]; then
    while IFS= read -r entry; do
        case "$entry" in
            DISPLAY=*|WAYLAND_DISPLAY=*|XAUTHORITY=*|XDG_RUNTIME_DIR=*|DBUS_SESSION_BUS_ADDRESS=*)
                export "$entry"
                ;;
        esac
    done < <(tr '\0' '\n' < "/proc/$session_pid/environ")
fi

if [[ -z "${DISPLAY:-}" ]]; then
    echo "no logged-in X11/XWayland desktop was found for $(id -un)" >&2
    echo "Log in to the Ubuntu UTM desktop before running this recipe." >&2
    exit 1
fi

for command in cargo xdotool import identify python3; do
    if ! command -v "$command" >/dev/null 2>&1; then
        echo "missing Linux guest dependency: $command" >&2
        echo "Install xdotool and ImageMagick in the Ubuntu guest." >&2
        exit 1
    fi
done

cd "$repo_root"
cargo build -p gpui-builder --features showcase --bin layout-showcase

# Force X11 so xdotool and ImageMagick capture the exact GPUI window even when
# the surrounding Ubuntu desktop session also offers Wayland.
export WINIT_UNIX_BACKEND=x11
bash scripts/run_linux_native_ui_smoke.sh \
    target/debug/layout-showcase \
    "$artifact" \
    "$screenshot" \
    utm-linux-x11-window
