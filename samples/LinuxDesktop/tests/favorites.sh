#!/usr/bin/env bash
# Drive pin, reorder, restart and launch through the real X11 shell.
# Usage: tests/favorites.sh path/to/ferro-desktop [output-directory] [--software]
set -euo pipefail
if [[ ${1:-} != --fixture ]]; then
    binary=$(realpath "${1:?Supply a ferro-desktop binary}")
    output=$(realpath -m "${2:-/tmp/ferro-desktop-favorites}")
    mkdir -p "$output"
    exec timeout 100s xvfb-run -a -s '-screen 0 1440x960x24 -nolisten tcp' \
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
    "$binary" --theme=windows "$@" >>"$output/favorites.log" 2>&1 &
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
start_shell "$@"
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
start_shell "$@"
key ctrl+k; sleep .25; key ctrl+shift+p Return
for ((i=0; i<80; i++)); do [[ -f "$fixture/beacon.launched" ]] && break; sleep .1; done
[[ -f "$fixture/beacon.launched" && ! -f "$fixture/atlas.launched" ]]
key ctrl+k; sleep .2
search Beacon; key ctrl+p; pins_are '["ferro-atlas.desktop"]'
echo 'Verified GIO launch from restored pins, reordering and removal.'
