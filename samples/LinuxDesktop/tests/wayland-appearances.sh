# Sourced by the private Wayland login-session test.
# The command endpoint publishes scene changes before the compositor finishes
# the first configure/map/keyboard-enter cycle. Ctrl+, always selects the
# appearance panel, so it can safely be retried until the layer receives it.
open_appearance() {
    if is_scene appearance; then return 0; fi
    wtype -M ctrl -k comma -m ctrl
    return 1
}

exercise_appearances() {
    local key profile color tx ty
    for profile in macos material windows; do
        case "$profile" in macos) key=2; color=342F3C ;; material) key=3; color=182922 ;; windows) key=1; color=F2F5FA ;; esac
        wtype -M logo -k space -m logo
        wait_for is_scene applications
        wait_for open_appearance
        wtype -k "$key"
        wait_for test -s "$XDG_CONFIG_HOME/ferro-desktop/appearance"
        wait_for grep -qx "$profile" "$XDG_CONFIG_HOME/ferro-desktop/appearance"
        wtype -k Escape
        wait_for is_scene desktop
        wait_for focused org.ferroui.WaylandClient
        capture "$profile-client"
        # The same focused terminal remains mapped. Test its compositor-drawn
        # titlebar, sampling away from the text and frame edges.
        read -r tx ty < <(tree | jq -r '.. | objects | select(.app_id?=="org.ferroui.WaylandClient") | [(.rect.x+.rect.width-25),(.rect.y-(.deco_rect.height/2|floor))] | @tsv')
        local pixel
        pixel=$(convert "$output/$profile-client.png" -format "%[hex:p{$tx,$ty}]" info:)
        [[ ${pixel:0:6} == "$color" ]] || fail "$profile terminal titlebar was $pixel, expected $color"
    done
}
