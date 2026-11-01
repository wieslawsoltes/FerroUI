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
install -Dm755 "$here/ferro-desktop-wayland" "${DESTDIR:-}$prefix/bin/ferro-desktop-wayland"
install -Dm644 "$here/sway.conf" "${DESTDIR:-}$prefix/share/ferro-desktop/sway.conf"
install -Dm644 "$here/ferrodesktop-portals.conf" "${DESTDIR:-}$prefix/share/xdg-desktop-portal/ferrodesktop-portals.conf"
install -Dm644 "$here/ferro-desktop-wayland.desktop" "${DESTDIR:-}${WAYLAND_SESSIONS_DIR:-/usr/share/wayland-sessions}/ferro-desktop-wayland.desktop"
install -Dm644 "$here/ferro-composition.conf" "${DESTDIR:-}$prefix/share/ferro-desktop/ferro-composition.conf"
install -Dm644 "$here/ferro-desktop-composed.desktop" "${DESTDIR:-}${WAYLAND_SESSIONS_DIR:-/usr/share/wayland-sessions}/ferro-desktop-composed.desktop"
