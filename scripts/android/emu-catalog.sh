#!/bin/bash
# Runs the ControlCatalog on the emulator and takes a picture of the screen at a handful of its
# pages (docs/porting/android-platform.md, section 10): starts the virtual device ferroui_api36
# without a window, installs the package, starts the activity with the smoke options of the host
# (samples/ControlCatalog.Android/application.rs) as extras of the intent, takes a picture when the
# application reports that a page is shown, uninstalls and shuts the emulator down. Exits 0 when
# every page asked for was shown and pictured.
#
#   scripts/android/emu-catalog.sh [options]
#
# Options:
#   --keep            leave the emulator running and the application installed
#   --out DIR         where the pictures go (default: /Volumes/1TB-macOS/ferroui-screenshots/android
#                     when that volume exists, else <build directory>/emu-catalog)
#   --logs DIR        where the logs go (default: <build directory>/emu-catalog)
#   --apk FILE        the package to run (default: the debug package apk.sh builds)
#   --build           build the package first (scripts/android/apk.sh catalog)
#   --gpu MODE        the GPU mode of the emulator: swiftshader_indirect (default) or host
#   --pages "A;B"     the headers of the pages to show
#                     (default: Home;Buttons;TextBlock;TextBox;ListBox;Image;Calendar)
#   --page-ms N       how long each page is shown, in milliseconds (default 12000)
#   --software        render through the native window instead of EGL
#
# The emulator may only run while no virtual machine does on the development machine: this script is
# run by whoever serialises that.
set -uo pipefail

keep=0
out=""
logs=""
apk=""
build=0
gpu="swiftshader_indirect"
pages="Home;Buttons;TextBlock;TextBox;ListBox;Image;Calendar"
page_ms=12000
software=0
while [ $# -gt 0 ]; do
    case "$1" in
        --keep) keep=1 ;;
        --out) out="${2:?--out needs a value}"; shift ;;
        --logs) logs="${2:?--logs needs a value}"; shift ;;
        --apk) apk="${2:?--apk needs a value}"; shift ;;
        --build) build=1 ;;
        --gpu) gpu="${2:?--gpu needs a value}"; shift ;;
        --pages) pages="${2:?--pages needs a value}"; shift ;;
        --page-ms) page_ms="${2:?--page-ms needs a value}"; shift ;;
        --software) software=1 ;;
        -h|--help) sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

repo="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=scripts/android/env.sh
. "$repo/scripts/android/env.sh" || exit 1
# shellcheck source=scripts/android/emu-common.sh
. "$repo/scripts/android/emu-common.sh"

build_dir="${FERROUI_ANDROID_BUILD_DIR:-${CARGO_TARGET_DIR:-$repo/target}/android}"
logs="${logs:-$build_dir/emu-catalog}"
if [ -z "$out" ]; then
    if [ -d /Volumes/1TB-macOS ]; then
        out="/Volumes/1TB-macOS/ferroui-screenshots/android"
    else
        out="$logs"
    fi
fi
apk="${apk:-$build_dir/catalog-arm64-v8a-debug.apk}"
application_id="org.ferroui.controlcatalog"
activity="$application_id/org.ferroui.android.FerroMainActivity"
mkdir -p "$out" "$logs"

if [ "$build" = 1 ]; then
    "$repo/scripts/android/apk.sh" catalog || exit 1
fi
if [ ! -f "$apk" ]; then
    echo "the package does not exist: $apk (build it with scripts/android/apk.sh catalog, or pass --build)" >&2
    exit 1
fi

cleanup() {
    if [ "$keep" = 0 ]; then
        if [ "$emulator_started" = 1 ]; then
            ADB_TIMEOUT=60 adb_do uninstall "$application_id" >/dev/null 2>&1 || true
        fi
        emu_stop
    elif [ "$emulator_started" = 1 ]; then
        echo "== the emulator keeps running ($FERROUI_SERIAL); stop it with: $ADB -s $FERROUI_SERIAL emu kill"
    fi
}
trap cleanup EXIT

emu_start "$gpu" "$logs/emulator.log" || exit 1
emu_install "$apk" "$application_id" || exit 1
adb_shell settings put secure show_ime_with_hard_keyboard 1 >/dev/null 2>&1 || true

if [ "$software" = 1 ]; then
    emu_write_file "$application_id" ferroui.properties "rendering=software"
