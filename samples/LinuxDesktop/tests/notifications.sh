#!/usr/bin/env bash
# Run real D-Bus clients against the managed shell and capture its toast and history.
# tests/notifications.sh path/to/ferro-desktop output-directory
set -euo pipefail
[[ $# == 2 ]] || { echo "usage: $0 /path/to/ferro-desktop capture-directory" >&2; exit 2; }
binary=$(realpath -- "$1")
mkdir -p -- "$2"; output=$(realpath -- "$2")
here=$(cd -- "$(dirname -- "$0")" && pwd)
if [[ -z ${FERRO_NOTICE_PRIVATE_DISPLAY:-} || ${DISPLAY:-} != "$FERRO_NOTICE_PRIVATE_DISPLAY" ]]; then
  exec timeout 90s xvfb-run -a -s '-screen 0 1280x800x24 -nolisten tcp' dbus-run-session -- \
    bash -c 'export FERRO_NOTICE_PRIVATE_DISPLAY=$DISPLAY; exec "$@"' bash "$0" "$binary" "$output"
fi
scratch=$(mktemp -d)
export XDG_RUNTIME_DIR="$scratch/runtime" XDG_CONFIG_HOME="$scratch/config"
mkdir -m 700 -- "$XDG_RUNTIME_DIR" "$XDG_CONFIG_HOME"
export PATH="$here/../session:$PATH" FERRO_DESKTOP_BINARY="$binary"
preview= supervisor= monitor=
cleanup() {
  for pid in "$monitor" "$supervisor" "$preview"; do [[ -z $pid ]] || kill "$pid" 2>/dev/null || true; done
  wait || true
  rm -rf -- "$scratch"
}
trap cleanup EXIT
name=org.freedesktop.Notifications
path=/org/freedesktop/Notifications
call() { gdbus call --session --dest "$name" --object-path "$path" --method "$name.$1" "${@:2}"; }
toast() { xdotool search --onlyvisible --name '^Ferro Desktop Notification$' 2>/dev/null | head -1; }
until_ok() {
  local attempt
  for ((attempt=0; attempt<100; ++attempt)); do if "$@"; then return 0; fi; sleep .05; done
  echo "Timed out waiting for $*" >&2; return 1
}
no_toast() { [[ -z $(toast || :) ]]; }
signal_seen() { grep -Fq -- "$1" "$output/signals.log"; }
notify() {
  call Notify "${6:-Build service}" "$1" '' "$2" "$3" "$4" '{}' "$5" |
    sed -nE 's/^\(uint32 ([0-9]+),\)$/\1/p'
}
# An ordinary preview must leave the session-bus name available.
"$binary" >"$output/preview.log" 2>&1 & preview=$!
timeout 20 xdotool search --onlyvisible --sync --name '^Ferro Desktop$' >/dev/null
gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
  --method org.freedesktop.DBus.NameHasOwner "$name" | grep -q false
kill "$preview"; wait "$preview" 2>/dev/null || true; preview=
ferro-desktop-session >"$output/managed.log" 2>&1 & supervisor=$!
gdbus wait --session --timeout 20 "$name"
call GetServerInformation | grep -q 'Ferro Desktop'
call GetCapabilities | grep -q "'actions'"
stdbuf -oL gdbus monitor --session --dest "$name" --object-path "$path" >"$output/signals.log" & monitor=$!
until_ok signal_seen 'Monitoring signals'
shell_window=$(timeout 20 xdotool search --onlyvisible --sync --name '^Ferro Desktop$' | head -1)
id=$(notify 0 'Compiling workspace' 'Compiling the desktop shell' '[]' 0)
[[ -n $id && $id != 0 ]]
until_ok toast >/dev/null
window=$(toast)
[[ $(notify "$id" 'Build succeeded' 'All targets passed on Linux.' "['open', 'Open build log']" 0) == "$id" ]]
sleep .3
[[ $(toast) == "$window" ]]
xprop -id "$window" _NET_WM_WINDOW_TYPE | grep -q '_NET_WM_WINDOW_TYPE_NOTIFICATION'
import -window root "$output/notification-toast.png"
# The visible action must emit ActionInvoked before NotificationClosed.
xdotool mousemove --window "$window" 60 100 click 1
until_ok signal_seen "ActionInvoked (uint32 $id, 'open')"
until_ok signal_seen "NotificationClosed (uint32 $id, uint32 2)"
until_ok no_toast
expired=$(notify 0 'Analysis finished' 'The report is in the project folder.' '[]' 250)
until_ok signal_seen "NotificationClosed (uint32 $expired, uint32 1)"
if call CloseNotification "$expired" >/dev/null 2>&1; then echo 'Expired identifier was accepted' >&2; exit 1; fi
recalled=$(notify 0 'Uploading symbols' 'Waiting for the server' '[]' 0)
call CloseNotification "$recalled" >/dev/null
until_ok signal_seen "NotificationClosed (uint32 $recalled, uint32 3)"
# Opening the center focuses its first control, Do Not Disturb.
xdotool windowfocus --sync "$shell_window" key --clearmodifiers ctrl+n
sleep .3; xdotool key space; sleep .2
silent=$(notify 0 'New review requested' 'Desktop appearance and keyboard navigation' '[]' 0)
calendar=$(notify 0 'Design review in 15 minutes' 'Studio · planning calendar' '[]' 0 'Calendar')
files=$(notify 0 'Project copied' '18 files copied to the workspace.' '[]' 0 'Files')
[[ -n $silent && -n $calendar && -n $files ]]
sleep .3; no_toast
import -window root "$output/notification-center.png"
xdotool key --clearmodifiers Escape
sleep .3; no_toast
call CloseNotification "$silent" >/dev/null
call CloseNotification "$calendar" >/dev/null
call CloseNotification "$files" >/dev/null
# A sticky-message burst evicts exactly the oldest active items after expired
# history has been discarded. Individual clients independently reopen the bus.
first= last=
for ((i=0; i<66; ++i)); do
  last=$(notify 0 "Queue $i" "Capacity probe" '[]' 0)
  [[ -n $last ]]; if [[ -z $first ]]; then first=$last; fi
done
until_ok signal_seen "NotificationClosed (uint32 $first, uint32 4)"
if call CloseNotification "$first" >/dev/null 2>&1; then echo 'Evicted notification remained active' >&2; exit 1; fi
call CloseNotification "$last" >/dev/null
kill "$supervisor"; wait "$supervisor" || true; supervisor=
name_released() {
  gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.NameHasOwner "$name" | grep -q false
}
until_ok name_released
echo "Verified private name ownership, replacement, action, expiration, recall, quiet mode and history."
