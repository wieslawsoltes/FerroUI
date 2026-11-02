#!/usr/bin/env bash
# Drive pin, reorder, restart and launch through the real X11 shell.
# Usage: tests/favorites.sh path/to/ferro-desktop [output-directory] [--software]
set -euo pipefail
if [[ ${1:-} != --fixture ]]; then
    binary=$(realpath "${1:?Supply a ferro-desktop binary}")
    output=$(realpath -m "${2:-/tmp/ferro-desktop-favorites}")
    mkdir -p "$output"
    exec timeout 100s env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET xvfb-run -a -s '-screen 0 1440x960x24 -nolisten tcp' \
        dbus-run-session -- bash -c 'export FERRO_FAVORITES_FIXTURE_DISPLAY=$DISPLAY; exec "$@"' \
        bash "$0" --fixture "$binary" "$output" "${@:3}"
fi
[[ -n ${DISPLAY:-} && $DISPLAY == "${FERRO_FAVORITES_FIXTURE_DISPLAY:-}" ]]
binary=$2
output=$3
shift 3
fixture=$(mktemp -d)
shell_pid= wm_pid=
cleanup() {
    [[ -z $shell_pid ]] || kill "$shell_pid" 2>/dev/null || :
    [[ -z $wm_pid ]] || kill "$wm_pid" 2>/dev/null || :
    wait || :
    rm -rf -- "$fixture"
}
trap cleanup EXIT
export XDG_DATA_HOME="$fixture/data" XDG_CONFIG_HOME="$fixture/config" XDG_RUNTIME_DIR="$fixture/runtime"
mkdir -p "$XDG_DATA_HOME/applications" "$XDG_CONFIG_HOME"
mkdir -m 700 "$XDG_RUNTIME_DIR"
for name in Atlas Beacon; do
    lower=${name,,}
    cat >"$XDG_DATA_HOME/applications/ferro-$lower.desktop" <<ENTRY
