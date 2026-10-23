#!/bin/bash
# In the Linux virtual machine: one smoke run of the catalog on the X11 desktop, every page shown
# for 150 ms, with the frame times of the composition targets (FERROUI_FRAME_TIMES) and a
# screenshot of the window only. Logs go to /tmp/vrun-<name>.log; times.sh prints them.
#
#   run.sh <name> [VAR=value ...]
#
# Examples: run.sh skia                      (Skia, the default backend)
#           run.sh vello FERROUI_RENDERER=vello
#           run.sh vello-soft FERROUI_RENDERER=vello FERROUI_SMOKE_RENDERING=software
set -u
name=$1; shift
export DISPLAY=${DISPLAY:-:0} XAUTHORITY=${XAUTHORITY:-$(ls /run/user/$(id -u)/.mutter-Xwaylandauth.* | head -1)}
BIN=${FERROUI_VM_BIN:-/media/psf/1TB-macOS/ferroui-vm-linux/release/control-catalog-desktop}
OUT=${FERROUI_VM_SHOTS:-/media/psf/1TB-macOS/ferroui-screenshots/vello-vm}; mkdir -p "$OUT"
pkill -f control-catalog-desktop; sleep 1
env "$@" FERROUI_FRAME_TIMES=1 FERROUI_SMOKE_EXIT_MS=22000 FERROUI_SMOKE_PAGES=150 FERROUI_CATALOG_WAYLAND=x11 "$BIN" > "/tmp/vrun-$name.log" 2>&1 &
PID=$!
sleep 9
W=$(xdotool search --pid $PID 2>/dev/null | tail -1)
if [ -n "$W" ]; then import -window "$W" "$OUT/$name.png" 2>/dev/null || xwd -id "$W" -silent > "$OUT/$name.xwd"; fi
wait $PID; echo "exit $?"
grep -E "frame times of|Rendering modes|Vello backend|draws the windows|panicked|error" "/tmp/vrun-$name.log" | head -8
