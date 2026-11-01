#!/usr/bin/env bash
# Native xdg-shell preview on Weston 13+, independent of the managed Sway login.
# Requires weston (with weston-screenshooter), wayland-utils, dbus and ImageMagick.
set -euo pipefail
if [[ ${1:-} != --private ]]; then
    [[ $# == 2 ]] || { echo "Usage: $0 /path/to/ferro-desktop output-dir" >&2; exit 2; }
    binary=$(realpath "$1"); [[ -x $binary ]]
    mkdir -p "$2"; output=$(realpath "$2")
    scratch=$(mktemp -d); trap 'rm -rf -- "$scratch"' EXIT
    mkdir -m700 "$scratch"/{runtime,home,config,cache}
    env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u SWAYSOCK -u DBUS_SESSION_BUS_ADDRESS \
        HOME="$scratch/home" XDG_RUNTIME_DIR="$scratch/runtime" XDG_CONFIG_HOME="$scratch/config" \
        XDG_CACHE_HOME="$scratch/cache" XDG_DATA_HOME="$scratch/home/data" \
        dbus-run-session -- timeout 100s bash "$0" --private "$binary" "$output"
    exit
fi
binary=$2; output=$3
export WAYLAND_DISPLAY=ferro-weston-test
desktop= weston_pid=
cleanup() {
    for pid in "$desktop" "$weston_pid"; do [[ -z $pid ]] || kill "$pid" 2>/dev/null || :; done
    wait || :
}
trap cleanup EXIT
weston --no-config --backend=headless --renderer=pixman --shell=kiosk --width=960 --height=640 \
    --socket="$WAYLAND_DISPLAY" --idle-time=0 --debug --log="$output/weston.log" &
weston_pid=$!
for ((n=0;n<100;n++)); do
    [[ ! -S $XDG_RUNTIME_DIR/$WAYLAND_DISPLAY ]] || break
    kill -0 "$weston_pid"; sleep .05
done
wayland-info > "$output/protocols.txt"
grep -q "'xdg_wm_base'" "$output/protocols.txt"
if grep -q "'zwlr_layer_shell_v1'" "$output/protocols.txt"; then echo "Unexpected layer-shell global" >&2; exit 1; fi
capture() {
    local path=$1 dir file
    dir=$(mktemp -d "$XDG_RUNTIME_DIR/capture.XXXXXX")
    (cd "$dir" && timeout 5s weston-screenshooter)
    for file in "$dir"/wayland-screenshot-*.png; do
        if [[ -f $file ]]; then mv "$file" "$path"; rmdir "$dir"; return; fi
    done
    echo "Weston produced no capture" >&2; return 1
}
capture "$XDG_RUNTIME_DIR/empty.png"
for mode in graphite software; do
    args=(); [[ $mode == graphite ]] || args+=(--software)
    trace="$output/$mode.log"
    WAYLAND_DEBUG=client "$binary" --platform=wayland --theme=windows --scene=applications "${args[@]}" > "$trace" 2>&1 &
    desktop=$!
    frame="$output/$mode.png"
    changed=0
    for ((n=0;n<60;n++)); do
        kill -0 "$desktop"
        if grep -Eq 'wl_surface[@#][0-9]+\.attach\(wl_buffer' "$trace"; then
            capture "$frame"
            # The kiosk shell fills the entire output. More than half must change from its idle frame.
            changed=$(compare -metric AE "$XDG_RUNTIME_DIR/empty.png" "$frame" null: 2>&1) || [[ $? == 1 ]]
            if awk -v pixels="$changed" 'BEGIN { exit !(pixels > 960*640/2) }'; then break; fi
        fi
        sleep .15
    done
    grep -Eq 'xdg_surface[@#][0-9]+\.get_toplevel' "$trace"
    grep -Eq 'xdg_toplevel[@#][0-9]+\.configure\(960, 640' "$trace"
    grep -Eq 'xdg_surface[@#][0-9]+\.set_window_geometry\(0, 0, 960, 640\)' "$trace"
    expected='Skia Graphite Vulkan'; [[ $mode == graphite ]] || expected='Skia CPU raster'
    grep -iq "$expected" "$trace"
    [[ $(identify -format '%wx%h' "$frame") == 960x640 ]]
    colors=$(identify -format '%k' "$frame")
    (( colors > 64 ))
    awk -v pixels="$changed" 'BEGIN { exit !(pixels > 960*640/2) }'
    echo "Weston $mode: xdg-shell configured at 960x640, $changed pixels changed, $colors colors"
    kill "$desktop"; wait "$desktop" || :; desktop=
done
