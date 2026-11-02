#!/usr/bin/env bash
# Run real D-Bus clients against the managed shell and capture its toast and history.
# tests/notifications.sh path/to/ferro-desktop output-directory
set -euo pipefail
[[ $# == 2 || $# == 3 ]] || { echo "usage: $0 /path/to/ferro-desktop capture-directory [windows|macos|material]" >&2; exit 2; }
profile=${3:-windows}
case "$profile" in windows|macos|material) ;; *) exit 2 ;; esac
binary=$(realpath -- "$1")
mkdir -p -- "$2"; output=$(realpath -- "$2")
here=$(cd -- "$(dirname -- "$0")" && pwd)
if [[ -z ${FERRO_NOTICE_PRIVATE_DISPLAY:-} || ${DISPLAY:-} != "$FERRO_NOTICE_PRIVATE_DISPLAY" ]]; then
  exec timeout 90s env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET xvfb-run -a -s '-screen 0 1280x800x24 -nolisten tcp' dbus-run-session -- \
    bash -c 'export FERRO_NOTICE_PRIVATE_DISPLAY=$DISPLAY; exec "$@"' bash "$0" "$binary" "$output" "$profile"
fi
scratch=$(mktemp -d)
export XDG_RUNTIME_DIR="$scratch/runtime" XDG_CONFIG_HOME="$scratch/config"
mkdir -m 700 -- "$XDG_RUNTIME_DIR" "$XDG_CONFIG_HOME"
mkdir -p "$XDG_CONFIG_HOME/ferro-desktop"
printf '%s\n' "$profile" >"$XDG_CONFIG_HOME/ferro-desktop/appearance"
export PATH="$here/../session:$PATH" FERRO_DESKTOP_BINARY="$binary"
preview= supervisor= monitor= client=
cleanup() {
  for pid in "$monitor" "$client" "$supervisor" "$preview"; do [[ -z $pid ]] || kill "$pid" 2>/dev/null || true; done
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
# Give the clients distinguishable art. Counting the actual icon bands in the
# presented pixels synchronizes captures with the delivered notifications.
convert -size 64x64 xc:none -fill '#B83F69' -draw 'roundrectangle 1,1 62,62 14,14' \
  -fill white -draw 'polygon 14,31 19,26 27,34 44,17 50,23 27,46' "$scratch/build.png"
convert "$scratch/build.png" -fill '#A2E453' -draw 'circle 49,49 49,38' "$scratch/coverage.png"
convert -size 64x64 xc:none -fill '#316EC9' -draw 'roundrectangle 1,1 62,62 14,14' \
  -fill white -draw 'roundrectangle 13,17 50,49 4,4' -fill '#316EC9' -draw 'rectangle 15,27 48,29' "$scratch/calendar.png"
convert -size 64x64 xc:none -fill '#C77F2A' -draw 'roundrectangle 1,1 62,62 14,14' \
  -fill white -draw 'polygon 11,20 26,20 31,25 53,25 51,47 12,47' "$scratch/files.png"

frame_with_cards() {
  local target=$1 count=$2
  import -window root "$target"
  /usr/bin/python3 - "$target" "$count" "${3:-}" <<'PYCOUNT'
import sys
from PIL import Image, ImageChops
im = Image.open(sys.argv[1]).convert("RGB")
if sys.argv[3]:
    x, y, w, h = map(int, sys.argv[3].split(","))
    im = im.crop((x, y, x + w, y + h))

def pixels(rgb):
    bands = [band.point([255 * (abs(i - value) <= 3) for i in range(256)])
             for band, value in zip(im.split(), rgb)]
    return ImageChops.multiply(ImageChops.multiply(*bands[:2]), bands[2])

# One collapsed app icon could also be an earlier, incomplete history. Require
# the last delivery's contrasting status dot before accepting any capture.
if pixels((0xa2, 0xe4, 0x53)).histogram()[255] < 4:
    sys.exit(1)
mask = pixels((0xb8, 0x3f, 0x69))
# Each icon occupies one contiguous band of rows. The check mark is
# surrounded by the colored tile on every row where it occurs.
occupied = [mask.crop((0, y, im.width, y + 1)).histogram()[255] >= 4 for y in range(im.height)]
sys.exit(sum(hit and (i == 0 or not occupied[i - 1]) for i, hit in enumerate(occupied)) != int(sys.argv[2]))
PYCOUNT
}

notify() {
  local icon desktop
  case ${6:-Build service} in
    Calendar) icon=calendar; desktop=org.ferroui.Calendar ;;
    Files) icon=files; desktop=org.ferroui.Files ;;
    *) icon=build; desktop=org.ferroui.Build ;;
  esac
  call Notify "${6:-Build service}" "$1" "$scratch/${7:-$icon}.png" "$2" "$3" "$4" "{'desktop-entry': <'$desktop'>}" "$5" |
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
# Keep an independent client focused while the notification is mapped and replaced.
python3 "$here/julia.py" > "$scratch/study.ppm"
display -immutable -title 'Ferro notification focus probe' -geometry +140+110 "$scratch/study.ppm" >"$output/client.log" 2>&1 & client=$!
client_window=$(timeout 10 xdotool search --onlyvisible --sync --name '^Ferro notification focus probe$' | head -1)
xdotool windowactivate --sync "$client_window"
import -window root "$scratch/before-banner.png"
id=$(notify 0 'Compiling workspace' 'Compiling the desktop shell' '[]' 0)
[[ -n $id && $id != 0 ]]
until_ok toast >/dev/null
window=$(toast)
[[ $(notify "$id" 'Build succeeded' 'All targets passed on Linux.' "['open', 'Open build log']" 0 'Build service' coverage) == "$id" ]]
[[ $(toast) == "$window" && $(xdotool getactivewindow) == "$client_window" ]]
# Xvfb has one 1280-pixel output, sometimes without a RandR primary flag.
# A D-Bus reply precedes drawing. Wait for the replacement's supplied artwork
# inside its native rectangle before using its action row or publishing the frame.
updated_banner() {
  local geometry width height
  geometry=$(xdotool getwindowgeometry --shell "$window") || return 1
  x=$(sed -n 's/^X=//p' <<<"$geometry"); y=$(sed -n 's/^Y=//p' <<<"$geometry")
  width=$(sed -n 's/^WIDTH=//p' <<<"$geometry"); height=$(sed -n 's/^HEIGHT=//p' <<<"$geometry")
  frame_with_cards "$output/notification-toast.png" 1 "$x,$y,$width,$height"
}
until_ok updated_banner
case "$profile" in
    windows) (( x > 850 && y > 540 && y < 680 ));;
    macos) (( x > 850 && y > 20 && y < 100 ));;
    material) (( x > 380 && x < 500 && y > 20 && y < 100 ));;