[Desktop Entry]
Type=Application
Name=Ferro Fixture $name
Exec=/usr/bin/touch "$fixture/$lower.launched"
ENTRY
done
openbox --sm-disable >"$output/openbox.log" 2>&1 &
wm_pid=$!
start_shell() {
    local profile=$1
    shift
    "$binary" --theme="$profile" "$@" >>"$output/favorites.log" 2>&1 &
    shell_pid=$!
    shell_window=$(timeout 25s xdotool search --sync --onlyvisible --name '^Ferro Desktop$' | head -1)
    xdotool windowactivate --sync "$shell_window"
    # Wait for the first native frame before sending an accelerator.
    for ((i=0; i<80; i++)); do
        import -window "$shell_window" -resize 40x40 "$fixture/probe.png"
        (( $(identify -format '%k' "$fixture/probe.png") > 4 )) && return
        sleep .1
    done
    echo 'The shell did not paint' >&2
    return 1
}
pins_are() {
    for ((i=0; i<80; i++)); do
        if [[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/favorites.json" 2>/dev/null || :) == "$1" ]]; then return; fi
        sleep .1
    done
    echo "Timed out waiting for favorites $1" >&2
    cat "$XDG_CONFIG_HOME/ferro-desktop/favorites.json" >&2 || :
    import -window "$shell_window" "$output/failed-pins.png" || :
    cat "$output/favorites.log" >&2
    return 1
}
key() { xdotool key --clearmodifiers "$@"; }
search() {
    key ctrl+a
    xdotool type --clearmodifiers --delay 12 "Ferro Fixture $1"
}
# A catalog group comes from GDesktopAppInfo, independent of display names.
cat >"$XDG_DATA_HOME/applications/ferro-ide.desktop" <<ENTRY
[Desktop Entry]
Type=Application
Name=00 Fixture IDE
Categories=IDE;Development;
Exec=/usr/bin/touch "$fixture/ide.launched"
ENTRY
start_shell windows "$@"
key ctrl+k; sleep .25
search Atlas; key ctrl+p; pins_are '["ferro-atlas.desktop"]'
search Beacon; key ctrl+p; pins_are '["ferro-atlas.desktop","ferro-beacon.desktop"]'
key ctrl+shift+p alt+Left
pins_are '["ferro-beacon.desktop","ferro-atlas.desktop"]'
key ctrl+a BackSpace
sleep .3
import -window "$shell_window" "$output/pinned-launcher.png"
key Escape
sleep .2
import -window "$shell_window" "$output/pinned-taskbar.png"
key alt+F4
wait "$shell_pid"; shell_pid=
start_shell windows "$@"
key ctrl+k; sleep .25; key ctrl+shift+p Return
for ((i=0; i<80; i++)); do [[ -f "$fixture/beacon.launched" ]] && break; sleep .1; done
[[ -f "$fixture/beacon.launched" && ! -f "$fixture/atlas.launched" ]]
key ctrl+k; sleep .2
search Beacon; key ctrl+p; pins_are '["ferro-atlas.desktop"]'
echo 'Verified GIO launch from restored pins, reordering and removal.'

# Keep keyboard focus in the search field while traversing a page boundary.
# Spotlight has six rows per page. The wide Material drawer has fifteen tiles.
key alt+F4
wait "$shell_pid"; shell_pid=
start_shell windows "$@"
key ctrl+k; sleep .25
key ctrl+3
for ((i=0; i<60; i++)); do [[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/start-view" 2>/dev/null || :) == Category ]] && break; sleep .1; done
[[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/start-view") == Category ]]
sleep .2; import -window "$shell_window" "$output/start-categories.png"
# Development sorts first. Return opens its folder; Alt+Left returns to All.
key Return alt+Left Return Return
for ((i=0; i<80; i++)); do [[ -f "$fixture/ide.launched" ]] && break; sleep .1; done
[[ -f "$fixture/ide.launched" ]] || { echo "Start category did not launch its selected app" >&2; exit 1; }
key ctrl+k; sleep .25; key ctrl+2
for ((i=0; i<60; i++)); do [[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/start-view") == List ]] && break; sleep .1; done
[[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/start-view") == List ]]
sleep .2; import -window "$shell_window" "$output/start-recent-list.png"
key alt+F4; wait "$shell_pid"; shell_pid=
for number in {01..18}; do
    cat >"$XDG_DATA_HOME/applications/ferro-navigation-$number.desktop" <<ENTRY
[Desktop Entry]
Type=Application
Name=Ferro Navigation $number
Exec=/usr/bin/touch "$fixture/navigation-$number.launched"
ENTRY
done
for profile in macos material; do
    case "$profile" in macos) expected=06 ;; material) expected=15 ;; esac
    start_shell "$profile" "$@"
    key ctrl+k; sleep .25
    if [[ $profile == material ]]; then
        # Find the centered grabber on the presented frame. Its vertical position
        # follows the number of apps and the available height.
        for ((i=0; i<50; i++)); do
            import -window "$shell_window" "$fixture/drawer.png"
            if grabber=$(/usr/bin/python3 - "$fixture/drawer.png" <<'GRABBER'
import sys
from PIL import Image
image = Image.open(sys.argv[1]).convert("RGB")
x = image.width // 2
for y in range(1, image.height - 2):
    color = image.getpixel((x, y))
    if (max(abs(a-b) for a,b in zip(color, (185, 201, 185))) < 12
            and all(image.getpixel((p, y)) == color for p in range(x-12, x+13))
            and image.getpixel((x, y+1)) == color
            and image.getpixel((x-22, y)) != color):
        print(x, y)
        break
else:
    sys.exit(1)
GRABBER
            ); then break; fi
            sleep .1
        done
        read -r gx gy <<<"$grabber"
        [[ -n $gx && -n $gy ]]
        xdotool mousemove --window "$shell_window" "$gx" "$gy" mousedown 1 \
            mousemove --window "$shell_window" "$gx" "$((gy + 75))" sleep .1 mouseup 1
        # Reopening and launching the app below verifies that the drag dismissed
        # the drawer. If it stayed open this accelerator would close it instead.
        key ctrl+k; sleep .25
    fi
    xdotool type --clearmodifiers --delay 12 'Ferro Navigation'
    key Page_Down Up Return
    for ((i=0; i<80; i++)); do
        [[ -f "$fixture/navigation-$expected.launched" ]] && break
        sleep .1
    done
    [[ -f "$fixture/navigation-$expected.launched" ]] || {
        echo "The $profile launcher did not open keyboard selection $expected" >&2
        import -window "$shell_window" "$output/failed-$profile-selection.png" || :
        exit 1
    }
    key alt+F4
    wait "$shell_pid"; shell_pid=
done
echo 'Verified Spotlight and Material keyboard selection across result pages.'
