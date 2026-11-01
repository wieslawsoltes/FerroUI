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
    exercise_winui_style
}

# Use the actual input device and per-output work area. All changes below happen
# in one shell process with the same native GTK/terminal session.
exercise_winui_style() {
    local windows_style="$XDG_CONFIG_HOME/ferro-desktop/windows-style"
    wtype -M logo -k space -m logo
    wait_for is_scene applications
    wait_for open_appearance
    wtype -k d
    wait_for grep -sxq 'dark normal' "$windows_style"
    wtype -k c
    wait_for grep -sxq 'dark compact' "$windows_style"
    wait_for winui_reserved 42
    capture winui-dark-compact HEADLESS-1 "$output/windows-client.png"

    # Selecting another desktop and returning restores the Windows subprofile.
    wtype -k 2
    wait_for grep -qx macos "$XDG_CONFIG_HOME/ferro-desktop/appearance"
    wtype -k 1
    wait_for grep -qx windows "$XDG_CONFIG_HOME/ferro-desktop/appearance"
    wait_for winui_reserved 42
    wtype -k h
    wait_for grep -sxq 'high-contrast compact' "$windows_style"
    capture winui-high-contrast HEADLESS-1 "$output/winui-dark-compact.png"
    local black
    black=$(convert "$output/winui-high-contrast.png" -gravity Northeast -crop 200x80+40+26 +repage -colorspace gray -format '%[fx:mean]' info:)
    awk -v value="$black" 'BEGIN {exit !(value < .08)}' || fail "High-contrast desktop continued painting its colored wallpaper"

    wtype -k l
    wait_for grep -sxq 'light compact' "$windows_style"
    wtype -k c
    wait_for grep -sxq 'light normal' "$windows_style"
    wait_for winui_reserved 52
    wtype -k Escape
    wait_for is_scene desktop
    wait_for focused org.ferroui.WaylandClient
    echo 'Verified WinUI light, dark, high contrast, compact panel reservations and restoration on profile switch.'
}

winui_reserved() {
    local output_height workspace_height
    output_height=$(swaymsg -rt get_outputs | jq -r '.[] | select(.name=="HEADLESS-1") | .rect.height')
    workspace_height=$(tree | jq -r 'first(..|objects|select(.type?=="output" and .name=="HEADLESS-1")|..|objects|select(.type?=="workspace")|.rect.height)')
    [[ $workspace_height =~ ^[0-9]+$ ]] && (( output_height - workspace_height == $1 ))
}