esac || { echo "Unexpected $profile banner position $x,$y" >&2; exit 1; }
xprop -id "$window" _NET_WM_WINDOW_TYPE | grep -q '_NET_WM_WINDOW_TYPE_NOTIFICATION'
# Openbox in this fixture has no compositing manager. Transparent rounded
# corners must expose the underlying output through the X11 bounding region.
before=$(convert "$scratch/before-banner.png" -format "%[pixel:p{$x,$y}]" info:)
shown=$(convert "$output/notification-toast.png" -format "%[pixel:p{$x,$y}]" info:)
[[ $before == "$shown" ]] || { echo "Opaque $profile toast corner at $x,$y: $before -> $shown" >&2; exit 1; }
# The visible action must emit ActionInvoked before NotificationClosed.
case "$profile" in
    material) action_y=128 ;;
    *) action_y=96 ;;
esac
xdotool mousemove --window "$window" 100 "$action_y" click 1
until_ok signal_seen "ActionInvoked (uint32 $id, 'open')"
until_ok signal_seen "NotificationClosed (uint32 $id, uint32 2)"
until_ok no_toast
default=$(notify 0 'Inspect workspace' 'Open the current build.' "['default', 'Inspect']" 0)
until_ok toast >/dev/null
default_target() {
  signal_seen "ActionInvoked (uint32 $default, 'default')" && return 0
  local target; target=$(toast) || return 1
  xdotool mousemove --window "$target" 110 65 click 1
  signal_seen "ActionInvoked (uint32 $default, 'default')"
}
until_ok default_target
until_ok signal_seen "NotificationClosed (uint32 $default, uint32 2)"
until_ok no_toast
expired=$(notify 0 'Analysis finished' 'The report is in the project folder.' '[]' 250)
until_ok signal_seen "NotificationClosed (uint32 $expired, uint32 1)"
if call CloseNotification "$expired" >/dev/null 2>&1; then echo 'Expired identifier was accepted' >&2; exit 1; fi
recalled=$(notify 0 'Uploading symbols' 'Waiting for the server' '[]' 0)
call CloseNotification "$recalled" >/dev/null
until_ok signal_seen "NotificationClosed (uint32 $recalled, uint32 3)"
until_ok no_toast
kill "$client"; wait "$client" 2>/dev/null || true; client=
# Opening the center focuses its first control, Do Not Disturb.
xdotool windowfocus --sync "$shell_window" key --clearmodifiers ctrl+n
sleep .3; xdotool key space; sleep .2
silent=$(notify 0 'New review requested' 'Desktop appearance and keyboard navigation' '[]' 0)
calendar=$(notify 0 'Design review in 15 minutes' 'Studio · planning calendar' '[]' 0 'Calendar')
files=$(notify 0 'Project copied' '18 files copied to the workspace.' '[]' 0 'Files')
more1=$(notify 0 'Package archived' 'Artifacts are ready to download.' '[]' 0)
more2=$(notify 0 'Coverage report' 'All reviewed branches passed.' '[]' 0 'Build service' coverage)
[[ -n $silent && -n $calendar && -n $files && -n $more1 && -n $more2 ]]
no_toast

