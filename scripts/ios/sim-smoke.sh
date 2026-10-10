#!/usr/bin/env bash
# Runs the smoke mode of the iOS example in a simulator (docs/porting/ios-platform.md, section 12):
# builds `examples/ios_view.rs` of ferroui-ios for the simulator, bundles it (scripts/ios/bundle.sh),
# boots the device if it is not booted, installs and launches the application with `--smoke`,
# waits for it to exit, prints its checks and a status, uninstalls it and shuts the device down.
#
#   scripts/ios/sim-smoke.sh <device name or udid> [--keep] [--timeout <seconds>] [--no-build]
#
#   --keep       leave the device booted and the application installed
#   --timeout    how long the application may run                 (default: 180)
#   --no-build   bundle the executable that is already built
#
# The build directory is CARGO_TARGET_DIR when it is set, else `target` of the workspace. The
# console output and the report the application wrote into its data container are kept under
# <build directory>/ios-smoke, two screenshots of the simulator under
# ~/Library/Caches/ferroui-ios-smoke.
#
# The exit code is 0 when the application reported that every check passed, 1 when a check failed
# or the application did not report, 2 for a problem of the script (no such device, the build).
set -uo pipefail

device=
keep=0
timeout=180
build_args=()

while [ $# -gt 0 ]; do
  case "$1" in
    --keep) keep=1; shift ;;
    --timeout) timeout="$2"; shift 2 ;;
    --no-build) build_args+=(--no-build); shift ;;
    -h|--help) sed -n '2,17p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "sim-smoke.sh: unknown option: $1" >&2; exit 2 ;;
    *) device="$1"; shift ;;
  esac
done

if [ -z "$device" ]; then
  echo "usage: scripts/ios/sim-smoke.sh <device name or udid> [--keep] [--timeout <seconds>] [--no-build]" >&2
  exit 2
fi

root="$(cd "$(dirname "$0")/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
out="$target_dir/ios-smoke"
bundle_id=org.ferroui.ios-view
udid_pattern='[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}'

# The device: an identifier as it is, a name through the list of the available devices (the first
# one of that name: a name exists once per runtime).
if printf '%s' "$device" | grep -Eq "^$udid_pattern\$"; then
  udid="$device"
