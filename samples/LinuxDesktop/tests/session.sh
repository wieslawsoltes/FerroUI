#!/usr/bin/env bash
# Exercise actual X11 clients, keyboard shortcuts, and WM decoration changes on a private display.
# Optionally capture all three appearances: session.sh capture path/to/ferro-desktop output-directory
set -euo pipefail
here=$(cd -- "$(dirname -- "$0")" && pwd)
case ${1:-test} in
    --session)
        [[ -n ${FERRO_DESKTOP_SESSION_DIR:-} && -n ${FERRO_TEST_PRIVATE_DISPLAY:-} && $DISPLAY == "$FERRO_TEST_PRIVATE_DISPLAY" ]]
        "$here/../build.sh" test --profile dev -- --ignored real_client_lifecycle
        exec "$here/../build.sh" test --profile dev -- --ignored running_openbox_redecorates_the_same_native_client
        ;;
    test) mode=test ;;
    capture)
        mode=capture
        [[ $# == 3 ]] || { echo "Usage: $0 capture /path/to/ferro-desktop output-directory" >&2; exit 2; }
        export FERRO_DESKTOP_BINARY
        FERRO_DESKTOP_BINARY=$(realpath "$2")
        mkdir -p -- "$3"; output=$(realpath "$3")
        ;;
    *) echo "Usage: $0 [test | capture /path/to/ferro-desktop output-directory]" >&2; exit 2 ;;
esac
if [[ -z ${FERRO_TEST_PRIVATE_DISPLAY:-} || ${DISPLAY:-} != "$FERRO_TEST_PRIVATE_DISPLAY" ]]; then
    exec xvfb-run -a -s '-screen 0 1440x960x24 -nolisten tcp' dbus-run-session -- \
        bash -c 'export FERRO_TEST_PRIVATE_DISPLAY=$DISPLAY; exec "$@"' bash "$here/session.sh" "$@"
fi
scratch=$(mktemp -d)
export XDG_RUNTIME_DIR="$scratch/runtime" XDG_CONFIG_HOME="$scratch/config" XDG_CACHE_HOME="$scratch/cache"
mkdir -m 700 "$XDG_RUNTIME_DIR" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
supervisor= viewer=
cleanup() {
    [[ -z $viewer ]] || kill "$viewer" 2>/dev/null || :
    [[ -z $supervisor ]] || kill "$supervisor" 2>/dev/null || :
    wait || :
    rm -rf -- "$scratch"
}
trap cleanup EXIT
if [[ $mode == test ]]; then
    FERRO_DESKTOP_BINARY="$here/session.sh" "$here/../session/ferro-desktop-session"
    exit
fi
"$here/../session/ferro-desktop-session" >"$output/session.log" 2>&1 &
supervisor=$!
timeout 30s xdotool search --sync --onlyvisible --name '^Ferro Desktop$' >/dev/null
python3 "$here/julia.py" > "$scratch/study.ppm"
display -immutable -title 'Julia study  ·  ImageMagick' -geometry +220+215 "$scratch/study.ppm" &
viewer=$!
client=$(timeout 15s xdotool search --sync --onlyvisible --name 'Julia study' | head -1)
xdotool windowactivate --sync "$client" windowmove "$client" 225 220 mousemove 1100 750
for choice in '1 windows' '2 macos' '3 material'; do
    read -r key theme <<< "$choice"
    xdotool key --clearmodifiers super+space
    sleep .3
    xdotool key --clearmodifiers ctrl+comma
    sleep .3
    xdotool key --clearmodifiers "$key"
    for ((i=0;i<60;i++)); do
        [[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/appearance" 2>/dev/null || :) == "$theme" ]] && break
        sleep .1
    done
    [[ $(cat "$XDG_CONFIG_HOME/ferro-desktop/appearance") == "$theme" ]]
    xdotool key --clearmodifiers Escape
    xdotool windowactivate --sync "$client"
    sleep .4
    import -window root -depth 8 "$output/$theme.png"
done
