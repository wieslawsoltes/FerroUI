#!/usr/bin/env bash
# Compose independent Wayland clients within one FerroUI output window.
# Usage: tests/ferro-compositor.sh /path/to/ferro-desktop output-directory
set -euo pipefail
here=$(cd -- "$(dirname "$0")" && pwd)
if [[ ${1:-} != --on-host ]]; then
    [[ $# == 2 ]] || { echo "Usage: $0 binary output-directory" >&2; exit 2; }
    binary=$(realpath "$1"); [[ -x $binary ]]
    mkdir -p "$2"; output=$(realpath "$2")
    source "$here/wayland-runtime.sh"
    scratch=$(mktemp -d); trap 'remove_wayland_runtime "$scratch"' EXIT
    mkdir -p "$scratch"/{tests,home,output}
    mkdir -m700 "$scratch/runtime"
    install -m755 "$here"/{ferro-compositor.sh,composed-clients.py,composed-launcher.sh} "$scratch/tests/"
    install -m755 "$here/../../../scripts/wayland/sway-run.sh" "$scratch/tests/"
    # wlroots deliberately rejects root. Run only staged test material and
    # the desktop under an unprivileged uid and an isolated bus.
    as_user=()
    if (( EUID == 0 )); then
        install -sm755 "$binary" "$scratch/ferro-desktop"; binary=$scratch/ferro-desktop
        chown -R nobody:"$(id -gn nobody)" "$scratch"
        as_user=(setpriv --reuid=nobody --regid="$(id -g nobody)" --clear-groups)
    fi
    # Compile against the wlr protocol XML used by the workspace's locked
    # Wayland client, or against a system installation of the same protocol.
    shopt -s nullglob
    xml=(${FERRO_TEST_VIRTUAL_POINTER_XML:+"$FERRO_TEST_VIRTUAL_POINTER_XML"}
        /usr/share/wlr-protocols/unstable/wlr-virtual-pointer-unstable-v1.xml
        "${CARGO_HOME:-$HOME/.cargo}"/registry/src/*/wayland-protocols-wlr-*/wlr-protocols/unstable/wlr-virtual-pointer-unstable-v1.xml)
    spec=
    for file in "${xml[@]}"; do [[ ! -f $file ]] || { spec=$file; break; }; done
    [[ -n $spec ]] || { echo 'Install wlr-protocols or set FERRO_TEST_VIRTUAL_POINTER_XML to the virtual-pointer XML.' >&2; exit 2; }
    wayland-scanner client-header "$spec" "$scratch/tests/virtual-pointer.h"
    wayland-scanner private-code "$spec" "$scratch/tests/virtual-pointer.c"
    cc -O2 -I"$scratch/tests" "$here/composed-seat.c" "$scratch/tests/virtual-pointer.c" \
        $(pkg-config --cflags --libs wayland-client) -o "$scratch/tests/composed-seat"
    status=0
    (cd "$scratch" && "${as_user[@]}" env -u WAYLAND_SOCKET -u DISPLAY -u WAYLAND_DISPLAY -u SWAYSOCK \
        -u DBUS_SESSION_BUS_ADDRESS HOME="$scratch/home" XDG_CONFIG_HOME="$scratch/home/config" \
        XDG_CACHE_HOME="$scratch/home/cache" XDG_DATA_HOME="$scratch/home/data" XDG_RUNTIME_DIR="$scratch/runtime" \
        dbus-run-session -- timeout 150s "$scratch/tests/sway-run.sh" --size 1280x800 --log "$scratch/output/sway.log" -- \
        bash "$scratch/tests/ferro-compositor.sh" --on-host "$binary" "$scratch/output") || status=$?
    cp -a "$scratch/output/." "$output/"
    exit "$status"
fi
binary=$2; output=$3
[[ -n ${SWAYSOCK:-} && -n ${XDG_RUNTIME_DIR:-} ]]
scratch=$(mktemp -d "$XDG_RUNTIME_DIR/composition.XXXXXX")
inner=ferro-composition-test
shell= gtk= qt= child= keyboard= modifier= mouse=
cleanup() {
    for pid in "$modifier" "$mouse" "$keyboard" "$child" "$qt" "$gtk" "$shell"; do
        [[ -z $pid ]] || kill "$pid" 2>/dev/null || :
    done
    wait || :
    rm -rf -- "$scratch"
}
trap cleanup EXIT
fail() {
    echo "FerroUI composition: $*" >&2
    for f in shell gtk qt subsurfaces; do tail -n 8 "$output/$f.log" 2>/dev/null >&2 || :; done
    [[ ! -f $scratch/frame.png ]] || cp "$scratch/frame.png" "$output/failure.png"
    exit 1
}
wait_for() {
    local n
    for ((n=0;n<80;n++)); do if "$@"; then return 0; fi; sleep .09; done
    fail "Timed out waiting for $*"
}
capture() { grim -s 1 -o HEADLESS-1 "$scratch/frame.png"; }
pixels() { /usr/bin/python3 "$here/composed-clients.py" "$scratch/frame.png" "$@"; }
has_color() { capture && pixels "$1" "${2:-present}"; }
shot() { cp "$scratch/frame.png" "$output/$1.png"; }
parent_tree() { swaymsg -rt get_tree; }
only_host() {
    parent_tree | jq -e '[..|objects|select(.shell?=="xdg_shell")] |
        length==1 and (.[0].app_id=="org.ferroui.Desktop.Compositor")' >/dev/null
}
inner_client() { exec env -u SWAYSOCK -u DISPLAY -u WAYLAND_SOCKET WAYLAND_DISPLAY="$inner" "$@"; }
pointer() { printf '%s\n' "$*" >&"$pointer_fd"; }
click() { pointer set "$1" "$2"; pointer press button1; pointer release button1; }
source "$here/composed-launcher.sh"
install_composed_launcher "$XDG_DATA_HOME" "$scratch"
"$binary" --platform=wayland --session --compositor=ferro --socket="$inner" >"$output/shell.log" 2>&1 & shell=$!
wait_for test -S "$XDG_RUNTIME_DIR/$inner"
wait_for only_host
swaymsg -rq 'floating_modifier Mod4' >/dev/null
mkfifo "$scratch/pointer"
exec {pointer_fd}<>"$scratch/pointer"
"$here/composed-seat" <"$scratch/pointer" >"$output/mouse.log" 2>&1 & mouse=$!
wait_for grep -qx ready "$output/mouse.log"
wtype -s 145000 & keyboard=$!
test_composed_launcher "$binary" "$scratch" "$inner"

inner_client GDK_BACKEND=wayland /usr/bin/python3 "$here/composed-clients.py" gtk >"$output/gtk.log" 2>&1 & gtk=$!
wait_for has_color d76a3d
inner_client LANG=C.UTF-8 QT_IM_MODULE=compose QT_QPA_PLATFORM=wayland WAYLAND_DEBUG=client /usr/bin/python3 "$here/composed-clients.py" qt >"$output/qt.log" 2>&1 & qt=$!
wait_for has_color 397abb
wait_for has_color d76a3d
only_host || fail 'An external client escaped onto the parent display'
shot overlap
# Raise GTK with a real click in its exposed patch: the former topmost Qt
# swatch is completely underneath it. Move GTK across the viewport with the
# compositor gesture so both native windows can be inspected side by side.
read -r gx gy < <(pixels d76a3d point)
click "$gx" "$gy"
wait_for has_color 397abb absent
wtype -s 70 -M alt -s 3500 -m alt & modifier=$!
sleep .28
pointer set "$gx" "$gy"; pointer press button1
pointer set "$((gx+460))" "$((gy+140))"
# Sway dispatches accumulated cursor motion on a pointer frame. Wait for
# the moved window before finishing the grab, just as a user observes it.
wait_for has_color 397abb
pointer release button1
wait "$modifier"; modifier=
wait_for has_color 397abb
wait_for has_color d76a3d
shot clients
scene_is() { [[ $("$binary" --platform=wayland --current-scene 2>/dev/null) == "$1" ]]; }
"$binary" --platform=wayland --show=settings
wait_for scene_is settings
open_picker() {
    if scene_is appearance; then return 0; fi
    wtype -s 120 -M ctrl -k comma -m ctrl
    return 1
}
wait_for open_picker
wait_for has_color picker light
wtype -k d
wait_for grep -sqx 'dark normal' "$XDG_CONFIG_HOME/ferro-desktop/windows-style"
wait_for has_color picker dark
shot dark-picker
wtype -k l
wait_for grep -sqx 'light normal' "$XDG_CONFIG_HOME/ferro-desktop/windows-style"
wtype -s 60 -k Escape -s 60
wait_for scene_is desktop
# Weston ships a real synchronized wl_subsurface client. Omit its optional
# EGL triangle so the test works on wl_shm-only displays.
inner_client WAYLAND_DEBUG=client weston-subsurfaces --no-triangle --red-mode=1 >"$output/subsurfaces.log" 2>&1 & child=$!
wait_for grep -q '\.get_subsurface(' "$output/subsurfaces.log"
wait_for has_color subsurface
shot subsurfaces
kill "$child"; wait "$child" || :; child=
wait_for has_color subsurface absent

# A keyboard event delivered to the focused external process produces a new
# wl_buffer. Observe the changed client pixels on the outer compositor.
wait_for has_color 397abb
read -r qx qy < <(pixels 397abb point); click "$qx" "$qy"
wtype -s 120 -k u -s 60
wait_for has_color 1ba48e
grep -qx updated "$output/qt.log" || fail 'The outer capture did not originate from the Qt input handler'
shot updated
wtype -s 120 -k F2 -s 100 'Łąé' -s 100 -k Return -s 80
wait_for grep -qx 'label Łąé' "$output/qt.log"
wtype -s 120 -k p -s 60
wait_for has_color f5ce52
grep -q '\.get_popup(' "$output/qt.log" || fail 'The Qt menu did not use xdg_popup'
shot popup
wtype -s 120 -k Escape -s 60
wait_for has_color f5ce52 absent

only_host || fail 'Expected exactly one outer application while all client roles were mapped'
parent_tree >"$output/parent-tree.json"
swaymsg -rq '[app_id="org.ferroui.Desktop.Compositor"] kill' >/dev/null
wait "$shell"; shell=
if grep -E 'Validation Error|VUID-|SYNC-HAZARD|panicked at' "$output/shell.log"; then
    fail 'The output host reported a Vulkan or runtime error'
fi
[[ ! -S $XDG_RUNTIME_DIR/$inner ]] || fail 'Private display survived host shutdown'
echo 'Verified GTK and Qt stacking, compositor move, seat input, updated pixels, xdg_popup and synchronized wl_subsurface inside one FerroUI output.'
