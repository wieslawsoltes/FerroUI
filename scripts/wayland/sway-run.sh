#!/bin/sh
# Runs a command as a client of a headless sway (the compositor of the tests of the Wayland
# backend: docs/porting/wayland-platform.md, section 11.1).
#
#   scripts/wayland/sway-run.sh [--scale N] [--size WxH] [--log FILE] -- command [arguments...]
#
# The compositor runs on the headless backend of wlroots with its software renderer, on a
# configuration without a bar and without borders, so that the one tiled window covers the output
# exactly. The command gets WAYLAND_DISPLAY, SWAYSOCK and XDG_RUNTIME_DIR of that compositor, and
# no DISPLAY. The exit status is the one of the command; the log of the compositor is printed when
# the command failed.
#
# Needs: sway (with swaymsg). Nothing of a session: no seat, no D-Bus, no GPU.
set -u

scale=1
size=1280x720
log=""
while [ $# -gt 0 ]; do
    case "$1" in
        --scale) scale="$2"; shift 2 ;;
        --size) size="$2"; shift 2 ;;
        --log) log="$2"; shift 2 ;;
        --) shift; break ;;
        *) echo "sway-run.sh: unknown option $1" >&2; exit 2 ;;
    esac
done
if [ $# -eq 0 ]; then
    echo "usage: sway-run.sh [--scale N] [--size WxH] [--log FILE] -- command [arguments...]" >&2
    exit 2
fi

runtime="$(mktemp -d "${TMPDIR:-/tmp}/ferroui-sway.XXXXXX")" || exit 1
chmod 700 "$runtime"
config="$runtime/config"
sway_log="${log:-$runtime/sway.log}"
cat > "$config" <<EOF
# One output of a known mode and scale.
output HEADLESS-1 resolution $size scale $scale
# No decorations of the compositor: the window is the output.
default_border none
default_floating_border none
focus_follows_mouse no
xwayland disable
EOF

export XDG_RUNTIME_DIR="$runtime"
unset DISPLAY WAYLAND_DISPLAY SWAYSOCK

WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman \
    sway --config "$config" > "$sway_log" 2>&1 &
sway_pid=$!

cleanup() {
    kill "$sway_pid" 2>/dev/null
    wait "$sway_pid" 2>/dev/null
    rm -rf "$runtime"
}

# The compositor is ready when its Wayland socket and its IPC socket exist.
tries=0
socket=""
ipc=""
while [ $tries -lt 100 ]; do
    socket="$(ls "$runtime" 2>/dev/null | grep '^wayland-[0-9]*$' | head -n 1)"
    ipc="$(ls "$runtime"/sway-ipc.*.sock 2>/dev/null | head -n 1)"
    if [ -n "$socket" ] && [ -n "$ipc" ]; then break; fi
    if ! kill -0 "$sway_pid" 2>/dev/null; then break; fi
    tries=$((tries + 1))
    sleep 0.1
done
if [ -z "$socket" ] || [ -z "$ipc" ]; then
    echo "sway-run.sh: the compositor did not start" >&2
    cat "$sway_log" >&2
    cleanup
    exit 1
fi

export WAYLAND_DISPLAY="$socket"
export SWAYSOCK="$ipc"
echo "sway-run.sh: sway $(sway --version 2>/dev/null | head -n 1) on $WAYLAND_DISPLAY, output $size at scale $scale"

"$@"
status=$?

if [ $status -ne 0 ]; then
    echo "sway-run.sh: the command ended with $status; the log of the compositor:" >&2
    tail -n 60 "$sway_log" >&2
fi
cleanup
exit $status
