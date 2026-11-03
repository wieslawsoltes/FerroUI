# Sourced by the isolated compositor test. Exercise the production GIO launch
# path, including its handling of field codes and the per-child environment.
install_composed_launcher() {
    local data=$1 tmp=$2
    mkdir -p "$data/applications"
    cat >"$tmp/record-launch" <<'PROBE'
#!/bin/sh
printf '%s\n' "$WAYLAND_DISPLAY" "$XDG_SESSION_TYPE" "${DISPLAY-unset}" "${SWAYSOCK-unset}" "${WAYLAND_SOCKET-unset}" >"$1"
PROBE
    chmod +x "$tmp/record-launch"
    printf '[Desktop Entry]\nType=Application\nName=Ferro Child Launch Probe\nExec="%s/record-launch" "%s/launched-env" %%U\n' "$tmp" "$tmp" >"$data/applications/org.ferroui.ChildLaunchProbe.desktop"
}

test_composed_launcher() {
    local binary=$1 tmp=$2 expected=$3 i lum
    "$binary" --platform=wayland --show=applications
    for ((i=0;i<100;i++)); do
        [[ $("$binary" --platform=wayland --current-scene 2>/dev/null || :) == applications ]] && break
        sleep .05
    done
    # The scene signal precedes rendering. Sample a patch inside the WinUI
    # search field until the light launcher covers the blue wallpaper.
    for ((i=0;i<100;i++)); do
        grim -s 1 -o HEADLESS-1 "$output/launcher-open.png"
        lum=$(convert "$output/launcher-open.png" -crop 120x8+630+556 +repage -colorspace Gray -format '%[fx:mean]' info:)
        awk -v v="$lum" 'BEGIN {exit !(v>.85)}' && break
        sleep .07
    done
    wtype -s 120 'Ferro Child Launch Probe'
    sleep .2
    grim -s 1 -o HEADLESS-1 "$output/launcher-query.png"
    wtype -s 80 -k Return
    for ((i=0;i<120;i++)); do [[ -s $tmp/launched-env ]] && break; sleep .05; done
    if [[ ! -s $tmp/launched-env ]]; then
        grim -s 1 -o HEADLESS-1 "$output/launcher-failed.png"
        echo "Missing launcher probe output; current shell scene: $("$binary" --current-scene)" >&2
        return 1
    fi
    diff -u <(printf '%s\n' "$expected" wayland unset unset unset) "$tmp/launched-env"
    echo "Verified a desktop entry launched from Start on the private Wayland display."
}
