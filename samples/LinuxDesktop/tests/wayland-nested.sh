#!/usr/bin/env bash
# Run an actual managed Sway window inside the disposable parent compositor.
set -euo pipefail
[[ ${FERRO_TEST_NESTED:-0} == 1 && ${FERRO_DESKTOP_NESTED:-} == 1 ]] || exit 2
here=$(cd -- "$(dirname -- "$0")" && pwd)
bus_id() {
    gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.GetId
}
if [[ ${1:-} == --child ]]; then
    trap 'swaymsg exit >/dev/null 2>&1 || :' EXIT
    [[ $WAYLAND_DISPLAY != "$FERRO_TEST_PARENT_DISPLAY" && $(bus_id) != "$FERRO_TEST_PARENT_BUS" ]]
    for ((i=0;i<100;i++)); do
        name=$(swaymsg -rt get_outputs | jq -r 'first(.[]|select(.active)|.name) // ""')
        if [[ -n $name && $("$FERRO_DESKTOP_BINARY" --platform=wayland --current-scene 2>/dev/null) == desktop ]] &&
            grim -o "$name" "$FERRO_TEST_NEST_DIR/frame.png"; then
            if (( $(identify -format '%k' "$FERRO_TEST_NEST_DIR/frame.png") > 120 )); then
                mv "$FERRO_TEST_NEST_DIR/frame.png" "$FERRO_TEST_NEST_DIR/nested-desktop.png"
                exit
            fi
        fi
        sleep .075
    done
    echo 'Nested desktop did not present its wallpaper and panel.' >&2
    exit 1
fi
[[ $# == 1 && ${WLR_BACKENDS:-} == headless ]] || exit 2
output=$1
scratch=$(mktemp -d "$XDG_RUNTIME_DIR/nested.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export FERRO_TEST_PARENT_DISPLAY=$WAYLAND_DISPLAY FERRO_TEST_NEST_DIR=$scratch
FERRO_TEST_PARENT_BUS=$(bus_id); export FERRO_TEST_PARENT_BUS
export FERRO_TEST_NEST_HELPER="$here/wayland-nested.sh" FERRO_DESKTOP_SWAY_CONFIG="$scratch/sway.conf"
printf 'include "%s/../session/sway.conf"\nexec "$FERRO_TEST_NEST_HELPER" --child\n' "$here" >"$scratch/sway.conf"
# Preserve the parent's real Wayland socket for Sway's nested output backend.
env -u WLR_WL_OUTPUTS WLR_BACKENDS=wayland WLR_RENDERER=pixman FERRO_DESKTOP_XWAYLAND=0 \
    timeout 24 "$here/../session/ferro-desktop-wayland" --nested >"$output/nested-session.log" 2>&1
test -s "$scratch/nested-desktop.png"
mv "$scratch/nested-desktop.png" "$output/"
# Logout from the nested session must preserve both the parent bus and its shell.
[[ $(bus_id) == "$FERRO_TEST_PARENT_BUS" && $("$FERRO_DESKTOP_BINARY" --platform=wayland --current-scene) == desktop ]]
echo 'Verified nested Sway window, isolated shell and bus, compositor pixels and parent survival.'
