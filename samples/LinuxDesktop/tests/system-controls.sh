#!/usr/bin/env bash
# Exercise the real audio client and both system-bus protocols on an isolated X11 session.
# Usage: tests/system-controls.sh target/debug/ferro-desktop output-directory [--software]
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
if [[ ${1:-} != --fixture ]]; then
    binary=$(realpath "${1:?Supply a ferro-desktop binary}")
    output=$(realpath -m "${2:?Supply a capture directory}")
    mkdir -p -- "$output"
    exec timeout 120s xvfb-run -a -s '-screen 0 1440x960x24 -nolisten tcp' \
        dbus-run-session -- bash -c 'export FERRO_SYSTEM_FIXTURE_DISPLAY=$DISPLAY; exec "$@"' bash \
        "$here/system-controls.sh" --fixture "$binary" "$output" "${@:3}"
fi
[[ -n ${DISPLAY:-} && $DISPLAY == "${FERRO_SYSTEM_FIXTURE_DISPLAY:-}" ]]
binary=$2 output=$3
shift 3
fixture=$(mktemp -d)
children=()
cleanup() {
    if (( ${#children[@]} )); then kill "${children[@]}" 2>/dev/null || :; fi
    wait || :
    rm -rf -- "$fixture"
}
trap cleanup EXIT
export LC_ALL=C
export XDG_RUNTIME_DIR="$fixture/run" XDG_CONFIG_HOME="$fixture/config" XDG_CACHE_HOME="$fixture/cache" XDG_DATA_HOME="$fixture/data"
mkdir -m 700 "$XDG_RUNTIME_DIR" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_DATA_HOME"
# No request can reach the user's real system bus, audio output or PipeWire graph.
export DBUS_SYSTEM_BUS_ADDRESS="$DBUS_SESSION_BUS_ADDRESS"
export PULSE_RUNTIME_PATH="$fixture/pulse" PULSE_STATE_PATH="$fixture/pulse-state" PULSE_COOKIE="$fixture/cookie"
export PULSE_SERVER="unix:$fixture/pulse/native" PIPEWIRE_REMOTE=ferro-test-absent
mkdir -m 700 "$PULSE_RUNTIME_PATH" "$PULSE_STATE_PATH"
cc -std=c11 -Wall -Werror "$here/system-bus-fixture.c" -o "$fixture/system-bus-fixture" $(pkg-config --cflags --libs gio-2.0)
"$fixture/system-bus-fixture" >"$output/system-bus.log" 2>&1 & children+=("$!")
pulseaudio -n --daemonize=no --exit-idle-time=-1 --use-pid-file=no --log-target=stderr \
    -L "module-native-protocol-unix socket=$PULSE_RUNTIME_PATH/native auth-anonymous=1" \
    -L 'module-null-sink sink_name=ferro_fixture' >"$output/pulse.log" 2>&1 & children+=("$!")

wait_until() {
    for ((i=0; i<80; i++)); do if "$@"; then return; fi; sleep .1; done
    echo "Timed out waiting for $*" >&2; return 1
}
ready_audio() { pactl info >/dev/null 2>&1; }
name_owned() {
    gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
        --method org.freedesktop.DBus.NameHasOwner "$1" | grep -q 'true'
}
wait_until ready_audio
wait_until name_owned org.freedesktop.NetworkManager
wait_until name_owned org.freedesktop.UPower
pactl set-default-sink ferro_fixture
pactl set-sink-mute ferro_fixture 0
pactl set-sink-volume ferro_fixture 45%

openbox --sm-disable >"$output/openbox.log" 2>&1 & children+=("$!")
"$binary" --theme=windows "$@" >"$output/desktop.log" 2>&1 & children+=("$!")
window=$(timeout 30s xdotool search --sync --onlyvisible --name '^Ferro Desktop$' | head -1)
xdotool windowactivate --sync "$window" mousemove 1410 920
key() { xdotool key --clearmodifiers "$@"; }
volume_is() { pactl get-sink-volume ferro_fixture | grep -q " $1% "; }
mute_is() { [[ $(pactl get-sink-mute ferro_fixture) == "Mute: $1" ]]; }
wifi_is() {
    gdbus call --session --dest org.freedesktop.NetworkManager \
        --object-path /org/freedesktop/NetworkManager --method org.freedesktop.DBus.Properties.Get \
        org.freedesktop.NetworkManager WirelessEnabled | grep -q "$1"
}
# Surface readbacks can settle after the asynchronous platform write has completed.
# Poll the independently observed device state and use bounded retries for the key press.
press_until() {
    local stroke=$1; shift
    for attempt in 1 2 3; do
        if "$@"; then return; fi
        key "$stroke"
        for ((i=0; i<16; i++)); do if "$@"; then sleep .2; return; fi; sleep .1; done
    done
    echo "The shell failed to apply $stroke ($*)" >&2
    return 1
}
capture() { import -window "$window" "$output/$1.png"; }
difference() { compare -metric AE "$output/$1.png" "$output/$2.png" null: 2>&1 || :; }
changed_from() {
    awk -v pixels="$(difference "$1" "$2")" -v minimum="${3:-10000}" 'BEGIN { exit !(pixels > minimum) }'
}
painted() {
    capture desktop
    local colors
    colors=$(identify -format '%k' "$output/desktop.png")
    (( colors > 60 ))
}
wait_until painted
key ctrl+shift+q
wait_until grep -q 'Read org.freedesktop.UPower.Device.State' "$output/system-bus.log"
sleep .2
press_until m mute_is yes
press_until m mute_is no
press_until plus volume_is 50
press_until minus volume_is 45
press_until w wifi_is false
press_until w wifi_is true
wait_until grep -q 'Set WirelessEnabled=1' "$output/system-bus.log"
# From Close, traverse the three sound buttons to the native OutputVolume
# slider. Its arrows step by one percent. Focus must survive the first
# asynchronous device write and refresh for the second edit to reach it.
key Tab Tab Tab Tab Right
wait_until volume_is 46
sleep .2; key Right
wait_until volume_is 47
sleep .2; key Left
wait_until volume_is 46
sleep .3
capture quick-settings
changed_from desktop quick-settings
key Escape ctrl+shift+c
sleep .6
capture calendar
changed_from desktop calendar
changed_from quick-settings calendar
# Move focus from the panel close button into Calendar. Page Down and Page Up
# must repaint the month grid, then restore the previous month's rendering.
key Tab; sleep .25; capture calendar-focused
key Page_Down
next_month_visible() { capture calendar-next; changed_from calendar-focused calendar-next 1500; }
wait_until next_month_visible
key Page_Up
original_month_visible() {
    capture calendar-return
    awk -v back="$(difference calendar-focused calendar-return)" \
        -v forward="$(difference calendar-focused calendar-next)" 'BEGIN { exit !(back < forward / 2) }'
}
wait_until original_month_visible
key Escape alt+F4
wait "${children[-1]}"
echo 'Quick Settings changed real PulseAudio mute and volume through buttons and the native slider, toggled the private network service, read battery state, and navigated Calendar forward and back.'
