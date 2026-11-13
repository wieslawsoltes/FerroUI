#!/bin/sh
# Starts the window manager of the X11 smoke runs (openbox) on the display of the caller, waits
# until it manages the screen, and runs the command it was given, with the exit status of that
# command.
#
# usage: scripts/x11/with-window-manager.sh <command> [<argument>...]
#
# The window manager says itself when it is ready: it runs the command of `--startup` once it
# has taken the screen and the windows that are on it. A fixed wait is a race with a start that
# takes longer, and a window manager that takes a window in the middle of a run reparents it
# into a frame: the window moves under the pointer and loses the keyboard focus.
set -u

if [ "$#" -eq 0 ]; then
    echo "usage: $0 <command> [<argument>...]" >&2
    exit 2
fi

folder=$(mktemp -d "${TMPDIR:-/tmp}/ferroui-window-manager.XXXXXX") || exit 1
ready="$folder/ready"

openbox --startup "touch $ready" &

tries=0
while [ ! -e "$ready" ]; do
    tries=$((tries + 1))
    if [ "$tries" -gt 300 ]; then
        echo "with-window-manager: openbox did not manage the screen within 60 s" >&2
        rm -rf "$folder"
        exit 1
    fi
    sleep 0.2
done
rm -rf "$folder"

"$@"