collapsed=1; if [[ $profile == windows ]]; then collapsed=3; fi
until_ok frame_with_cards "$scratch/history.png" "$collapsed"
cp "$scratch/history.png" "$output/notification-center.png"
# Move from Do Not Disturb to the first source's disclosure. Its adjacent
# clear action retires that group and leaves the other two senders active.
xdotool key --clearmodifiers Tab Tab
# A replacement while the disclosure has keyboard focus must preserve it.
[[ $(notify "$more2" 'Coverage report updated' 'All reviewed branches passed.' '[]' 0 'Build service' coverage) == "$more2" ]]
sleep .25
xdotool key --clearmodifiers space
until_ok frame_with_cards "$scratch/history.png" 4
cp "$scratch/history.png" "$output/notification-expanded.png"
xdotool key --clearmodifiers Tab space
until_ok signal_seen "NotificationClosed (uint32 $more2, uint32 2)"
until_ok signal_seen "NotificationClosed (uint32 $more1, uint32 2)"
until_ok signal_seen "NotificationClosed (uint32 $silent, uint32 2)"
if [[ $profile == material ]]; then
  # Swipe the actual Files card. Locate its sender icon in the newly laid-out shade.
  files_card() {
    import -window root "$scratch/files-card.png"
    /usr/bin/python3 - "$scratch/files-card.png" >"$scratch/card-position" <<'PYPOINT'
import sys
from PIL import Image, ImageChops
im = Image.open(sys.argv[1]).convert("RGB")
bands = [band.point([255 * (abs(i - value) <= 3) for i in range(256)])
         for band, value in zip(im.split(), (0xc7, 0x7f, 0x2a))]
bbox = ImageChops.multiply(ImageChops.multiply(*bands[:2]), bands[2]).getbbox()
if not bbox or bbox[2] - bbox[0] > 32 or bbox[3] - bbox[1] > 32:
    sys.exit(1)
print((bbox[0] + bbox[2]) // 2, (bbox[1] + bbox[3]) // 2)
PYPOINT
  }
  until_ok files_card
  read -r px py <"$scratch/card-position"
  xdotool mousemove "$px" "$py" mousedown 1 sleep .08 mousemove "$((px+46))" "$py" sleep .08 mousemove "$((px+140))" "$py" mouseup 1
  until_ok signal_seen "NotificationClosed (uint32 $files, uint32 2)"
else
  call CloseNotification "$files" >/dev/null
fi
xdotool key --clearmodifiers Escape
no_toast
call CloseNotification "$calendar" >/dev/null
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
echo "Verified $profile banner placement, client actions, sender artwork, stack expansion and clearing, expiration, recall, quiet mode and eviction."
