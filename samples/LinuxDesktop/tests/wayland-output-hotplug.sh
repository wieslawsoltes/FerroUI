#!/usr/bin/env bash
# Optional phase of wayland-session.sh. Exercise the shell on a real added wl_output.
set -euo pipefail
[[ ${FERRO_DESKTOP_NESTED:-} == 1 && ${WLR_BACKENDS:-} == headless && ${FERRO_TEST_OUTPUT_HOTPLUG:-} == 1 && $# == 2 ]] ||
    { echo 'Run through FERRO_TEST_OUTPUT_HOTPLUG=1 tests/wayland-session.sh' >&2; exit 2; }
binary=$1
frames=$2/hotplug
mkdir -p "$frames"
scratch=$(mktemp -d "$XDG_RUNTIME_DIR/hotplug.XXXXXX")
head=
cleanup() {
    if [[ -n $head ]]; then swaymsg "output $head unplug" >/dev/null || :; fi
    rm -rf -- "$scratch"
}
trap cleanup EXIT

fail() { echo "Wayland output test: $*" >&2; exit 1; }
outputs() { swaymsg -rt get_outputs; }
command_ok() { swaymsg -r "$1" | jq -e 'length > 0 and all(.[]; .success)' >/dev/null; }
wait_for() {
    local i
    for ((i=0;i<120;i++)); do if "$@"; then return; fi; sleep .05; done
    fail "Timed out waiting for $*"
}
scene_is() { [[ $("$binary" --platform=wayland --current-scene) == "$1" ]]; }
active() { outputs | jq -e --arg out "$1" 'any(.[]; .name==$out and .active)' >/dev/null; }
disabled() { outputs | jq -e --arg out "$1" 'any(.[]; .name==$out and (.active|not))' >/dev/null; }
unplugged() { outputs | jq -e --arg out "$1" 'all(.[]; .name!=$out)' >/dev/null; }
reserved() {
    local rect
    rect=$(outputs | jq -c --arg out "$1" '.[] | select(.name==$out and .active) | .rect')
    [[ -n $rect ]] && swaymsg -rt get_workspaces | jq -e --arg out "$1" --argjson rect "$rect" \
        'any(.[]; .output==$out and .rect.width==$rect.width and .rect.height < $rect.height and .rect.height > ($rect.height/2))' >/dev/null
}
changed_pixels() {
    local pixels status=0
    pixels=$(compare -fuzz 3% -metric AE "$1" "$2" null: 2>&1) || status=$?
    (( status <= 1 )) || fail "$pixels"
    printf '%.0f\n' "$pixels"
}
capture() {
    local out=$1 dest=$2 dims i actual changed previous="$scratch/previous.png" next="$scratch/next.png"
    dims=$(outputs | jq -r --arg out "$out" '.[] | select(.name==$out and .active) | .rect | "\(.width)x\(.height)"')
    [[ -n $dims ]] || fail "No logical dimensions for $out"
    rm -f "$previous"
    for ((i=0;i<40;i++)); do
        if grim -s 1 -o "$out" "$next"; then
            actual=$(identify -format '%wx%h' "$next")
            if [[ $actual == "$dims" && $(identify -format '%k' "$next") -gt 60 && -f $previous ]]; then
                changed=$(changed_pixels "$previous" "$next")
                if (( changed < 500 )); then cp "$next" "$dest"; return; fi
            fi
            mv "$next" "$previous"
        fi
        sleep .1
    done
    [[ ! -f $previous ]] || cp "$previous" "$dest"
    fail "$out did not present a stable rendered frame at $dims"
}
fraction_changed() {
    local before=$1 after=$2 pixels changed
    pixels=$(identify -format '%[fx:w*h]' "$before")
    changed=$(changed_pixels "$before" "$after")
    awk -v d="$changed" -v p="$pixels" 'BEGIN { print d/p }'
}
# The Windows Quick Settings sheet is anchored at the bottom right in logical
# coordinates. Compare its real rendered interior against a captured reference.
# Live taskbar/window-list repaints elsewhere cannot impersonate a second sheet.
overlay_interior() { convert "$1" -gravity SouthEast -crop 230x125+42+83 +repage "$2"; }
matches_overlay() {
    local changed
    overlay_interior "$1" "$scratch/candidate.png"
    changed=$(changed_pixels "$scratch/overlay-template.png" "$scratch/candidate.png")
    (( changed < 230*125/6 ))
}
different() { local f; f=$(fraction_changed "$1" "$2"); awk -v f="$f" 'BEGIN { exit !(f > .025) }'; }
similar() { local f; f=$(fraction_changed "$1" "$2"); awk -v f="$f" 'BEGIN { exit !(f < .025) }'; }

outputs > "$frames/before.json"
mapfile -t original < <(jq -r '.[] | select(.active) | .name' "$frames/before.json" | sort)
[[ ${#original[@]} == 2 && ${original[0]} == HEADLESS-1 && ${original[1]} == HEADLESS-2 ]] || fail 'Expected two original virtual heads'
wait_for scene_is desktop
# Keep static, empty workspaces as compositor-image references. Existing clients
# stay mapped on their own workspaces throughout the test.
for i in 0 1; do
    command_ok "focus output ${original[i]}; workspace --no-auto-back-and-forth number $((240+i))"
    wait_for reserved "${original[i]}"
    capture "${original[i]}" "$scratch/base-$i.png"
done
command_ok create_output
head=$(outputs | jq -r --slurpfile before "$frames/before.json" '[.[] | .name] - [$before[0][] | .name] | .[]')
[[ $head =~ ^HEADLESS-[0-9]+$ ]] || fail 'Could not identify the added virtual head'
command_ok "output $head mode 1024x720 position -1024 0 scale 1"
command_ok "focus output $head"
wait_for reserved "$head"
capture "$head" "$frames/attached.png"
# The center excludes the taskbar. This catches a second compositor background
# surface covering the Ferro wallpaper even when visible panel text has many colors.
wallpaper_colors=$(convert "$frames/attached.png" -gravity Center -crop 82%x72%+0+0 +repage -format '%k' info:)
(( wallpaper_colors > 60 )) || fail 'New monitor is missing the Ferro wallpaper'
"$binary" --platform=wayland --show=settings
wait_for scene_is settings
capture "$head" "$frames/settings.png"
overlay_interior "$frames/settings.png" "$scratch/overlay-template.png"
if matches_overlay "$frames/attached.png"; then fail 'Settings reference is indistinguishable from wallpaper'; fi
different "$frames/attached.png" "$frames/settings.png" || fail 'Overlay did not draw on the newly focused monitor'

command_ok "output $head mode 1280x960 position -768 0 scale 1.25 transform 90"
outputs > "$frames/rotated.json"
jq -e --arg out "$head" 'any(.[]; .name==$out and .rect.x == -768 and .rect.width == 768 and .rect.height == 1024 and .scale == 1.25)' "$frames/rotated.json" >/dev/null
wait_for reserved "$head"
capture "$head" "$frames/rotated-settings.png"
wtype -s 250 -k Escape; wait_for scene_is desktop
capture "$head" "$frames/rotated-desktop.png"
matches_overlay "$frames/rotated-settings.png" || fail 'Rotated monitor did not show the same settings sheet'
if matches_overlay "$frames/rotated-desktop.png"; then fail 'Settings sheet remained visible after dismissal'; fi
different "$frames/rotated-settings.png" "$frames/rotated-desktop.png" || fail 'Reconfigured monitor lost its overlay'
"$binary" --platform=wayland --show=settings
wait_for scene_is settings
capture "$head" "$scratch/before-disable.png"
different "$frames/rotated-desktop.png" "$scratch/before-disable.png" || fail 'Overlay did not reopen'

command_ok "output $head disable"
wait_for disabled "$head"
wait_for scene_is settings
# Exactly one remaining output should now display the migrated open overlay.
migrated=0
for i in 0 1; do
    capture "${original[i]}" "$frames/remaining-$i.png"
    if matches_overlay "$scratch/base-$i.png"; then fail "Original monitor already showed Quick Settings"; fi
    if matches_overlay "$frames/remaining-$i.png"; then migrated=$((migrated+1)); fi
done
(( migrated == 1 )) || fail "Expected one migrated overlay, found $migrated"
command_ok "output $head enable"
wait_for active "$head"
wait_for reserved "$head"
capture "$head" "$frames/restored.png"
similar "$frames/rotated-desktop.png" "$frames/restored.png" || fail 'Restored monitor did not repaint its desktop and panel'

command_ok "output $head unplug"; wait_for unplugged "$head"
head=
wtype -s 250 -k Escape; wait_for scene_is desktop
for i in 0 1; do
    capture "${original[i]}" "$frames/finished-$i.png"
    if matches_overlay "$frames/finished-$i.png"; then fail "Overlay remained on ${original[i]}"; fi
    wait_for reserved "${original[i]}"
    colors=$(convert "$frames/finished-$i.png" -gravity Center -crop 82%x72%+0+0 +repage -format '%k' info:)
    (( colors > 60 )) || fail "Surviving output ${original[i]} did not repaint its wallpaper"
done
outputs > "$frames/after.json"
echo 'Verified virtual output creation, rotation, fractional scale, disable, overlay migration, restoration and unplug using compositor pixels.'
