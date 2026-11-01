# Optional phase of ferro-compositor.sh. Stage the actual installed desktop
# entry, recording launch and protocol traffic around the unmodified catalog.
install_gallery_probe() {
    install -Dm644 "$here/org.ferroui.WinUIGallery.desktop" "$XDG_DATA_HOME/applications/org.ferroui.WinUIGallery.desktop"
    mkdir -p "$scratch/gallery-bin"
    cat >"$scratch/gallery-bin/control-catalog-desktop" <<'WRAPPER'
#!/bin/sh
printf '%s\n' "$$" >"$FERRO_TEST_CATALOG_OUT/catalog.pid"
printf '%s\n' "$WAYLAND_DISPLAY" "$FERROUI_CATALOG_THEME" "$FERROUI_CATALOG_WAYLAND" >"$FERRO_TEST_CATALOG_OUT/catalog-launch.txt"
export WAYLAND_DEBUG=client FERROUI_CATALOG_THEME_VARIANT=light FERROUI_SMOKE_WINDOW_SIZE=965x615 FERROUI_SMOKE_EXIT_MS=35000
exec "$FERRO_TEST_CATALOG" >"$FERRO_TEST_CATALOG_OUT/catalog-protocol.log" 2>&1
WRAPPER
    chmod 755 "$scratch/gallery-bin/control-catalog-desktop"
    export PATH="$scratch/gallery-bin:$PATH" FERRO_TEST_CATALOG_OUT="$output"
}

stop_gallery_probe() {
    [[ -f $output/catalog.pid ]] || return 0
    local pid
    read -r pid <"$output/catalog.pid"
    if [[ $(readlink "/proc/$pid/exe" || :) == "$FERRO_TEST_CATALOG" ]]; then
        kill "$pid" 2>/dev/null || :
    fi
}

test_composed_gallery() {
    local attempt delta colors stable=0
    grim -s 1 -o HEADLESS-1 "$scratch/gallery-before.png"
    "$binary" --platform=wayland --show=applications
    # Wait for the WinUI launcher search field to be painted on the monitor.
    for ((attempt=0;attempt<80;attempt++)); do
        grim -s 1 -o HEADLESS-1 "$scratch/gallery-launcher.png"
        local field
        field=$(convert "$scratch/gallery-launcher.png" -crop 120x8+630+556 +repage -colorspace Gray -format '%[fx:mean]' info:)
        awk -v v="$field" 'BEGIN {exit !(v>.85)}' && break
        sleep .07
    done
    wtype -M ctrl -k a -m ctrl -k BackSpace -s 120 'FerroUI WinUI Gallery' -s 150 -k Return
    wait_for test -s "$output/catalog-launch.txt"
    diff -u <(printf '%s\n' "$inner" winui software) "$output/catalog-launch.txt"
    wait_for grep -q '\.get_toplevel(' "$output/catalog-protocol.log"
    wait_for grep -q '\.set_title("FerroUI Control Gallery")' "$output/catalog-protocol.log"
    wait_for grep -q '\.attach(wl_buffer' "$output/catalog-protocol.log"
    # Both the visible output and the parent compositor tree matter. The
    # gallery supplies its own wl_buffer while Sway sees only the FerroUI host.
    for ((attempt=0;attempt<80;attempt++)); do
        grim -s 1 -o HEADLESS-1 "$scratch/gallery-frame.png"
        delta=$(compare -fuzz 4% -metric AE "$scratch/gallery-before.png" "$scratch/gallery-frame.png" null: 2>&1) || [[ $? == 1 ]]
        colors=$(convert "$scratch/gallery-frame.png" -crop 780x420+65+90 +repage -format '%k' info:)
        if (( colors>200 )) && awk -v n="$delta" 'BEGIN {exit !(n>150000)}' && [[ -f $scratch/gallery-last.png ]]; then
            delta=$(compare -fuzz 1% -metric AE "$scratch/gallery-last.png" "$scratch/gallery-frame.png" null: 2>&1) || [[ $? == 1 ]]
            if awk -v n="$delta" 'BEGIN {exit !(n<1200)}'; then
                stable=$((stable+1))
                if (( stable >= 2 )); then break; fi
            else
                stable=0
            fi
        fi
        mv "$scratch/gallery-frame.png" "$scratch/gallery-last.png"; sleep .12
    done
    (( stable>=2 )) && only_host || fail "Could not verify a stable ControlCatalog window inside FerroUI"
    cp "$scratch/gallery-frame.png" "$output/winui-control-catalog.png"
    # Keep the following GTK/Qt checks independent of the optional gallery.
    local pid
    read -r pid <"$output/catalog.pid"
    stop_gallery_probe
    for ((attempt=0;attempt<80;attempt++)); do
        [[ $(readlink "/proc/$pid/exe" || :) != "$FERRO_TEST_CATALOG" ]] && break
        sleep .05
    done
    echo 'Verified WinUI ControlCatalog launched from Start and presented its own Wayland buffers inside the FerroUI output.'
}
