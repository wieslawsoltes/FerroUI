#!/usr/bin/env bash
# Verify the installed global shortcuts over a maximized, independent X11 application.
# Usage: tests/shortcuts.sh path/to/ferro-desktop output-directory
set -euo pipefail
[[ $# == 2 ]] || { echo "Usage: $0 binary output-directory" >&2; exit 2; }
here=$(cd -- "$(dirname -- "$0")" && pwd)
binary=$(realpath "$1"); mkdir -p "$2"; output=$(realpath "$2")
if [[ -z ${FERRO_SHORTCUT_PRIVATE_DISPLAY:-} || $FERRO_SHORTCUT_PRIVATE_DISPLAY != "${DISPLAY:-}" ]]; then
    exec timeout 90s env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET xvfb-run -a -s '-screen 0 1280x800x24 -nolisten tcp' dbus-run-session -- \
        bash -c 'export FERRO_SHORTCUT_PRIVATE_DISPLAY=$DISPLAY; exec "$@"' bash "$here/shortcuts.sh" "$binary" "$output"
fi
scratch=$(mktemp -d)
export FERRO_DESKTOP_BINARY=$binary XDG_RUNTIME_DIR=$scratch/run XDG_CONFIG_HOME=$scratch/config XDG_CACHE_HOME=$scratch/cache
mkdir -m 700 "$XDG_RUNTIME_DIR" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
preview= supervisor= client=
cleanup() {
    for pid in "$preview" "$supervisor" "$client"; do
        [[ -z $pid ]] || kill "$pid" 2>/dev/null || :
    done
    wait || :
    rm -rf -- "$scratch"
}
trap cleanup EXIT

# Preview mode must not register the shell endpoint, and an unknown command is never dispatched.
"$binary" --software >"$output/preview.log" 2>&1 & preview=$!
timeout 15s xdotool search --sync --onlyvisible --all --pid "$preview" --name '^Ferro Desktop$' >/dev/null
if "$binary" --show=settings >"$output/no-session.log" 2>&1; then exit 1; fi
grep -q 'No Ferro Desktop session' "$output/no-session.log"
kill "$preview"; wait "$preview" || :; preview=

"$here/../session/ferro-desktop-session" >"$output/session.log" 2>&1 & supervisor=$!
shell=$(timeout 20s xdotool search --sync --onlyvisible --name '^Ferro Desktop$' | head -1)
if timeout 12s "$binary" --session --software >"$output/duplicate.log" 2>&1; then exit 1; fi
grep -q 'already owns this X11 screen' "$output/duplicate.log"

xdotool set_desktop 1
for ((n=0;n<100;n++)); do
    [[ $(xdotool get_desktop) == 1 ]] && break
    sleep .05
done
[[ $(xdotool get_desktop) == 1 ]]
# The client has no fixed-size hints; it can be maximized and tracked independently of the shell.
python3 - <<'PY' &
import ctypes as C, ctypes.util, signal
x = C.CDLL(ctypes.util.find_library("X11"))
def fn(name, result, *arguments):
    f = getattr(x, name)
    f.restype, f.argtypes = result, arguments
    return f
ptr, xid, i, u = C.c_void_p, C.c_ulong, C.c_int, C.c_uint
display = fn("XOpenDisplay", ptr, C.c_char_p)(None)
root = fn("XDefaultRootWindow", xid, ptr)(display)
window = fn("XCreateSimpleWindow", xid, ptr, xid, i, i, u, u, u, xid, xid)(display, root, 40, 40, 320, 200, 0, 0, 0x25394A)
fn("XStoreName", i, ptr, xid, C.c_char_p)(display, window, b"Global shortcut client")
fn("XMapWindow", i, ptr, xid)(display, window)
fn("XSync", i, ptr, i)(display, 0)
signal.pause()
PY
client=$!
external=$(timeout 10s xdotool search --sync --onlyvisible --name '^Global shortcut client$' | head -1)
xdotool windowactivate --sync "$external" key --clearmodifiers super+Up
await_focus() {
    for ((n=0;n<100;n++)); do
        if [[ $(xdotool getactivewindow 2>/dev/null || :) == "$1" && $(xdotool getwindowfocus 2>/dev/null || :) == "$1" && $(xdotool get_desktop) == 1 ]]; then return; fi
        sleep .05
    done
    echo "Expected focus $1 on workspace 2. Active $(xdotool getactivewindow), input focus $(xdotool getwindowfocus), workspace $(xdotool get_desktop)." >&2; exit 1
}
await_scene() {
    local current=
    for ((n=0;n<100;n++)); do
        current=$("$binary" --current-scene 2>/dev/null || :)
        [[ $current == "$1" ]] && return
        sleep .05
    done
    echo "Expected visible scene $1, observed $current." >&2; exit 1
}
await_focus "$external"
for ((n=0;n<100;n++)); do
    dimensions=$(xdotool getwindowgeometry --shell "$external")
    if awk -F= '/^WIDTH/ {w=$2} /^HEIGHT/ {h=$2} END {exit !(w>=1270 && h>=710)}' <<< "$dimensions"; then break; fi
    sleep .05
done
awk -F= '/^WIDTH/ {w=$2} /^HEIGHT/ {h=$2} END {exit !(w>=1270 && h>=710)}' <<< "$dimensions"

names=(settings notifications workspaces applications)
global_keys=(super+a super+n super+Tab super+space)
capture() {
    local previous=$scratch/previous.png frame=$scratch/frame.png changed colors attempt
    import -window root "$previous"
    for ((attempt=0;attempt<60;attempt++)); do
        sleep .1
        import -window root "$frame"
        colors=$(identify -format '%k' "$frame")
        changed=$(compare -metric AE "$previous" "$frame" null: 2>&1 || :)
        if (( colors > 60 )) && awk -v n="$changed" 'BEGIN {exit !(n+0 < 400)}'; then
            cp "$frame" "$1"; return
        fi
        mv "$frame" "$previous"
    done
    echo "Scene did not settle for capture $1." >&2; exit 1
}
for i in 0 1 2 3; do
    name=${names[$i]}; key=${global_keys[$i]}
    xdotool key --clearmodifiers "$key"
    await_focus "$shell"
    await_scene "$name"
    capture "$output/$name.png"
    # Each published scene must produce distinct pixels. Device discovery, focus decoration
    # and theme transitions can change details between isolated runs.
    for ((other=0;other<i;other++)); do
        difference=$(compare -metric AE "$output/${names[$other]}.png" "$output/$name.png" null: 2>&1 || :)
        awk -v n="$difference" 'BEGIN {exit !(n+0 > 6000)}' || { echo "Missing rendered $name surface" >&2; exit 1; }
    done
    xdotool key --clearmodifiers Escape
    await_focus "$external"
    await_scene desktop
done
# Switch directly between two global sheets. One Escape should return to the client.
xdotool key --clearmodifiers super+a
await_focus "$shell"
xdotool key --clearmodifiers super+n
await_scene notifications
xdotool key --clearmodifiers Escape
await_focus "$external"
# Repeating the launcher binding also toggles back to the original application.
xdotool key --clearmodifiers super+space
await_focus "$shell"
xdotool key --clearmodifiers super+space
await_focus "$external"
echo "Verified all four global surfaces and restored the maximized client on workspace 2."
