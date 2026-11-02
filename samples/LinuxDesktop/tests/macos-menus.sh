#!/usr/bin/env bash
# Drive native menu tracking, client management and persistent Dock pins on a private X server.
set -euo pipefail
if [[ ${1:-} != --fixture ]]; then
    binary=$(realpath "${1:?Supply a ferro-desktop binary}")
    out=$(realpath -m "${2:?Supply a capture directory}")
    mkdir -p "$out"
    exec timeout 90s env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET xvfb-run -a -s '-screen 0 1280x800x24 -nolisten tcp' \
        dbus-run-session -- bash -c 'export FERRO_MENU_TEST_DISPLAY=$DISPLAY; exec "$@"' \
        bash "$0" --fixture "$binary" "$out"
fi
[[ -n ${DISPLAY:-} && $DISPLAY == "${FERRO_MENU_TEST_DISPLAY:-}" ]]
binary=$2
out=$3
fixture=$(mktemp -d)
pids=()
cleanup() {
    for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || :; done
    wait || :
    rm -rf -- "$fixture"
}
trap cleanup EXIT
export XDG_DATA_HOME="$fixture/data" XDG_CONFIG_HOME="$fixture/config" XDG_RUNTIME_DIR="$fixture/runtime"
mkdir -p "$XDG_DATA_HOME/applications" "$XDG_CONFIG_HOME"
mkdir -m 700 "$XDG_RUNTIME_DIR"
openbox --sm-disable >"$out/menu-openbox.log" 2>&1 & pids+=("$!")
wait_for() {
    local i
    for ((i=0;i<65;i++)); do if "$@"; then return; fi; sleep .08; done
    import -window root "$out/menu-failure.png"
    echo "Timed out waiting for $*" >&2; exit 1
}
wm_ready() { xprop -root _NET_SUPPORTING_WM_CHECK | grep -q 'window id #'; }
wait_for wm_ready

python3 "$(dirname "$0")/julia.py" >"$fixture/terrain.ppm"
convert "$fixture/terrain.ppm" -resize 350x230 "$fixture/terrain.png"
convert "$fixture/terrain.png" -flop -modulate 100,75,130 "$fixture/coast.png"
# Independent, resizable GTK top levels exercise the X11 window manager
# and real WM_CLASS grouping. Keep the image windows on the same workspace.
GDK_BACKEND=x11 /usr/bin/python3 - "$fixture" >"$out/menu-client.log" 2>&1 <<'PYGTK' &
from pathlib import Path
import sys
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import GLib, Gtk

GLib.set_prgname("ferro-menu-proof")
windows = []
for title, image, x, y in (("Terrain study", "terrain", 435, 120), ("Coastal study", "coast", 825, 375)):
    window = Gtk.Window(title=title)
    window.set_default_size(350, 240)
    window.move(x, y)
    window.add(Gtk.Image.new_from_file(str(Path(sys.argv[1]) / (image + ".png"))))
    window.connect("destroy", Gtk.main_quit)
    window.show_all()
    windows.append(window)
Gtk.main()
PYGTK
pids+=("$!")
terrain=$(xdotool search --sync --onlyvisible --name '^Terrain study$' | head -1)
coast=$(xdotool search --sync --onlyvisible --name '^Coastal study$' | head -1)
wmclass=$(xprop -id "$terrain" WM_CLASS | sed -n 's/.*", "\([^"]*\)"/\1/p')
[[ $wmclass =~ ^[[:alnum:]_.-]+$ ]]
cat >"$XDG_DATA_HOME/applications/$wmclass.desktop" <<ENTRY
[Desktop Entry]
Type=Application
Name=Studio
Icon=display-im6.q16
Exec=display
ENTRY
"$binary" --session --theme=macos >"$out/menu-shell.log" 2>&1 & pids+=("$!")
scene() { [[ $("$binary" --current-scene 2>/dev/null || :) == "$1" ]]; }
wait_for scene desktop
xdotool windowactivate --sync "$terrain" mousemove 12 445

