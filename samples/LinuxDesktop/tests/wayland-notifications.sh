# Sourced by wayland-session.sh with a private bus, HEADLESS-1 and a focused foot client.
exercise_notifications() (
    set -euo pipefail
    local name=org.freedesktop.Notifications app=org.ferroui.WaylandClient monitor=HEADLESS-1 id=0
    local dir="$XDG_RUNTIME_DIR/banner-probe" terminal_rect
    mkdir -m700 "$dir"
    call() { gdbus call --session --dest "$name" --object-path /org/freedesktop/Notifications --method "$name.$1" "${@:2}"; }
    notify() { call Notify 'Build service' "$1" '' "$2" "$3" "$4" '{}' 0 | sed -nE 's/^\(uint32 ([0-9]+),\)$/\1/p'; }
    cleanup_notice() {
        if [[ $id != 0 && -n $id ]]; then call CloseNotification "$id" >/dev/null || :; fi
        rm -rf -- "$dir"
    }
    trap cleanup_notice EXIT
    # Sample at logical scale so the upper-right rectangle is independent of the
    # maximum scale configured elsewhere. Crop separately to isolate each surface.
    sample() {
        grim -s 1 -o "$monitor" "$dir/frame.png" &&
            convert "$dir/frame.png" -gravity NorthEast -crop 380x166+0+0 +repage "$dir/banner.png" &&
            convert "$dir/frame.png" -crop "$terminal_rect" +repage "$dir/terminal.png"
    }
    changes() {
        local count status=0
        count=$(compare -fuzz 3% -metric AE "$1" "$2" null: 2>&1) || status=$?
        (( status < 2 )) || fail "$count"
        printf '%.0f\n' "$count"
    }
    first_banner() {
        sample && (( $(changes "$dir/before.png" "$dir/banner.png") > 18000 )) && focused "$app"
    }
    replaced_banner() {
        sample && (( $(changes "$dir/before.png" "$dir/banner.png") > 26000 )) &&
            (( $(changes "$dir/first.png" "$dir/banner.png") > 5000 )) && focused "$app"
    }
    received_text() {
        sample && (( $(changes "$dir/before-input.png" "$dir/terminal.png") > 260 )) && focused "$app"
    }
    dismissed() {
        sample && (( $(changes "$dir/before.png" "$dir/banner.png") < 70 )) && focused "$app"
    }

    wait_for focused "$app"
    gdbus wait --session --timeout 8 "$name"
    call GetServerInformation | grep -q 'Ferro Desktop'
    terminal_rect=$(tree | jq -er --arg app "$app" 'first(.. | objects | select(.app_id?==$app and .shell?=="xdg_shell" and .focused)) | .rect |
        "\(.width-32)x\(.height-65)+\(.x+16)+\(.y+45)"')
    sample; cp "$dir/banner.png" "$dir/before.png"
    id=$(notify 0 'Compiling the workspace' 'Building the Linux desktop shell.' '[]')
    [[ $id =~ ^[1-9][0-9]*$ ]] || fail 'No notification identifier'
    wait_for first_banner
    cp "$dir/banner.png" "$dir/first.png"
    [[ $(notify "$id" 'Build succeeded' 'All targets passed on Linux.' "['open','View build log']") == "$id" ]] ||
        fail 'Replacing the notification changed its identifier'
    wait_for replaced_banner
    cp "$dir/frame.png" "$output/notification-banner.png"
    cp "$dir/terminal.png" "$dir/before-input.png"
    # Sway tree focus alone cannot distinguish a keyboard grab by a layer
    # surface. The real terminal's line discipline echoes these native key events.
    wtype -s 150 'Keyboard focus stayed in this terminal.'
    wait_for received_text
    wtype -M ctrl -k u -m ctrl
    call CloseNotification "$id" >/dev/null; id=0
    wait_for dismissed
    echo 'Verified native Wayland notification placement, replacement, terminal keyboard delivery and pixel restoration on close.'
)
