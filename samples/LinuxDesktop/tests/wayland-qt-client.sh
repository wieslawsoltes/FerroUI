#!/usr/bin/env bash
# Called in a private wayland-session.sh run after an independent GTK client maps.
set -euo pipefail
[[ ${FERRO_DESKTOP_NESTED:-} == 1 && ${WLR_BACKENDS:-} == headless && ${FERRO_TEST_QT:-} == 1 && $# == 1 ]] ||
    { echo 'Run through FERRO_TEST_QT=1 tests/wayland-session.sh' >&2; exit 2; }
dest=$1
scratch=$(mktemp -d "$XDG_RUNTIME_DIR/qt.XXXXXX")
qt=
gtk=
gtk_restore=
cleanup() {
    [[ -z $qt ]] || { kill "$qt" 2>/dev/null || :; wait "$qt" || :; }
    if [[ -n $gtk_restore ]]; then swaymsg "[con_id=$gtk] $gtk_restore" >/dev/null || :; fi
    rm -rf -- "$scratch"
}
trap cleanup EXIT
tree() { swaymsg -rt get_tree; }
fail() { echo "Qt Wayland test: $*" >&2; exit 1; }
command_ok() { swaymsg -r "$1" | jq -e 'length > 0 and all(.[]; .success)' >/dev/null; }
# Choose the GTK native window on its existing workspace.
gtk_node=$(tree | jq -ce 'first(.. | objects | select(.shell?=="xdg_shell" and ((.app_id? // "")|test("gtk";"i"))))')
gtk=$(jq -r '.id' <<<"$gtk_node")
if [[ $(jq -r '.type' <<<"$gtk_node") == floating_con ]]; then
    gtk_restore=$(jq -r '.rect | "resize set width \(.width) px height \(.height) px, move absolute position \(.x) px \(.y) px"' <<<"$gtk_node")
else
    gtk_restore='floating disable'
fi
command_ok "[con_id=$gtk] focus"
space=$(swaymsg -rt get_workspaces | jq -ce '.[] | select(.focused)')
out=$(jq -r '.output' <<<"$space")
read -r ox oy < <(swaymsg -rt get_outputs | jq -r --arg out "$out" '.[]|select(.name==$out and .active)|.rect|[.x,.y]|@tsv')
read -r x y width height < <(jq -r '.rect|[.x,.y,.width,.height]|@tsv' <<<"$space")
(( width >= 1024 && height >= 580 )) || fail 'Use a landscape output at least 1024x580 for the toolkit capture'
command_ok "[con_id=$gtk] floating enable, resize set width $((width/2-38)) px height $((height-180)) px, move absolute position $((x+24)) px $((y+55)) px"

# This is the stock Qt Widgets colour chooser; Qt owns the swatches, palette,
# native Wayland surface, text inputs and event loop independently of Ferro.
env -u DISPLAY QT_QPA_PLATFORM=wayland /usr/bin/python3 -u - >"$dest/qt.log" 2>&1 <<'PY' &
from PyQt5.QtGui import QColor
from PyQt5.QtWidgets import QApplication, QColorDialog

app = QApplication(["ferro-qt-interop"])
assert app.platformName() == "wayland", app.platformName()
app.setDesktopFileName("org.ferroui.NativeQtColor")
dialog = QColorDialog(QColor("#427CAD"))
dialog.setOption(QColorDialog.DontUseNativeDialog)
dialog.setWindowTitle("Select Color — Qt Widgets")
dialog.show()
app.exec()
PY
qt=$!
qt_node=
for ((i=0;i<110;i++)); do
    qt_node=$(tree | jq -c --argjson pid "$qt" 'first(.. | objects | select(.pid?==$pid and .app_id?=="org.ferroui.NativeQtColor" and .shell?=="xdg_shell")) // empty')
    [[ -z $qt_node ]] || break
    kill -0 "$qt" || { cat "$dest/qt.log" >&2; fail 'Qt exited'; }
    sleep .06
done
[[ -n $qt_node ]] || fail 'Qt did not create a native xdg-shell toplevel'
qt_id=$(jq -r '.id' <<<"$qt_node")
command_ok "[con_id=$qt_id] floating enable, move absolute position $((x+width/2+12)) px $((y+height/3)) px, focus"

# Require both actual toplevels, multiple rendered colours inside each client's
# frame, and a settled composited frame. Capture at one pixel per logical unit.
recorded=false
for ((i=0;i<38;i++)); do
    tree > "$scratch/tree.json"
    views=$(jq -c --argjson gtk "$gtk" --argjson qt "$qt_id" '[..|objects|select(.shell?=="xdg_shell" and (.id==$gtk or .id==$qt))|{id,pid,app_id,shell,name,rect,visible,focused}]' "$scratch/tree.json")
    if jq -e --argjson qt "$qt_id" 'length==2 and all(.[];.visible!=false) and any(.[];.id==$qt and .focused)' <<<"$views" >/dev/null &&
        grim -s 1 -o "$out" "$scratch/next.png"; then
        colored=0
        while read -r vx vy vw vh; do
            # Use the window interior, excluding borders and its titlebar.
            (( vw > 90 && vh > 100 )) || continue
            colors=$(convert "$scratch/next.png" -crop "$((vw-36))x$((vh-70))+$((vx-ox+18))+$((vy-oy+50))" +repage -format '%k' info:)
            if (( colors > 90 )); then colored=$((colored+1)); fi
        done < <(jq -r '.[] | .rect | [.x,.y,.width,.height] | @tsv' <<<"$views")
        # The enclosing session restores the Windows appearance first. Keep the
        # light taskbar in the capture as evidence of the live desktop shell.
        panel=$(convert "$scratch/next.png" -gravity South -crop 90%x2%+0+9 +repage -colorspace Gray -format '%[fx:mean]' info:)
        if (( colored == 2 )) && awk -v p="$panel" 'BEGIN {exit !(p > .65)}' && [[ -f $scratch/previous.png ]]; then
            delta=$(compare -metric AE -fuzz 2% "$scratch/previous.png" "$scratch/next.png" null: 2>&1) || :
            if awk -v d="$delta" 'BEGIN {exit !(d+0 < 750)}'; then
                cp "$scratch/next.png" "$dest/gtk-qt-native.png"
                jq . <<<"$views" > "$dest/gtk-qt-native.json"
                recorded=true; break
            fi
        fi
        mv "$scratch/next.png" "$scratch/previous.png"
    fi
    sleep .12
done
[[ $recorded == true ]] || fail 'Both native toolkits did not present a stable frame'
echo 'Verified GTK and Qt Widgets running on separate native xdg-shell surfaces.'