# A stable crop below the menu bar distinguishes each open heading from the
# previous one. Clock, Dock and independently managed clients lie outside it.
capture_menu() {
    local name=$1 region=$2 reference=$3 changed stable i
    rm -f "$fixture/last.png"
    for ((i=0;i<45;i++)); do
        import -window root "$fixture/frame.png"
        convert "$fixture/frame.png" -crop "$region" +repage "$fixture/current.png"
        changed=$(compare -metric AE "$fixture/current.png" "$reference" null: 2>&1 || :)
        if [[ -f "$fixture/last.png" ]]; then
            stable=$(compare -metric AE "$fixture/current.png" "$fixture/last.png" null: 2>&1 || :)
            if awk -v c="$changed" -v s="$stable" 'BEGIN {exit !(c+0 > 1100 && s+0 < 20)}'; then
                cp "$fixture/frame.png" "$out/$name.png"
                cp "$fixture/current.png" "$fixture/last-menu.png"
                return
            fi
        fi
        cp "$fixture/current.png" "$fixture/last.png"
        sleep .1
    done
    cp "$fixture/frame.png" "$out/menu-failure.png"
    echo "Did not see a distinct, settled $name" >&2; exit 1
}
region=360x190+8+36
ready() {
    import -window root "$fixture/desktop.png"
    # Wait for the menu bar and native desktop wallpaper to replace the unmapped root.
    convert "$fixture/desktop.png" -format '%[fx:p{80,95}.r-p{80,95}.g]' info: |
        awk '{exit !($1 > .03)}'
}
wait_for ready
convert "$fixture/desktop.png" -crop "$region" +repage "$fixture/base.png"
xdotool mousemove 29 16 click 1
capture_menu menu-system "$region" "$fixture/base.png"
xdotool mousemove 78 16
capture_menu menu-application "$region" "$fixture/last-menu.png"
xdotool mousemove 142 16
capture_menu menu-window "$region" "$fixture/last-menu.png"
# The host desktop sits *under* external clients. A maximized GTK window
# must not cover the next pull-down's Restore Size command.
maximized() { xprop -id "$terrain" _NET_WM_STATE | grep -q _NET_WM_STATE_MAXIMIZED_VERT; }
normal_size() { ! maximized; }
xdotool mousemove 197 71 click 1
wait_for maximized
import -window root "$fixture/first-zoom.png"
convert "$fixture/first-zoom.png" -crop "$region" +repage "$fixture/zoom-base.png"
xdotool mousemove 142 16 click 1
capture_menu menu-over-maximized "$region" "$fixture/zoom-base.png"
xdotool mousemove 197 71 click 1
wait_for normal_size
xdotool mousemove 142 16 click 1
capture_menu menu-window "$region" "$fixture/base.png"
xdotool mousemove 197 45 click 1
hidden() { xprop -id "$terrain" _NET_WM_STATE | grep -q '_NET_WM_STATE_HIDDEN'; }
wait_for hidden

# Right-click the running Studio group and restore the first window. Both
# mapped X clients share one WM_CLASS and therefore one Dock entry.
dock=470x265+540+495
capture_dock() {
    xdotool mousemove 724 766 click 3
    capture_menu dock-windows "$dock" "$fixture/dock-base.png"
}
import -window root "$fixture/after-minimize.png"
convert "$fixture/after-minimize.png" -crop "$dock" +repage "$fixture/dock-base.png"
capture_dock
xdotool key End Return
pinned() { [[ -f "$XDG_CONFIG_HOME/ferro-desktop/favorites.json" ]] && jq -e --arg id "$wmclass.desktop" 'index($id) != null' "$XDG_CONFIG_HOME/ferro-desktop/favorites.json" >/dev/null; }
wait_for pinned
# Updating the favorites file precedes the Dock's repaint. Wait for the
# selected menu to unmap before opening a menu on the newly pinned button.
dismissed() {
    import -window root "$fixture/after-pin.png"
    convert "$fixture/after-pin.png" -crop '180x80+730+675' +repage "$fixture/clear.png"
    convert "$out/dock-windows.png" -crop '180x80+730+675' +repage "$fixture/shown.png"
    local count
    count=$(compare -metric AE "$fixture/clear.png" "$fixture/shown.png" null: 2>&1 || :)
    awk -v count="$count" 'BEGIN {exit !(count+0>1400)}'
}
wait_for dismissed
convert "$fixture/after-pin.png" -crop "$dock" +repage "$fixture/dock-base.png"
xdotool mousemove 718 766 click 3
capture_menu dock-pinned "$dock" "$fixture/dock-base.png"
xdotool key Home
xdotool key Return
restored() { [[ $(xdotool getactivewindow) == "$terrain" ]] && ! hidden; }
wait_for restored

# Use the bar via keyboard and verify actual scene dispatch from its first item.
xdotool mousemove 29 16 click 1 key Down Return
wait_for scene appearance
xdotool key Escape
wait_for scene desktop
echo "Verified menu tracking, native minimize/restore, Dock pinning and keyboard activation."
