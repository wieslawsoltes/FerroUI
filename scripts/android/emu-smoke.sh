#!/bin/bash
# Runs the smoke application of the Android platform on the emulator
# (docs/porting/android-platform.md, section 10): starts the virtual device ferroui_api36 without a
# window, installs the package, runs the application once per rendering mode, does on the device
# what it asks for (touch input, keys, the night mode), prints its check lines, pulls its report, takes a picture of the screen,
# uninstalls and shuts the emulator down. Exits 0 only when every run passed.
#
#   scripts/android/emu-smoke.sh [options]
#
# Options:
#   --keep            leave the emulator running and the application installed
#   --out DIR         where the logs, the reports and the pictures go
#                     (default: <build directory>/emu-smoke, see scripts/android/apk.sh)
#   --apk FILE        the package to run (default: the debug package apk.sh builds)
#   --build           build the package first (scripts/android/apk.sh smoke)
#   --gpu MODE        the GPU mode of the emulator: swiftshader_indirect (default) or host
#   --modes "A B"     the rendering modes to run, of egl and software (default: both)
#   --no-input        do nothing the application asks for but pictures (it then skips the checks
#                     that need the script: touch, keys, the night mode)
#
# The emulator may only run while no virtual machine does on the development machine: this script is
# run by whoever serialises that.
set -uo pipefail

keep=0
out=""
apk=""
build=0
gpu="swiftshader_indirect"
modes="egl software"
input=1
while [ $# -gt 0 ]; do
    case "$1" in
        --keep) keep=1 ;;
        --out) out="${2:?--out needs a value}"; shift ;;
        --apk) apk="${2:?--apk needs a value}"; shift ;;
        --build) build=1 ;;
        --gpu) gpu="${2:?--gpu needs a value}"; shift ;;
        --modes) modes="${2:?--modes needs a value}"; shift ;;
        --no-input) input=0 ;;
        -h|--help) sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
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
out="${out:-$build_dir/emu-smoke}"
apk="${apk:-$build_dir/smoke-arm64-v8a-debug.apk}"
application_id="org.ferroui.smoke"
activity="$application_id/org.ferroui.android.FerroMainActivity"
mkdir -p "$out"

if [ "$build" = 1 ]; then
    "$repo/scripts/android/apk.sh" smoke || exit 1
fi
if [ ! -f "$apk" ]; then
    echo "the package does not exist: $apk (build it with scripts/android/apk.sh smoke, or pass --build)" >&2
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

emu_start "$gpu" "$out/emulator.log" || exit 1
emu_install "$apk" "$application_id" || exit 1
# An emulator has a hardware keyboard, and the system shows no soft keyboard beside one unless told.
adb_shell settings put secure show_ime_with_hard_keyboard 1 >/dev/null 2>&1 || true

# What the application asks for with a line "SCRIPT <command> <arguments>":
#   picture NAME          a picture of the screen, <out>/NAME-<mode>.png
#   tap X Y               a tap at a pixel of the screen
#   swipe X1 Y1 X2 Y2 MS  a swipe
#   keyevent CODE...      key codes, each pressed and released
#   text TEXT             text, as the keys of a virtual keyboard
#   night yes|no          the night mode of the system
#   home                  the home button: the application goes to the background
#   resume                the activity is started as the launcher starts it: an activity that
#                         is in the background comes back, one that finished is created again
#   rotate 0..3           the rotation of the display, in quarter turns
#   view URI              an intent with the URI for the activity that runs
#   back                  the back button
# Everything but a picture is left out with --no-input.
night_changed=0
rotated=0
# How the launcher starts the activity: an intent that equals this one brings its task back.
launch="-a android.intent.action.MAIN -c android.intent.category.LAUNCHER"
smoke_request() {
    local mode="$1" command="$2"
    shift 2
    if [ "$command" = picture ]; then
        emu_screencap "$out/$1-$mode.png" || true
        [ "$1" = smoke ] && pictured=1
        return 0
    fi
    [ "$input" = 1 ] || return 0
    echo "   script: $command $*"
    case "$command" in
        tap) adb_shell input tap "$1" "$2" >/dev/null 2>&1 || true; sleep 1 ;;
        swipe) adb_shell input swipe "$1" "$2" "$3" "$4" "$5" >/dev/null 2>&1 || true ;;
        keyevent) adb_shell input keyevent "$@" >/dev/null 2>&1 || true ;;
        text) adb_shell input text "$1" >/dev/null 2>&1 || true ;;
        night) night_changed=1; adb_shell cmd uimode night "$1" >/dev/null 2>&1 || true ;;
        home) adb_shell input keyevent HOME >/dev/null 2>&1 || true ;;
        resume) adb_shell am start $launch -n "$activity" >/dev/null 2>&1 || true ;;
        rotate)
            rotated=1
            adb_shell settings put system accelerometer_rotation 0 >/dev/null 2>&1 || true
            adb_shell settings put system user_rotation "$1" >/dev/null 2>&1 || true ;;
        view) adb_shell am start --activity-single-top -a android.intent.action.VIEW -d "'$1'" -n "$activity" >/dev/null 2>&1 || true ;;
        back) adb_shell input keyevent BACK >/dev/null 2>&1 || true ;;
        *) echo "   script: unknown request '$command'" ;;
    esac
}

