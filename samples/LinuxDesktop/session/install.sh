#!/bin/sh
# Stage the binary built by Cargo, session supervisor, WM configuration, and display-manager entry.
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
    echo 'Usage: install.sh /path/to/ferro-desktop [/path/to/control-catalog-desktop]' >&2
    exit 2
fi
binary=$1
catalog=${2-}
if [ ! -f "$binary" ] || [ ! -x "$binary" ]; then echo "Missing desktop executable: $binary" >&2; exit 2; fi
if [ "$#" -eq 2 ] && { [ ! -f "$catalog" ] || [ ! -x "$catalog" ]; }; then
    echo "Missing ControlCatalog executable: $catalog" >&2
    exit 2
fi
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

# Install the existing sample as a separate process when explicitly supplied.
if [ -n "$catalog" ]; then
    install -Dm755 "$catalog" "${DESTDIR:-}$prefix/bin/control-catalog-desktop"
    install -Dm644 "$here/org.ferroui.WinUIGallery.desktop" "${DESTDIR:-}$prefix/share/applications/org.ferroui.WinUIGallery.desktop"
fi
