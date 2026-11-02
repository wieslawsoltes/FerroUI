#!/usr/bin/env bash
# Exercise Material hotseat suggestions, touch-style holding and freedesktop actions on a private X server.
set -euo pipefail
if [[ ${1:-} != --fixture ]]; then
    binary=$(realpath "${1:?Supply a desktop binary}")
    out=$(realpath -m "${2:?Supply an output directory}")
    mkdir -p -- "$out"
    exec timeout 80s env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET xvfb-run -a -s '-screen 0 1440x960x24 -nolisten tcp' \
        dbus-run-session -- "$0" --fixture "$binary" "$out"
fi
binary=$2 out=$3
fixture=$(mktemp -d)
export XDG_DATA_HOME="$fixture/data" XDG_CONFIG_HOME="$fixture/config" XDG_RUNTIME_DIR="$fixture/run" XDG_CACHE_HOME="$fixture/cache"
mkdir -m700 "$XDG_RUNTIME_DIR"
mkdir -p "$XDG_DATA_HOME/applications" "$XDG_CONFIG_HOME/ferro-desktop"
wm= shell=
cleanup() {
    [[ -z $shell ]] || kill "$shell" 2>/dev/null || :
    [[ -z $wm ]] || kill "$wm" 2>/dev/null || :
    wait || :
    rm -rf -- "$fixture"
}
trap cleanup EXIT
# Unique icon colors locate the real buttons even if the compositor places or
# scales the window differently. The marker distinguishes a shortcut from Open.
convert -size 64x64 xc:none -fill '#d94678' -draw 'roundrectangle 1,1 62,62 13,13' -fill white \
    -draw 'polygon 16,13 43,13 49,19 49,51 16,51 rectangle 22,23 40,25' "$fixture/notes.png"
convert -size 64x64 xc:none -fill '#4266c5' -draw 'circle 32,32 32,2' -fill white \
    -draw 'polygon 14,43 31,18 49,43' "$fixture/studio.png"
