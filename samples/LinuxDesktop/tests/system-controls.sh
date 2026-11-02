#!/usr/bin/env bash
# Exercise the real audio client and system-bus protocols on an isolated X11 session.
# Usage: tests/system-controls.sh target/debug/ferro-desktop output-directory [--software]
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
if [[ ${1:-} != --fixture ]]; then
    binary=$(realpath "${1:?Supply a ferro-desktop binary}")
    output=$(realpath -m "${2:?Supply a capture directory}")
    mkdir -p -- "$output"
    exec timeout 120s env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET xvfb-run -a -s '-screen 0 1440x960x24 -nolisten tcp' \
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
wait_until name_owned org.bluez
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
bluetooth_is() {
    gdbus call --session --dest org.bluez --object-path /org/bluez/hci0 \
        --method org.freedesktop.DBus.Properties.Get org.bluez.Adapter1 Powered | grep -q "$1"
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
# A system-bus write can finish before the corresponding tile is presented.
# Capture both radio edges. Require a settled frame different from the
# opposite radio state, so CI cannot publish the preceding state by chance.
settled_panel() {
    local dest=$1 opposite=${2:-} delta steady i
    rm -f "$fixture/previous.png"
    for ((i=0;i<50;i++)); do
        import -window "$window" "$fixture/candidate.png"
        if [[ -f $fixture/previous.png ]]; then
            steady=$(compare -metric AE "$fixture/previous.png" "$fixture/candidate.png" null: 2>&1 || :)
            delta=1000000
            if [[ -n $opposite ]]; then
                delta=$(compare -metric AE "$opposite" "$fixture/candidate.png" null: 2>&1 || :)
            fi
            if awk -v steady="$steady" -v delta="$delta" 'BEGIN {exit !(steady+0 < 80 && delta+0 > 350)}'; then
                cp "$fixture/candidate.png" "$dest"; return
            fi
        fi
        mv "$fixture/candidate.png" "$fixture/previous.png"
        sleep .12
    done
    cp "$fixture/previous.png" "$output/failed-radio-frame.png"
    echo "Missing settled radio transition for $dest" >&2; return 1
}
bluetooth_cycle() {
    settled_panel "$fixture/before-radio.png"
    press_until b bluetooth_is false
    settled_panel "$fixture/radio-off.png" "$fixture/before-radio.png"
    press_until b bluetooth_is true
    settled_panel "$output/$1.png" "$fixture/radio-off.png"
}
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
grep -q 'Read connection ID at .*/ActiveConnection/1$' "$output/system-bus.log"
if grep -q 'Read connection ID at .*/ActiveConnection/2$' "$output/system-bus.log"; then
    echo "Used the Ethernet profile as the Wi-Fi name" >&2; exit 1
fi
bluetooth_cycle quick-settings
# Discover the native Slider in normal tab order. Optional monitor controls
# and each profile's spatial layout may change the number of preceding tabs.
# Only the volume Slider can advance the audio sink by one keyboard step.
for ((tab=0;tab<16;tab++)); do
    key Tab Right
    for ((i=0;i<6;i++)); do volume_is 46 && break; sleep .08; done
    volume_is 46 && break
done
wait_until volume_is 46
sleep .2; key Right
wait_until volume_is 47
sleep .2; key Left
wait_until volume_is 46
sleep .3
changed_from desktop quick-settings
# The window can present the previous scene after key delivery, especially
# under synchronization validation. Ctrl+Shift+C selects Calendar from the
# active panel directly and can be retried without toggling it closed.
calendar_visible() {
    capture calendar
    if changed_from desktop calendar && changed_from quick-settings calendar; then return 0; fi
    key ctrl+shift+c
    return 1
}
key ctrl+shift+c
wait_until calendar_visible
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
# Exercise both remaining layouts in the same shell and audio session. The
# external D-Bus readbacks prove the active panel, including after a rebuild.
appearance_is() {
    [[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/appearance" 2>/dev/null || :) == "$1" ]]
}
for entry in '2 macos macos-control-center' '3 material material-quick-settings'; do
    read -r choice profile frame <<<"$entry"
    key ctrl+comma
    key "$choice"
    wait_until appearance_is "$profile"
    key ctrl+shift+q
    press_until w wifi_is false
    press_until w wifi_is true
    bluetooth_cycle "$frame"
done
key Escape alt+F4
wait "${children[-1]}"
echo 'Quick Settings changed real PulseAudio mute and volume through buttons and the native slider, toggled Wi-Fi and BlueZ adapter power in all three profiles, read battery state, and navigated Calendar forward and back.'
