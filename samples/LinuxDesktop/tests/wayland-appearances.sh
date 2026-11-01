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
    local key profile
    for profile in macos material windows; do
        case "$profile" in macos) key=2 ;; material) key=3 ;; windows) key=1 ;; esac
        wtype -M logo -k space -m logo
        wait_for is_scene applications
        wait_for open_appearance
        wtype -k "$key"
        wait_for grep -qx "$profile" "$XDG_CONFIG_HOME/ferro-desktop/appearance"
        wtype -k Escape
        wait_for is_scene desktop
        wait_for focused org.ferroui.WaylandClient
        capture "$profile-client"
    done
}
