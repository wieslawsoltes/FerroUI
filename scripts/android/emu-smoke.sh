#!/bin/bash
# Runs the smoke application of the Android platform on the emulator
# (docs/porting/android-platform.md, section 10): starts the virtual device ferroui_api36 without a
# window, installs the package, runs the application once per rendering mode, injects the touch
# input it asks for, prints its check lines, pulls its report, takes a picture of the screen,
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
#   --no-input        do not inject touch input (the application then skips its touch checks)
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

failed=0
for mode in $modes; do
    echo
    echo "== smoke run: rendering mode $mode"
    adb_shell am force-stop "$application_id" >/dev/null 2>&1 || true
    emu_write_file "$application_id" smoke.properties "mode=$mode
input=$input"
    adb_shell "run-as $application_id rm -f files/smoke-report.txt" >/dev/null 2>&1 || true
    adb_do logcat -c >/dev/null 2>&1 || true
    if ! adb_shell am start -W -n "$activity" > "$out/am-start-$mode.txt" 2>&1; then
        echo "[FAIL] the activity could not be started:"
        cat "$out/am-start-$mode.txt"
        failed=1
        continue
    fi

    # The lines of the application, until it has written its report.
    injected=0
    pictured=0
    done_=0
    waited=0
    limit="${FERROUI_SMOKE_TIMEOUT:-120}"
    lines=""
    while [ "$waited" -lt "$limit" ]; do
        lines="$(ADB_TIMEOUT=20 adb_do logcat -d -v raw -s ferroui-smoke:V 2>/dev/null | tr -d '\r' || true)"
        target="$(printf '%s\n' "$lines" | sed -n 's/^INPUT-TARGET \([0-9]*\) \([0-9]*\)$/\1 \2/p' | tail -n 1)"
        if [ -n "$target" ] && [ "$pictured" = 0 ]; then
            # The frames were read back: this is what is on the screen.
            emu_screencap "$out/smoke-$mode.png" || true
            pictured=1
        fi
        if [ -n "$target" ] && [ "$injected" = 0 ] && [ "$input" = 1 ]; then
            x="${target% *}"
            y="${target#* }"
            echo "   injecting a tap at $x,$y and a swipe from it"
            adb_shell input tap "$x" "$y" >/dev/null 2>&1 || true
            sleep 1
            adb_shell input swipe "$x" "$y" "$x" "$((y + 300))" 400 >/dev/null 2>&1 || true
            injected=1
        fi
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