else
  udid="$(xcrun simctl list devices available | grep -F "    $device (" | head -n 1 | grep -Eo "$udid_pattern" | head -n 1)"
fi
if [ -z "${udid:-}" ]; then
  echo "sim-smoke.sh: no available simulator device named \"$device\"; the devices:" >&2
  xcrun simctl list devices available >&2
  exit 2
fi
line="$(xcrun simctl list devices | grep -F "$udid" | head -n 1)"
if [ -z "$line" ]; then
  echo "sim-smoke.sh: no simulator device with the identifier $udid" >&2
  exit 2
fi
echo "Device:$line"

mkdir -p "$out"
rm -f "$out/screenshot-early.log" "$out/console.log" "$out/ios_view_smoke.txt" "$out/screenshot.log"
# The screenshots go under the home directory: the simulator service, which writes them, may not
# write to every volume a build directory can be on.
shots="$HOME/Library/Caches/ferroui-ios-smoke"
mkdir -p "$shots"
rm -f "$shots/screenshot-early.png" "$shots/screenshot.png"

# Build and bundle first: nothing is booted for a build that fails.
app="$("$root/scripts/ios/bundle.sh" --bundle-id "$bundle_id" ${build_args[@]+"${build_args[@]}"} | tail -n 1)"
if [ -z "$app" ] || [ ! -d "$app" ]; then
  echo "sim-smoke.sh: the application was not bundled" >&2
  exit 2
fi
echo "Bundle: $app"

cleanup() {
  if [ "$keep" = 0 ]; then
    xcrun simctl uninstall "$udid" "$bundle_id" >/dev/null 2>&1
    xcrun simctl shutdown "$udid" >/dev/null 2>&1
    echo "The application was uninstalled and the device shut down."
  else
    echo "The device stays booted, with the application installed (--keep)."
  fi
}

if ! printf '%s' "$line" | grep -q '(Booted)'; then
  echo "Booting the device..."
  if ! xcrun simctl boot "$udid"; then
    echo "sim-smoke.sh: the device did not boot" >&2
    exit 2
  fi
fi
# Waits until the system of the device has started.
if ! xcrun simctl bootstatus "$udid" -b >/dev/null; then
  echo "sim-smoke.sh: the device did not finish booting" >&2
  cleanup
  exit 2
fi

if ! xcrun simctl install "$udid" "$app"; then
  echo "sim-smoke.sh: the application was not installed" >&2
  cleanup
  exit 2
fi

# The application holds its view for a few seconds after it reported (FERROUI_SMOKE_HOLD), which
# is when the screenshot is taken. `launch --console-pty` returns when the application has exited.
echo "Launching $bundle_id --smoke (at most $timeout seconds)..."
SIMCTL_CHILD_FERROUI_SMOKE_HOLD=4 SIMCTL_CHILD_RUST_BACKTRACE=1 \
  xcrun simctl launch --console-pty --terminate-running-process "$udid" "$bundle_id" --smoke \
  > "$out/console.log" 2>&1 &
launch=$!

started=$SECONDS
reported=0
timed_out=0
early=0
while kill -0 "$launch" 2>/dev/null; do
  # A first screenshot a few seconds after the launch, whatever the application reports.
  if [ "$early" = 0 ] && [ $((SECONDS - started)) -ge 6 ]; then
    early=1
    xcrun simctl io "$udid" screenshot "$shots/screenshot-early.png" > "$out/screenshot-early.log" 2>&1
  fi
  if [ "$reported" = 0 ] && grep -Eq 'SMOKE (PASSED|FAILED)' "$out/console.log" 2>/dev/null; then
    reported=1
    sleep 1
    xcrun simctl io "$udid" screenshot "$shots/screenshot.png" > "$out/screenshot.log" 2>&1
  fi
  if [ $((SECONDS - started)) -ge "$timeout" ]; then
    timed_out=1
    xcrun simctl terminate "$udid" "$bundle_id" >/dev/null 2>&1
    kill "$launch" 2>/dev/null
    break
  fi
  sleep 1
done
wait "$launch" 2>/dev/null
launch_status=$?

# The report the application wrote into its data container.
container="$(xcrun simctl get_app_container "$udid" "$bundle_id" data 2>/dev/null)"
if [ -n "$container" ] && [ -f "$container/Documents/ios_view_smoke.txt" ]; then
  cp "$container/Documents/ios_view_smoke.txt" "$out/ios_view_smoke.txt"
fi

echo
echo "--- console ($out/console.log) ---"
# The pseudo terminal ends lines with a carriage return.
tr -d '\r' < "$out/console.log"
echo "--- end of console ---"
echo
if [ -f "$out/ios_view_smoke.txt" ]; then
  echo "--- report of the application ($out/ios_view_smoke.txt) ---"
  cat "$out/ios_view_smoke.txt"
  echo "--- end of report ---"
else
  echo "The application wrote no report into its data container."
fi
if [ -f "$shots/screenshot-early.png" ]; then
  echo "Screenshot of the simulator, six seconds after the launch: $shots/screenshot-early.png"
fi
if [ -f "$shots/screenshot.png" ]; then
  echo "Screenshot of the simulator, after the report: $shots/screenshot.png"
elif [ -f "$out/screenshot.log" ]; then
  echo "No screenshot was taken:"
  cat "$out/screenshot.log"
else
  echo "No screenshot was taken: the application did not report while it ran."
fi
echo

status=1
if [ "$timed_out" = 1 ]; then
  echo "RESULT: the application did not exit within $timeout seconds"
elif grep -q 'SMOKE PASSED' "$out/console.log" "$out/ios_view_smoke.txt" 2>/dev/null; then
  echo "RESULT: SMOKE PASSED (simctl launch exited with $launch_status)"
  status=0
elif grep -q 'SMOKE FAILED' "$out/console.log" "$out/ios_view_smoke.txt" 2>/dev/null; then
  echo "RESULT: SMOKE FAILED (simctl launch exited with $launch_status)"
else
  echo "RESULT: the application exited without a report (simctl launch exited with $launch_status)"
  echo "The crash reports of the simulator, if any: ~/Library/Logs/DiagnosticReports/ios_view-*.ips"
fi

cleanup
exit "$status"