apps="$XDG_DATA_HOME/applications"
cat > "$apps/leaves.desktop" <<ENTRY
[Desktop Entry]
Type=Application
Name=Leaf Notes
Icon=$fixture/notes.png
Exec=/usr/bin/touch "$fixture/default"
Actions=Compose;
[Desktop Action Compose]
Name=New note
Exec=/usr/bin/touch "$fixture/compose"
ENTRY
cat > "$apps/studio.desktop" <<ENTRY
[Desktop Entry]
Type=Application
Name=Paper Studio
Icon=$fixture/studio.png
Exec=/usr/bin/true
ENTRY
pins="$XDG_CONFIG_HOME/ferro-desktop/favorites.json"
printf '%s\n' '["leaves.desktop","studio.desktop"]' > "$pins"
openbox --sm-disable >"$out/home-openbox.log" 2>&1 & wm=$!
"$binary" --theme=material >"$out/home-shell.log" 2>&1 & shell=$!
win=$(timeout 20s xdotool search --sync --onlyvisible --name '^Ferro Desktop$' | head -1)
xdotool windowactivate --sync "$win"
key() { xdotool key --clearmodifiers "$@"; }
capture() { import -window "$win" "$1"; }
until_ok() {
    for ((attempt=0;attempt<60;attempt++)); do if "$@"; then return; fi; sleep .08; done
    echo "Timed out waiting for $*" >&2; capture "$out/home-failure.png"; return 1
}
note_location() {
    capture "$fixture/frame.png"
    # Only count the Home row above the search pill. All Apps can contain
    # the same icon elsewhere while a launch request is in flight.
    /usr/bin/python3 - "$fixture/frame.png" <<'PY'
import sys
from PIL import Image, ImageChops
im = Image.open(sys.argv[1]).convert("RGB")
top, bottom = im.height - 240, im.height - 150
piece = im.crop((0, top, im.width, bottom))
def mask(band, value):
    return band.point([255 if abs(i - value) <= 4 else 0 for i in range(256)])
r, g, b = [mask(band, value) for band, value in zip(piece.split(), (217,70,120))]
color = ImageChops.multiply(ImageChops.multiply(r, g), b)
if color.histogram()[255] < 100:
    sys.exit(1)
l,t,r,b = color.getbbox()
print((l+r)//2, top+(t+b)//2)
PY
}
find_note() { coords=$(note_location) && read -r x y <<<"$coords"; }
open_menu() {
    xdotool mousemove --window "$win" "$x" "$y"
    if [[ $1 == hold ]]; then
        xdotool mousedown 1; sleep 1.1; xdotool mouseup 1
    else
        xdotool click 3
    fi
}
menu_visible() {
    capture "$fixture/menu.png"
    local delta=$(compare -fuzz 3% -metric AE "$fixture/frame.png" "$fixture/menu.png" null: 2>&1 || :)
    awk -v pixels="$delta" 'BEGIN {exit !(pixels+0>3500)}'
}
pins_are() { [[ $(cat "$pins") == "$1" ]]; }
until_ok find_note
cp "$fixture/frame.png" "$out/material-home.png"
open_menu right
until_ok menu_visible
cp "$fixture/menu.png" "$out/material-app-shortcuts.png"
key Home Return
until_ok test -f "$fixture/compose"
[[ ! -e $fixture/default ]]
# GIO can dispatch the action before the closing popup leaves the frame.
# Synchronize with its former location before testing a second opening.
menu_dismissed() {
    capture "$fixture/frame.png"
    local region="270x135+$((x-135))+$((y-178))"
    convert "$fixture/frame.png" -crop "$region" +repage "$fixture/after.png"
    convert "$out/material-home.png" -crop "$region" +repage "$fixture/before.png"
    local delta=$(compare -fuzz 3% -metric AE "$fixture/before.png" "$fixture/after.png" null: 2>&1 || :)
    awk -v pixels="$delta" 'BEGIN {exit !(pixels+0<250)}'
}
until_ok menu_dismissed
until_ok find_note
open_menu hold
until_ok menu_visible
key End Return
until_ok pins_are '["studio.desktop"]'
note_absent() { ! note_location >/dev/null; }
until_ok note_absent
[[ ! -e $fixture/default ]] || { echo 'Holding a shortcut also launched its app' >&2; exit 1; }
# A real default launch now suggests Leaf Notes in the next empty slot.
key ctrl+k; sleep .35
xdotool type --clearmodifiers --delay 25 'Leaf Notes'
key Return
until_ok test -f "$fixture/default"
until_ok find_note
cp "$fixture/frame.png" "$out/material-suggested.png"
# The canvas menu changes only the Android prediction row. Pins and recent
# launches survive toggling suggestions off and on.
suggestions_are() { [[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/material-home-suggestions" 2>/dev/null) == "$1" ]]; }
toggle_suggestions() {
    capture "$fixture/frame.png"
    xdotool mousemove --window "$win" 188 376 click 3
    until_ok menu_visible
    key End Return
    until_ok suggestions_are "$1"
}
toggle_suggestions off
until_ok pins_are '["studio.desktop"]'
until_ok note_absent
toggle_suggestions on
until_ok find_note
open_menu right
until_ok menu_visible
key End Return
until_ok pins_are '["studio.desktop","leaves.desktop"]'
until_ok menu_dismissed
until_ok find_note
# Press on the tile, then release outside to move focus without opening
# its application. Invoke the shortcut menu from the keyboard.
rm -f "$fixture/compose"
xdotool mousemove --window "$win" "$x" "$y" mousedown 1 mousemove --window "$win" "$((x+70))" "$((y-20))" mouseup 1
sleep .15
key Menu
until_ok menu_visible
key Home Return
until_ok test -f "$fixture/compose"
# Verify the actual artwork after a live resize. The rust-colored flower
# center stays in view, and the clock retains dark green wallpaper behind it.
garden_presented() {
    local w=$1 h=$2 x=$3 y=$4
    capture "$fixture/garden.png"
    /usr/bin/python3 - "$fixture/garden.png" "$w" "$h" "$x" "$y" <<'PYGARDEN'
import sys
from PIL import Image
w, h, x, y = map(int, sys.argv[2:])
im = Image.open(sys.argv[1]).convert("RGB")
if im.size != (w,h):
    sys.exit(1)
red, green, blue = im.getpixel((x,y))
r, g, b = im.getpixel((min(255, w - 1), 118))
sys.exit(0 if red > green + 40 and red > blue + 50 and g > r + 8 and g > b + 3 else 1)
PYGARDEN
}
xdotool windowsize --sync "$win" 360 640
until_ok garden_presented 360 640 252 384
cp "$fixture/garden.png" "$out/material-portrait.png"
xdotool windowsize --sync "$win" 1280 800
until_ok garden_presented 1280 800 896 295
key Escape alt+F4
wait "$shell"; shell=
echo 'Verified Material home app actions, hold without launch, opt-in/out of app suggestions and persistent pin changes.'
