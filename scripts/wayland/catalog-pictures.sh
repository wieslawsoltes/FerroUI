#!/bin/sh
# Pictures of the ControlCatalog on the Wayland backend, taken by the compositor.
#
#   scripts/wayland/sway-run.sh [--scale N] -- \
#       scripts/wayland/catalog-pictures.sh <catalog binary> <output directory> [software|egl] [interval ms]
#
# Runs inside a compositor that has the screen copy protocol of wlroots (sway-run.sh starts one):
# the catalog shows a handful of pages one after the other (FERROUI_SMOKE_SCREENSHOTS, with the
# pages of FERROUI_SMOKE_SCREENSHOT_PAGES when set), and after each page is selected `grim` writes
# a picture of the output as the compositor composed it: <mode>-NN-<page>.png. The pictures the
# catalog draws of itself through the render target of the framework go to
# <output directory>/framework-<mode>, for comparison.
#
# With CATALOG_POPUP=combobox (or menu) the catalog opens the first combo box (or menu item) of
# each page before the picture is taken (FERROUI_SMOKE_OPEN_POPUP), and the pictures are named
# <mode>-popup-NN-<page>.png: a popup is a surface of its own, which only the compositor's
# picture shows. Use it with FERROUI_SMOKE_SCREENSHOT_PAGES=ComboBox (or Menu).
#
# With CATALOG_DECORATED=1 the window of the catalog gives up the decorations of the compositor
# (FERROUI_SMOKE_DRAWN_DECORATIONS) and is made a floating window (swaymsg), so that the title
# bar, the border and the shadow the framework draws are in the picture:
# <mode>-decorated-NN-<page>.png. Use an output larger than the window (sway-run.sh --size).
#
# Needs: grim. Prints the number of pictures; fails when there is none or the catalog failed.
set -u

bin="$1"
out="$2"
mode="${3:-software}"
interval="${4:-2500}"

mkdir -p "$out" "$out/framework-$mode" || exit 1
log="$out/catalog-$mode.log"
: > "$log"

case "$mode" in
    egl) wayland=1 ;;
    *) wayland=software ;;
esac

# The settle time before a picture: most of the interval, so the page has been laid out, drawn
# and presented.
settle="$(awk "BEGIN { print ($interval * 0.7) / 1000 }")"

prefix="$mode"
if [ -n "${CATALOG_POPUP:-}" ]; then
    prefix="$mode-popup"
    export FERROUI_SMOKE_OPEN_POPUP="$CATALOG_POPUP"
fi

if [ -n "${CATALOG_DECORATED:-}" ]; then
    prefix="$mode-decorated"
    export FERROUI_SMOKE_DRAWN_DECORATIONS=1
fi

index=0
FERROUI_CATALOG_WAYLAND="$wayland" \
FERROUI_SMOKE_SCREENSHOTS="$out/framework-$mode" \
FERROUI_SMOKE_PAGES="$interval" \
    "$bin" 2>>"$log" | while IFS= read -r line; do
    echo "$line" >> "$log"
    case "$line" in
        "Selecting "*)
            page="$(echo "${line#Selecting }" | tr -c 'A-Za-z0-9\n' '-' | sed 's/--*/-/g; s/^-//; s/-$//')"
            index=$((index + 1))
            if [ -n "${CATALOG_DECORATED:-}" ] && [ "$index" -eq 1 ]; then
                swaymsg floating enable >>"$log" 2>&1
            fi
            sleep "$settle"
            grim "$out/$prefix-$(printf '%02d' "$index")-$page.png" 2>>"$log"
            ;;
    esac
done

pictures="$(ls "$out" | grep -c "^$prefix-[0-9][0-9]-.*\.png$")"
echo "catalog-pictures.sh: $pictures pictures of the compositor in $out ($mode)"
grep -E "^ControlCatalog: |panicked|Screenshots written|Screenshots: opening|Screenshots: a control|Screenshots: the control|^Popup placement:" "$log" | head -n 20
if [ -n "${CATALOG_POPUP:-}" ]; then
    # The catalog says where the compositor put the popup it opened, and whether that is at
    # the control the popup belongs to.
    if ! grep -q "^Popup placement: \[ok\]" "$log" || grep -q "^Popup placement: \[FAILED\]" "$log"; then
        echo "catalog-pictures.sh: the popup is not at its control" >&2
        exit 1
    fi
fi
if ! grep -q "ControlCatalog: the Wayland backend" "$log"; then
    echo "catalog-pictures.sh: the catalog did not select the Wayland backend" >&2
    exit 1
fi
if grep -q "panicked" "$log"; then
    echo "catalog-pictures.sh: the catalog panicked; the end of its log:" >&2
    tail -n 30 "$log" >&2
    exit 1
fi
test "$pictures" -gt 0
