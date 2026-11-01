#!/usr/bin/env bash
# Usage: tests/smoke.sh path/to/ferro-desktop output-directory [--software]
# Needs Xvfb, xauth, Openbox, xdotool, dbus-run-session and ImageMagick 6.
set -euo pipefail

if (( $# < 2 )); then
    echo "Usage: $0 path/to/ferro-desktop output-directory [--software]" >&2
    exit 2
fi
binary=$(realpath "$1")
mkdir -p "$2"
output=$(realpath "$2")
shift 2
if [[ ! -x "$binary" ]]; then
    echo "Missing executable: $binary" >&2
    exit 2
fi

# Always create a private display and bus. An accidental invocation from a
# real session must not focus its applications or move its workspaces.
if [[ "${FERRO_DESKTOP_PRIVATE_DISPLAY:-}" != "${DISPLAY:-}" || -z "${DISPLAY:-}" ]]; then
    exec timeout 150s env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET xvfb-run -a -s '-screen 0 1440x960x24 -nolisten tcp' \
        dbus-run-session -- bash -c 'export FERRO_DESKTOP_PRIVATE_DISPLAY=$DISPLAY; exec "$@"' \
        bash "$0" "$binary" "$output" "$@"
fi

fixture=$(mktemp -d)
cleanup() {
    if [[ -n "${shell_pid:-}" ]]; then kill "$shell_pid" 2>/dev/null || true; fi
    if [[ -n "${wm_pid:-}" ]]; then kill "$wm_pid" 2>/dev/null || true; fi
    rm -rf -- "$fixture"
}
trap cleanup EXIT

export XDG_DATA_HOME="$fixture/data"
export XDG_CONFIG_HOME="$fixture/config"
export XDG_RUNTIME_DIR="$fixture/runtime"
export XDG_CURRENT_DESKTOP=FerroDesktop
export FERROUI_FRAME_TIMES=1
mkdir -p "$XDG_DATA_HOME/applications" "$XDG_CONFIG_HOME"
mkdir -m 700 "$XDG_RUNTIME_DIR"
convert -size 420x240 'gradient:#193345-#91ddc3' \
    -fill '#192d40' -draw 'rectangle 20,183 95,220' \
    -fill '#285867' -draw 'rectangle 96,183 171,220' \
    -fill '#367e86' -draw 'rectangle 172,183 247,220' \
    -fill '#63aaa5' -draw 'rectangle 248,183 323,220' \
    -fill '#9fe2c9' -draw 'rectangle 324,183 399,220' "$fixture/study.png"
cat >"$XDG_DATA_HOME/applications/ferro-smoke-viewer.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=ImageMagick Display
Comment=Open the Ferro color study in ImageMagick
Icon=display-im6.q16
Exec=display -nostdin -title "Color study - ImageMagick" "$fixture/study.png"
EOF

# Disable the WM's own shortcuts so a workspace switch tests the shell's
# keyboard handler and EWMH request.
cat >"$fixture/openbox.xml" <<'EOF'
<openbox_config xmlns="http://openbox.org/3.4/rc">
  <desktops><number>4</number><names><name>Studio</name><name>Research</name><name>Review</name><name>Personal</name></names></desktops>
  <keyboard><keybind key="A-F4"><action name="Close"/></keybind></keyboard>
</openbox_config>
EOF
openbox --config-file "$fixture/openbox.xml" >"$output/openbox.log" 2>&1 &
wm_pid=$!
for ((i=0; i<100; i++)); do
    [[ "$(xdotool get_num_desktops 2>/dev/null || true)" == 4 ]] && break
    sleep .1
done
[[ "$(xdotool get_num_desktops)" == 4 ]]

"$binary" "$@" >"$output/desktop.log" 2>&1 &
shell_pid=$!
await_window() {
    local id
    for ((i=0; i<150; i++)); do
        kill -0 "$shell_pid" || { cat "$output/desktop.log" >&2; return 1; }
        if id=$(xdotool search --onlyvisible --name "$1" 2>/dev/null); then
            printf '%s\n' "${id%%$'\n'*}"
            return
        fi
        sleep .1
    done
    echo "Window did not appear: $1" >&2
    cat "$output/desktop.log" >&2
    return 1
}
shell_window=$(await_window '^Ferro Desktop$')
xdotool windowsize --sync "$shell_window" 1280 800 windowactivate --sync "$shell_window"
xdotool mousemove 1420 940

# Wait for both first paint and large view transitions. This excludes a
# blank mapped window and stale or duplicate frames from the screenshots.
capture() {
    local name=$1 previous=${2:-} pixels colors
    rm -f -- "$fixture/prior.png"
    for ((i=0; i<60; i++)); do
        import -window "$shell_window" "$fixture/frame.png"
        colors=$(identify -format '%k' "$fixture/frame.png")
        if (( colors > 60 )) && [[ -f "$fixture/prior.png" ]]; then
            local changed=true
            if [[ -n "$previous" ]]; then
                pixels=$(compare -metric AE "$output/$previous.png" "$fixture/frame.png" null: 2>&1 || true)
                awk -v pixels="$pixels" 'BEGIN {exit !(pixels + 0 > 12000)}' || changed=false
            fi
            # Capture after the view has settled. Allow text caret or clock changes.
            pixels=$(compare -metric AE "$fixture/prior.png" "$fixture/frame.png" null: 2>&1 || true)
            if [[ "$changed" == true ]] && awk -v pixels="$pixels" 'BEGIN {exit !(pixels + 0 < 400)}'; then
                cp "$fixture/frame.png" "$output/$name.png"
                return
            fi
        fi
        mv "$fixture/frame.png" "$fixture/prior.png"
        sleep .1
    done
    cp "$fixture/prior.png" "$output/failed-$name.png"
    echo "The $name view did not render" >&2
    return 1
}

# The native Start/search flyouts leave surrounding wallpaper unchanged on Windows
# and macOS; the Material app drawer dims it. All three capture the outside press.
dismiss_launcher() {
    local baseline=$1 opened=$2 theme=$3 before after delta
    before=$(convert "$output/$baseline.png" -format '%[pixel:p{126,194}]' info:)
    after=$(convert "$output/$opened.png" -format '%[pixel:p{126,194}]' info:)
    if [[ $theme == material ]]; then
        [[ $before != "$after" ]] || { echo "Missing Material modal backdrop" >&2; return 1; }
    else
        [[ $before == "$after" ]] || { echo "$theme search repainted the surrounding wallpaper" >&2; return 1; }
    fi
    xdotool mousemove --window "$shell_window" 126 194 click 1 mousemove 1420 940
    for ((i=0; i<60; i++)); do
        import -window "$shell_window" "$fixture/dismissed.png"
        delta=$(compare -metric AE "$output/$baseline.png" "$fixture/dismissed.png" null: 2>&1 || :)
        if awk -v delta="$delta" 'BEGIN {exit !(delta + 0 < 6000)}'; then return; fi
        sleep .1
    done
    echo "Outside click did not dismiss the $theme launcher" >&2
    return 1
}

capture desktop
xdotool key --clearmodifiers ctrl+k
capture applications desktop
dismiss_launcher desktop applications windows
xdotool key --clearmodifiers ctrl+k
# Reopening must restore keyboard focus and retain the partially typed query.
xdotool type --clearmodifiers --delay 20 'ferro color'
xdotool key --clearmodifiers Escape ctrl+k
xdotool type --clearmodifiers --delay 20 ' study'
xdotool key --clearmodifiers Return
viewer=$(await_window '^Color study - ImageMagick$')
[[ "$viewer" != "$shell_window" ]]
xdotool windowactivate --sync "$shell_window" key --clearmodifiers Escape
xdotool windowmove --sync "$viewer" 810 384 windowactivate --sync "$viewer"
read -r sx sy sw sh < <(xdotool getwindowgeometry --shell "$shell_window" | awk -F= '
    $1=="X" {x=$2} $1=="Y" {y=$2} $1=="WIDTH" {w=$2} $1=="HEIGHT" {h=$2}
    END {print x, y, w, h}')
import -window root -crop "${sw}x${sh}+${sx}+${sy}" +repage "$output/native-client.png"
xdotool windowactivate --sync "$shell_window" key --clearmodifiers Escape ctrl+w
capture workspaces desktop
xdotool key --clearmodifiers Escape

# Exercise the production settings handler and validate its commit to disk.
# The three images come from the same process, workspace and launched client.
for choice in '2 macos' '3 material' '1 windows'; do
    read -r key theme <<<"$choice"
    xdotool key --clearmodifiers ctrl+comma
    if [[ "$theme" == macos ]]; then capture appearance desktop; fi
    xdotool key --clearmodifiers "$key"
    for ((i=0; i<60; i++)); do
        [[ "$(cat "$XDG_CONFIG_HOME/ferro-desktop/appearance" 2>/dev/null || true)" == "$theme" ]] && break
        sleep .1
    done
    [[ "$(cat "$XDG_CONFIG_HOME/ferro-desktop/appearance")" == "$theme" ]]
    xdotool key --clearmodifiers Escape
    capture "$theme" appearance
    if [[ "$theme" != windows ]]; then
        xdotool key --clearmodifiers ctrl+k
        # Opening an appearance can restore a previous search. Show the full
        # installed catalog for the screenshot.
        xdotool key --clearmodifiers ctrl+a BackSpace
        capture "$theme-applications" "$theme"
        dismiss_launcher "$theme" "$theme-applications" "$theme"
    fi
    [[ "$(xdotool get_desktop)" == 0 ]]
    [[ "$(xdotool get_desktop_for_window "$viewer")" == 0 ]]
done
for pair in 'windows macos' 'macos material' 'material windows'; do
    read -r first second <<<"$pair"
    pixels=$(compare -metric AE "$output/$first.png" "$output/$second.png" null: 2>&1 || true)
    awk -v pixels="$pixels" 'BEGIN {exit !(pixels + 0 > 12000)}' || {
        echo "Appearances $first and $second look identical" >&2
        exit 1
    }
done
xdotool key --clearmodifiers ctrl+alt+2
for ((i=0; i<50; i++)); do
    [[ "$(xdotool get_desktop)" == 1 ]] && break
    sleep .1
done
[[ "$(xdotool get_desktop)" == 1 ]]
xdotool set_desktop 0 windowactivate --sync "$shell_window" key --clearmodifiers alt+F4
wait "$shell_pid"
unset shell_pid
echo "Captured all appearances, activated a native X11 client and switched workspace."