failed=0
for mode in $modes; do
    echo
    echo "== smoke run: rendering mode $mode"
    adb_shell am force-stop "$application_id" >/dev/null 2>&1 || true
    emu_write_file "$application_id" smoke.properties "mode=$mode
input=$input"
    adb_shell "run-as $application_id rm -f files/smoke-report.txt" >/dev/null 2>&1 || true
    adb_do logcat -c >/dev/null 2>&1 || true
    if ! adb_shell am start -W $launch -n "$activity" > "$out/am-start-$mode.txt" 2>&1; then
        echo "[FAIL] the activity could not be started:"
        cat "$out/am-start-$mode.txt"
        failed=1
        continue
    fi

    # The lines of the application, until it has written its report. A line "SCRIPT <command>
    # <arguments>" asks for something to be done on the device; each is done once, in order.
    executed=0
    pictured=0
    done_=0
    waited=0
    limit="${FERROUI_SMOKE_TIMEOUT:-360}"
    lines=""
    while [ "$waited" -lt "$limit" ]; do
        lines="$(ADB_TIMEOUT=20 adb_do logcat -d -v raw -s ferroui-smoke:V 2>/dev/null | tr -d '\r' || true)"
        requests="$(printf '%s\n' "$lines" | sed -n 's/^SCRIPT //p')"
        count=0
        while IFS= read -r request; do
            [ -n "$request" ] || continue
            count=$((count + 1))
            [ "$count" -gt "$executed" ] || continue
            executed=$count
            smoke_request "$mode" $request
        done <<EOF
$requests
EOF
        if printf '%s\n' "$lines" | grep -q '^REPORT WRITTEN$'; then
            done_=1
            break
        fi
        # A crash ends the wait.
        if ADB_TIMEOUT=20 adb_do logcat -d -b crash -v raw 2>/dev/null | grep -q "$application_id"; then
            break
        fi
        sleep 2
        waited=$((waited + 2))
    done

    if [ "$pictured" = 0 ]; then
        emu_screencap "$out/smoke-$mode.png" || true
    fi
    if [ "$night_changed" = 1 ]; then
        # A run that ended early may have left the night mode on.
        adb_shell cmd uimode night no >/dev/null 2>&1 || true
        night_changed=0
    fi
    if [ "$rotated" = 1 ]; then
        adb_shell settings put system user_rotation 0 >/dev/null 2>&1 || true
        rotated=0
    fi
    ADB_TIMEOUT=30 adb_do logcat -d -v threadtime > "$out/logcat-$mode.txt" 2>/dev/null || true
    ADB_TIMEOUT=30 adb_do logcat -d -b crash -v threadtime > "$out/crash-$mode.txt" 2>/dev/null || true
    emu_read_file "$application_id" smoke-report.txt > "$out/smoke-report-$mode.txt"

    printf '%s\n' "$lines" | grep -v '^--------- beginning of' | sed 's/^/   /'
    if [ "$done_" = 1 ] && grep -q '^RESULT: PASS$' "$out/smoke-report-$mode.txt"; then
        echo "== $mode: PASS (report: $out/smoke-report-$mode.txt)"
    else
        failed=1
        if [ "$done_" = 0 ]; then
            echo "== $mode: FAIL: the application did not finish its report within $limit s"
            echo "   the lines of the backend and of the runtime about the application (logcat: $out/logcat-$mode.txt):"
            grep -E " (ferroui|AndroidRuntime|DEBUG|libc)( |:)" "$out/logcat-$mode.txt" | tail -n 40 | sed 's/^/   /'
        else
            echo "== $mode: FAIL (report: $out/smoke-report-$mode.txt, logcat: $out/logcat-$mode.txt)"
        fi
    fi
done

echo
echo "output directory: $out"
if [ "$failed" = 0 ]; then
    echo "SMOKE PASSED"
    exit 0
fi
echo "SMOKE FAILED"
exit 1
