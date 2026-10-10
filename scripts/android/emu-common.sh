# Sourced by the emulator scripts of this directory: starting and stopping the emulator, waiting for
# the system, and the adb calls with timeouts (docs/porting/android-platform.md, section 10).
#
# The emulator may only run while no virtual machine does on the development machine: these scripts
# are started by whoever serialises that, never by a build.
#
#   ANDROID_HOME       the SDK
#   ANDROID_AVD_HOME   where the virtual devices are (the emulator's own default when not set)
#   FERROUI_AVD        the virtual device (default ferroui_api36)
#   FERROUI_EMU_PORT   the console port of the emulator (default 5584: the serial is emulator-5584)

FERROUI_AVD="${FERROUI_AVD:-ferroui_api36}"
FERROUI_EMU_PORT="${FERROUI_EMU_PORT:-5584}"
FERROUI_SERIAL="emulator-$FERROUI_EMU_PORT"
ADB="$ANDROID_HOME/platform-tools/adb"
EMULATOR="$ANDROID_HOME/emulator/emulator"
emulator_pid=""
emulator_started=0

# Runs a command with a limit of seconds.
with_timeout() {
    local seconds="$1"
    shift
    perl -e 'alarm shift; exec @ARGV' "$seconds" "$@"
}

adb_do() {
    with_timeout "${ADB_TIMEOUT:-60}" "$ADB" -s "$FERROUI_SERIAL" "$@"
}

adb_shell() {
    adb_do shell "$@"
}

# emu_start <gpu mode> <log file>: starts the virtual device without a window and waits until the
# system has booted. Fails when the device does not exist, when another emulator already uses the
# port, and when the boot takes longer than FERROUI_EMU_BOOT_TIMEOUT seconds (default 420).
emu_start() {
    local gpu="$1" log="$2"
    if [ ! -x "$EMULATOR" ] || [ ! -x "$ADB" ]; then
        echo "the emulator or adb is missing under $ANDROID_HOME (packages emulator and platform-tools)" >&2
        return 1
    fi
    if ! "$EMULATOR" -list-avds 2>/dev/null | grep -qx "$FERROUI_AVD"; then
        echo "the virtual device $FERROUI_AVD does not exist. Create it with:" >&2
        echo "  $ANDROID_HOME/cmdline-tools/latest/bin/avdmanager create avd -n $FERROUI_AVD -k \"system-images;android-36;google_apis;arm64-v8a\" -d pixel_7" >&2
        return 1
    fi
    "$ADB" start-server >/dev/null 2>&1 || true
    if "$ADB" devices | grep -q "^$FERROUI_SERIAL"; then
        echo "a device $FERROUI_SERIAL is already running; stop it first ($ADB -s $FERROUI_SERIAL emu kill)" >&2
        return 1
    fi

    echo "== starting the emulator: $FERROUI_AVD, -gpu $gpu, port $FERROUI_EMU_PORT (log: $log)"
    "$EMULATOR" -avd "$FERROUI_AVD" -port "$FERROUI_EMU_PORT" -no-window -no-audio -no-boot-anim \
        -no-snapshot -no-metrics -gpu "$gpu" > "$log" 2>&1 &
    emulator_pid=$!
    emulator_started=1

    local limit="${FERROUI_EMU_BOOT_TIMEOUT:-420}" waited=0 booted=""
    while [ "$waited" -lt "$limit" ]; do
        if ! kill -0 "$emulator_pid" 2>/dev/null; then
            echo "the emulator ended while starting; the end of its log:" >&2
            tail -n 30 "$log" >&2
            emulator_started=0
            return 1
        fi
        booted="$(ADB_TIMEOUT=10 adb_shell getprop sys.boot_completed 2>/dev/null | tr -d '\r\n' || true)"
        if [ "$booted" = "1" ]; then
            break
        fi
        sleep 3
        waited=$((waited + 3))
    done
    if [ "$booted" != "1" ]; then
        echo "the system did not finish booting within $limit s; the end of the log of the emulator:" >&2
        tail -n 30 "$log" >&2
        return 1
    fi
    # The package manager answers a little after the boot is reported.
    waited=0
    while [ "$waited" -lt 60 ]; do
        if ADB_TIMEOUT=10 adb_shell pm path android 2>/dev/null | grep -q '^package:'; then
            break
        fi
        sleep 2
        waited=$((waited + 2))
    done
    # The lock screen away, the screen on and kept on.
    adb_shell input keyevent KEYCODE_WAKEUP >/dev/null 2>&1 || true
    adb_shell wm dismiss-keyguard >/dev/null 2>&1 || true
    adb_shell settings put system screen_off_timeout 1800000 >/dev/null 2>&1 || true
    echo "== booted: Android $(adb_shell getprop ro.build.version.release | tr -d '\r\n') (API $(adb_shell getprop ro.build.version.sdk | tr -d '\r\n')), $(adb_shell getprop ro.product.cpu.abi | tr -d '\r\n'), $(adb_shell wm size | tr -d '\r' | tr '\n' ' ')$(adb_shell wm density | tr -d '\r' | tr '\n' ' ')"
}

# Shuts the emulator down, if this script started it.
emu_stop() {
    if [ "$emulator_started" = 1 ]; then
        echo "== shutting the emulator down"
        ADB_TIMEOUT=20 adb_do emu kill >/dev/null 2>&1 || true
        local waited=0
        while [ -n "$emulator_pid" ] && kill -0 "$emulator_pid" 2>/dev/null && [ "$waited" -lt 30 ]; do
            sleep 1
            waited=$((waited + 1))
        done
        if [ -n "$emulator_pid" ] && kill -0 "$emulator_pid" 2>/dev/null; then
            kill "$emulator_pid" 2>/dev/null || true
        fi
        emulator_started=0
    fi
}

# emu_install <apk> <application id>: replaces the application with the package.
emu_install() {
    local apk="$1" application_id="$2"
    echo "== installing $apk"
    ADB_TIMEOUT=60 adb_do uninstall "$application_id" >/dev/null 2>&1 || true
    ADB_TIMEOUT=300 adb_do install -r -t "$apk"
}

# emu_write_file <application id> <file under files/> <content>: writes a file of the application.
# The application is debuggable, so the shell may act as it.
emu_write_file() {
    local application_id="$1" name="$2" content="$3"
    adb_shell "run-as $application_id sh -c 'mkdir -p files && cat > files/$name'" <<EOF
$content
EOF
}

# emu_read_file <application id> <file under files/>: prints a file of the application.
emu_read_file() {
    local application_id="$1" name="$2"
    adb_do exec-out run-as "$application_id" cat "files/$name" 2>/dev/null || true
}

# emu_screencap <png file>: a picture of the screen of the device, written on the host.
emu_screencap() {
    local file="$1"
    if ADB_TIMEOUT=30 adb_do exec-out screencap -p > "$file" 2>/dev/null && [ -s "$file" ]; then
        echo "   picture: $file"
    else
        echo "   no picture could be taken ($file)" >&2
        rm -f "$file"
        return 1
    fi
}
