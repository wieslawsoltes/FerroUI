#!/usr/bin/env bash
# Run the shipped display-manager session on Sway's headless backend. No host bus/display.
set -euo pipefail
script=$(realpath "$0")
here=$(dirname "$script")
mode=native
if [[ ${1:-} == --xwayland ]]; then mode=mixed; shift; fi
if [[ ${1:-} != --private ]]; then
    if (( $# != 2 )); then echo "Usage: $0 [--xwayland] /path/to/ferro-desktop output-dir" >&2; exit 2; fi
    binary=$(realpath "$1")
    [[ -x $binary ]] || { echo "Missing executable: $binary" >&2; exit 2; }
    mkdir -p "$2"; output=$(realpath "$2")
    scratch=$(mktemp -d); trap 'rm -rf -- "$scratch"' EXIT
    # Sway deliberately refuses uid 0. Stage only the executable and session/test files
    # when run from a root-only checkout. Never start a compositor or GTK as root.
    if (( EUID == 0 )); then
        mkdir -p "$scratch"/{tests,session,res}
        install -m755 "$script" "$scratch/tests/wayland-session.sh"
        if [[ $mode == native ]]; then
            install -m644 "$here/wayland-appearances.sh" "$here/wayland-notifications.sh" "$scratch/tests/"
        fi
        install -m644 "$here/wayland-screencast.py" "$scratch/tests/"
        if [[ ${FERRO_TEST_BENCHMARK:-0} == 1 ]]; then install -m644 "$here/wayland-benchmark.py" "$scratch/tests/"; fi
        if [[ ${FERRO_TEST_QT:-0} == 1 ]]; then install -m755 "$here/wayland-qt-client.sh" "$scratch/tests/"; fi
        if [[ ${FERRO_TEST_OUTPUT_HOTPLUG:-0} == 1 ]]; then install -m755 "$here/wayland-output-hotplug.sh" "$scratch/tests/"; fi
        cp -a "$here/../session/." "$scratch/session/"
        if [[ ${RUST_BACKTRACE:-0} == 0 ]]; then
            install -s -m755 "$binary" "$scratch/ferro-desktop"
        else
            install -m755 "$binary" "$scratch/ferro-desktop"
        fi
        if [[ -n ${FERRO_TEST_WLR_PORTAL:-} ]]; then
            install -m755 "$FERRO_TEST_WLR_PORTAL" "$scratch/test-wlr-portal"
            export FERRO_TEST_WLR_PORTAL=$scratch/test-wlr-portal
        fi
        chown -R nobody:"$(id -gn nobody)" "$scratch"
        args=(); [[ $mode == native ]] || args+=(--xwayland)
        status=0
        (cd "$scratch" && setpriv --reuid=nobody --regid="$(id -g nobody)" --clear-groups \
            -- "$scratch/tests/wayland-session.sh" "${args[@]}" "$scratch/ferro-desktop" "$scratch/res") || status=$?
        cp -R -- "$scratch/res/." "$output/"
        exit "$status"
    fi
    mkdir -m700 "$scratch"/{runtime,home,config,cache,data}
    # The installed desktop-specific portal preferences are also tested in isolation.
    mkdir -p "$scratch/data/xdg-desktop-portal"
    cp "$here/../session/ferrodesktop-portals.conf" "$scratch/data/xdg-desktop-portal/"
    if [[ -n ${FERRO_TEST_WLR_PORTAL:-} ]]; then
        # The private bus activates the requested backend on first use, with
        # the environment the supervisor imported after the Sway handoff.
        ln -s -- "$(realpath -- "$FERRO_TEST_WLR_PORTAL")" "$scratch/test-wlr-portal"
        mkdir -p "$scratch/data/dbus-1/services"
        printf '[D-BUS Service]\nName=org.freedesktop.impl.portal.desktop.wlr\nExec="%s/test-wlr-portal" -l INFO\n' "$scratch" \
            > "$scratch/data/dbus-1/services/org.freedesktop.impl.portal.desktop.wlr.service"
    fi
    env -u DISPLAY -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u SWAYSOCK -u DBUS_SESSION_BUS_ADDRESS \
        -u DESKTOP_STARTUP_ID -u XDG_ACTIVATION_TOKEN HOME="$scratch/home" XDG_RUNTIME_DIR="$scratch/runtime" \
        XDG_CONFIG_HOME="$scratch/config" XDG_CACHE_HOME="$scratch/cache" XDG_DATA_HOME="$scratch/data" \
        XDG_DATA_DIRS="$scratch/data:/usr/local/share:/usr/share" FERRO_DESKTOP_NESTED=1 \
        dbus-run-session -- timeout 240s "$script" --private "$binary" "$output" "$mode"
    exit
fi
shift; binary=$1; output=$2; mode=$3
[[ -n ${DBUS_SESSION_BUS_ADDRESS:-} && -z ${WAYLAND_DISPLAY:-} && -z ${DISPLAY:-} && -z ${SWAYSOCK:-} &&
    ${FERRO_DESKTOP_NESTED:-} == 1 && $(stat -c %u "$XDG_RUNTIME_DIR") == "$EUID" ]]
export WLR_BACKENDS=headless WLR_HEADLESS_OUTPUTS=2 WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
export FERRO_DESKTOP_BINARY="$binary" XDG_SESSION_TYPE=wayland XDG_SESSION_DESKTOP=ferro-desktop-wayland XDG_CURRENT_DESKTOP=FerroDesktop:sway
if [[ $mode == mixed ]]; then export FERRO_DESKTOP_XWAYLAND=1; else export FERRO_DESKTOP_XWAYLAND=0; fi
export FERRO_DESKTOP_SWAY_CONFIG="$XDG_CONFIG_HOME/test-sway.conf"
cat "$here/../session/sway.conf" > "$FERRO_DESKTOP_SWAY_CONFIG"
cat >> "$FERRO_DESKTOP_SWAY_CONFIG" <<'CONFIG'
output HEADLESS-1 mode 1280x800 pos 0 0 scale 1
output HEADLESS-2 mode 960x720 pos 1280 0 scale 1.5
workspace 1 output HEADLESS-1
workspace 2 output HEADLESS-2
CONFIG
supervisor= terminal= gtk= pw= wp= damage= keyboard=
cleanup() {
    for pid in "$keyboard" "$damage" "$gtk" "$terminal" "$supervisor" "$wp" "$pw"; do [[ -z $pid ]] || kill "$pid" 2>/dev/null || :; done
    wait || :
}
trap cleanup EXIT
fail() { echo "$*" >&2; tail -n 40 "$output/session.log" >&2 || :; exit 1; }
wait_for() { local i; for ((i=0;i<130;i++)); do if "$@"; then return; fi; sleep .075; done; fail "Timed out waiting for $*"; }
if [[ ${FERRO_TEST_PORTALS:-0} == 1 || ${FERRO_TEST_SCREENCAST:-0} == 1 ]]; then
    # Only this throwaway session preselects a monitor. Login sessions use
    # the existing per-user portal policy and interactive output chooser.
    mkdir -p "$XDG_CONFIG_HOME/xdg-desktop-portal-wlr"
    printf '[screencast]\nchooser_type=none\noutput_name=HEADLESS-1\n' > "$XDG_CONFIG_HOME/xdg-desktop-portal-wlr/config"
    pipewire > "$output/pipewire.log" 2>&1 & pw=$!
    wait_for test -S "$XDG_RUNTIME_DIR/pipewire-0"
    wireplumber > "$output/wireplumber.log" 2>&1 & wp=$!
fi
"$here/../session/ferro-desktop-wayland" > "$output/session.log" 2>&1 &
supervisor=$!
shopt -s nullglob
for ((i=0;i<200;i++)); do
    for socket in "$XDG_RUNTIME_DIR"/sway-ipc.*.sock; do [[ ! -S $socket ]] || export SWAYSOCK=$socket; done
    for socket in "$XDG_RUNTIME_DIR"/wayland-*; do [[ ! -S $socket ]] || export WAYLAND_DISPLAY=${socket##*/}; done
    [[ -z ${SWAYSOCK:-} || -z ${WAYLAND_DISPLAY:-} ]] || break
    kill -0 "$supervisor" || fail "Compositor exited at startup"
    sleep .05
done
[[ -n ${SWAYSOCK:-} && -n ${WAYLAND_DISPLAY:-} ]] || fail "Missing compositor sockets"
# Hotplugging the last keyboard changes wl_seat capabilities and removes focus.
# Keep a virtual device on seat0 while the short-lived wtype clients send events.
wtype -s 240000 >"$output/keyboard.log" 2>&1 & keyboard=$!
# An activated shell scene is published before the compositor round-trips its
# layer mapping and keyboard focus. Keep each device mapped across dispatch.
wtype() { command wtype -s 100 "$@" -s 60; }
scene() { "$binary" --platform=wayland --current-scene 2>/dev/null; }
is_scene() { [[ $(scene) == "$1" ]]; }
tree() { swaymsg -r -t get_tree; }
focused() { tree | jq -e --arg app "$1" 'any(.. | objects; .app_id? == $app and .focused? == true)' >/dev/null; }
output_names() { swaymsg -rt get_outputs | jq -r '[.[] | select(.active)] | length'; }
capture() {
    # Wait for a nonempty and approximately settled frame, not just a mapped surface.
    local i name=$1 out=${2:-HEADLESS-1} frame="$XDG_RUNTIME_DIR/frame.png" last="$XDG_RUNTIME_DIR/previous.png"
    rm -f "$last"
    for ((i=0;i<35;i++)); do
        grim -o "$out" "$frame"
        if [[ $(identify -format '%k' "$frame") -gt 60 && -f $last ]]; then
            local changed
            changed=$(compare -metric AE "$last" "$frame" null: 2>&1 || :)
            if awk -v d="$changed" 'BEGIN {exit !(d+0 < 500)}'; then cp "$frame" "$output/$name.png"; return; fi
        fi
        mv "$frame" "$last"; sleep .12
    done
    cp "$last" "$output/failed-$name.png"; fail "$name did not produce a stable rendered Wayland frame"
}
wait_for is_scene desktop
# Explicit inherited sockets take priority over a conflicting named display.
python3 - "$binary" <<'PYTHON'
import os, socket, subprocess, sys
with socket.socket(socket.AF_UNIX) as peer:
    peer.connect(os.path.join(os.environ["XDG_RUNTIME_DIR"], os.environ["WAYLAND_DISPLAY"]))
    env = dict(os.environ, WAYLAND_SOCKET=str(peer.fileno()), WAYLAND_DISPLAY="wrong-display")
    current = subprocess.check_output([sys.argv[1], "--platform=wayland", "--current-scene"],
                                     env=env, pass_fds=(peer.fileno(),), text=True)
    assert current.strip() == "desktop"
PYTHON
printf -v bus_name 'org.ferroui.Desktop.Wayland.d%x_%x' \
    "$(stat -c %d "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY")" "$(stat -c %i "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY")"
duplicate_status=0
timeout 8 "$binary" --platform=wayland --session >"$output/duplicate.log" 2>&1 || duplicate_status=$?
[[ $duplicate_status == 1 ]] && grep -q 'already owns this Wayland display' "$output/duplicate.log" ||
    fail "Duplicate managed shell returned $duplicate_status instead of reporting the existing owner"
wait_for is_scene desktop
swaymsg -rt get_outputs > "$output/outputs.json"
[[ $(output_names) == 2 ]] || fail 'Expected two active Wayland outputs'
wayland-info > "$output/protocols.txt"
grep -q zwlr_layer_shell_v1 "$output/protocols.txt"
# xdg-shell only lists ordinary clients. Native shell layers do not become tiled windows.
[[ $(tree | jq '[..|objects|select(.shell?=="xdg_shell")] | length') == 0 ]] || fail 'Managed shell incorrectly appeared as a toplevel'
swaymsg 'focus output HEADLESS-1' >/dev/null
capture desktop
# The full shell background, including wallpaper, must cover the otherwise
# solid compositor clear color. Sample well outside taskbar and shortcut text.
wallpaper_colors=$(convert "$output/desktop.png" -crop 600x400+320+145 +repage -format '%k' info:)
[[ $wallpaper_colors -gt 100 ]] || fail "Wallpaper layer did not cover the compositor background"
if [[ ${FERRO_TEST_BENCHMARK:-0} == 1 ]]; then
    /usr/bin/python3 "$here/wayland-benchmark.py" "$binary" "$output" "$bus_name"
fi
foot --app-id=org.ferroui.WaylandClient --title='Terminal — native Wayland' sh -c \
    'printf "\n   Ferro Desktop\n   Native Wayland terminal\n\n   compositor: Sway\n   graphics:   Skia Graphite / Vulkan\n"; exec sleep 120' > "$output/terminal.log" 2>&1 &
terminal=$!
wait_for focused org.ferroui.WaylandClient
swaymsg '[app_id="org.ferroui.WaylandClient"] floating enable, resize set 600 350, move position 100 145' >/dev/null
capture native-client
if [[ $mode == native ]]; then
    source "$here/wayland-notifications.sh"
    exercise_notifications
fi
# Keyboard bindings are delivered by Sway's virtual keyboard protocol.
for spec in 'space applications' 'a settings' 'n notifications' 'Tab workspaces'; do
    read -r key name <<< "$spec"
    wtype -M logo -k "$key" -m logo
    wait_for is_scene "$name"
    capture "$name"
    wtype -k Escape
    wait_for is_scene desktop
    wait_for focused org.ferroui.WaylandClient
done
if [[ $mode == native ]]; then
    source "$here/wayland-appearances.sh"
    exercise_appearances
fi
# The same command endpoint can be driven directly without constructing an application window.
"$binary" --platform=wayland --show=applications
wait_for is_scene applications
wtype -k Escape; wait_for is_scene desktop
# Clipboard transfer between independent Wayland clients with X11 absent.
printf 'Ferro Desktop / Wayland' | wl-copy
[[ $(wl-paste -n) == 'Ferro Desktop / Wayland' ]]
# A GTK application on the Wayland GDK backend coexists with the terminal.
GDK_BACKEND=wayland gtk3-demo > "$output/gtk.log" 2>&1 &
gtk=$!
has_gtk() { tree | jq -e 'any(.. | objects; .shell?=="xdg_shell" and ((.app_id? // "") | test("gtk";"i")))' >/dev/null; }
wait_for has_gtk
if [[ ${FERRO_TEST_QT:-0} == 1 ]]; then "$here/wayland-qt-client.sh" "$output"; fi
swaymsg '[app_id="org.ferroui.WaylandClient"] move container to workspace 2' >/dev/null
swaymsg 'workspace number 2' >/dev/null
wait_for focused org.ferroui.WaylandClient
tree > "$output/clients.json"
if [[ $mode == mixed ]]; then
    swaymsg 'exec xmessage -title Ferro-Xwayland -buttons OK Legacy-client' >/dev/null
    has_legacy() { tree | jq -e 'any(..|objects; .shell?=="xwayland" and .name?=="Ferro-Xwayland")' >/dev/null; }
    wait_for has_legacy
    tree > "$output/mixed-clients.json"
fi
if [[ ${FERRO_TEST_PORTALS:-0} == 1 || ${FERRO_TEST_SCREENCAST:-0} == 1 ]]; then
    stream=()
    if [[ ${FERRO_TEST_SCREENCAST:-0} == 1 ]]; then
        stream+=(--stream)
        # Damage HEADLESS-1 while the media consumer connects. A completely idle
        # output need not deliver further damage-driven screencopy frames.
        swaymsg 'focus output HEADLESS-1' >/dev/null
        (for ((n=0;n<100;n++)); do wtype -k Tab || exit; sleep .15; done) &
        damage=$!
    fi
    FERRO_TEST_PRIVATE_PORTAL=1 /usr/bin/python3 "$here/wayland-screencast.py" "$output" "${stream[@]}"
    [[ $(identify -format '%k' "$output/portal-screenshot.png") -gt 60 ]] || fail "Empty portal screenshot"
    if (( ${#stream[@]} )); then
        [[ $(identify -format '%k' "$output/portal-capture.png") -gt 60 ]] || fail "Empty PipeWire frame"
        kill "$damage" 2>/dev/null || :; wait "$damage" || :; damage=
    fi
fi
if [[ ${FERRO_TEST_OUTPUT_HOTPLUG:-0} == 1 ]]; then
    "$here/wayland-output-hotplug.sh" "$binary" "$output"
fi
if [[ ${FERRO_TEST_LOCK:-0} == 1 ]]; then
    # Only run on this script's throwaway compositor. A successful --daemonize
    # returns after the compositor confirms all outputs are covered.
    before_lock=$(scene)
    WAYLAND_DEBUG=1 timeout 10 "$here/../session/ferro-desktop-wayland" --lock >"$output/lock.log" 2>&1
    grep -Eq 'ext_session_lock_v1[@#][0-9]+\.locked\(' "$output/lock.log" || fail "Missing compositor lock acknowledgement"
    wtype -M logo -k a -m logo
    # A global binding spawns its own asynchronous command client.
    for ((i=0;i<8;i++)); do
        [[ $(scene) == "$before_lock" ]] || fail "Global launcher binding escaped the session lock"
        sleep .075
    done
fi
# Graceful compositor shutdown must terminate the managed login supervisor.
swaymsg exit >/dev/null || :
wait "$supervisor" || :
supervisor=
gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.NameHasOwner "$bus_name" | grep -q false || fail 'Stale shell control service survived logout'
echo "Verified native Wayland login, independent GTK/terminal clients, four global scenes, keyboard focus, clipboard and output/workspace movement ($mode)."
