#!/bin/sh
# Stage the binary built by Cargo, session supervisor, WM configuration, and display-manager entry.
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
binary=${1:?Usage: install.sh /path/to/release/ferro-desktop}
prefix=${PREFIX:-/usr/local}
sessions=${XSESSIONS_DIR:-/usr/share/xsessions}
install -Dm755 "$binary" "${DESTDIR:-}$prefix/bin/ferro-desktop"
install -Dm755 "$here/ferro-desktop-session" "${DESTDIR:-}$prefix/bin/ferro-desktop-session"
install -Dm644 "$here/openbox.xml" "${DESTDIR:-}$prefix/share/ferro-desktop/openbox.xml"
install -Dm644 "$here/ferro-desktop.desktop" "${DESTDIR:-}$sessions/ferro-desktop.desktop"