fi
# The pictures of an earlier run have other numbers when the pages changed.
rm -f "$out"/catalog-*.png
adb_do logcat -c >/dev/null 2>&1 || true
echo "== starting the catalog: pages $pages, $page_ms ms each"
adb_shell am start -W -n "$activity" \
    --es FERROUI_SMOKE_PAGES "$page_ms" \
    --es FERROUI_SMOKE_PAGE_LIST "'$pages'" \
    --es FERROUI_SMOKE_EXIT_MS 1000 > "$logs/am-start.txt" 2>&1 || true

# The pages asked for, to know how long to wait and what to expect.
expected=0
old_ifs="$IFS"
IFS=';'
for page in $pages; do
    [ -n "$page" ] && expected=$((expected + 1))
done
IFS="$old_ifs"

pictured=0
finished=0
waited=0
# The start of a debug build on a software GPU is slow: a minute for the first view, then the pages.
limit=$((180 + expected * (page_ms / 1000 + 20)))
while [ "$waited" -lt "$limit" ]; do
    lines="$(ADB_TIMEOUT=20 adb_do logcat -d -v raw -s ferroui-catalog:V 2>/dev/null | tr -d '\r' || true)"
    shown="$(printf '%s\n' "$lines" | grep -c '^PAGE-SHOWN ' || true)"
    while [ "$pictured" -lt "$shown" ]; do
        line="$(printf '%s\n' "$lines" | grep '^PAGE-SHOWN ' | sed -n "$((pictured + 1))p")"
        header="${line#PAGE-SHOWN * }"
        name="$(printf '%s' "$header" | tr -c 'A-Za-z0-9' '_')"
        echo "   page $pictured: $header"
        emu_screencap "$out/catalog-$(printf '%02d' "$pictured")-$name.png" || true
        if [ "$header" = TextBox ]; then
            # The page is a list of samples: the first one is opened, a tap into its text box
            # makes the box the client of the input method, and the picture is taken once the
            # system says the soft keyboard is shown, with a word typed into the box.
            adb_shell input tap ${FERROUI_CATALOG_SAMPLE_TAP:-400 620} >/dev/null 2>&1 || true
            sleep 2
            adb_shell input tap ${FERROUI_CATALOG_TEXTBOX_TAP:-516 652} >/dev/null 2>&1 || true
            shown_ime=0
            for _ in 1 2 3 4 5 6 7 8 9 10; do
                if adb_shell dumpsys input_method 2>/dev/null | grep -q 'mInputShown=true'; then
                    shown_ime=1
                    break
                fi
                sleep 1
            done
            echo "   the soft keyboard is shown: $shown_ime"
            adb_shell input text FerroUI >/dev/null 2>&1 || true
            sleep 1
            emu_screencap "$out/catalog-$(printf '%02d' "$pictured")-$name-keyboard.png" || true
        fi
        pictured=$((pictured + 1))
    done
    if printf '%s\n' "$lines" | grep -q '^CATALOG DONE'; then
        finished=1
        break
    fi
    if ADB_TIMEOUT=20 adb_do logcat -d -b crash -v raw 2>/dev/null | grep -q "$application_id"; then
        echo "== the application crashed"
        break
    fi
    sleep 1
    waited=$((waited + 1))
done

if [ "$pictured" = 0 ]; then
    # Whatever is on the screen, to see how far the application came.
    emu_screencap "$out/catalog-start.png" || true
fi
ADB_TIMEOUT=30 adb_do logcat -d -v threadtime > "$logs/logcat.txt" 2>/dev/null || true
ADB_TIMEOUT=30 adb_do logcat -d -b crash -v threadtime > "$logs/crash.txt" 2>/dev/null || true
ADB_TIMEOUT=20 adb_do logcat -d -v raw -s ferroui-catalog:V ferroui:V 2>/dev/null | tr -d '\r' \
    | grep -v '^--------- beginning of' | sed 's/^/   /'

echo
echo "pictures: $out"
echo "logs:     $logs"
if [ "$finished" = 1 ] && [ "$pictured" = "$expected" ]; then
    echo "CATALOG PASSED: $pictured page(s) shown and pictured"
    exit 0
fi
echo "CATALOG FAILED: $pictured of $expected page(s) pictured, finished: $finished"
echo "   the lines of the backend and of the runtime about the application (logcat: $logs/logcat.txt):"
grep -E " (ferroui|ferroui-catalog|AndroidRuntime|DEBUG|libc)( |:)" "$logs/logcat.txt" | tail -n 40 | sed 's/^/   /'
exit 1
