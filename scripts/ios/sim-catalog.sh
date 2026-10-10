#!/usr/bin/env bash
# Runs the ControlCatalog sample in a simulator and takes a screenshot of a few of its pages
# (docs/porting/ios-platform.md, section 12): builds `control-catalog-ios` for the simulator,
# bundles it (scripts/ios/bundle.sh), boots the device if it is not booted, installs and launches
# the application with its smoke run (FERROUI_SMOKE_PAGES, FERROUI_SMOKE_PAGE_NAMES), takes a
# screenshot of the simulator while each page is shown, and when the last page was shown uninstalls
# the application and shuts the device down.
#
#   scripts/ios/sim-catalog.sh <device name or udid> [--pages <a,b,c>] [--page-ms <n>] [--keep]
#                              [--timeout <seconds>] [--no-build]
#
#   --pages      the headers of the pages to show, in order     (default: Home,Buttons,TextBox,ListBox)
#   --page-ms    how long a page is shown, in milliseconds      (default: 5000)
#   --keep       leave the device booted and the application installed
#   --timeout    how long the application may run               (default: 240)
#   --no-build   bundle the executable that is already built
#
# The build directory is CARGO_TARGET_DIR when it is set, else `target` of the workspace; the
# console output is kept under <build directory>/ios-smoke/catalog-console.log. The screenshots
# go to ~/Library/Caches/ferroui-ios-smoke/catalog-<n>-<header>.png (the simulator service, which
# writes them, may not write to every volume a build directory can be on), and their paths are
# printed.
#
# The exit code is 0 when every page that was asked for was shown and has a screenshot, 1 when not,
# 2 for a problem of the script (no such device, the build).
set -uo pipefail

device=
pages="Home,Buttons,TextBox,ListBox"
page_ms=5000
keep=0
timeout=240
build_args=()

while [ $# -gt 0 ]; do
  case "$1" in
    --pages) pages="$2"; shift 2 ;;
    --page-ms) page_ms="$2"; shift 2 ;;
    --keep) keep=1; shift ;;
    --timeout) timeout="$2"; shift 2 ;;
    --no-build) build_args+=(--no-build); shift ;;
    -h|--help) sed -n '2,25p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) echo "sim-catalog.sh: unknown option: $1" >&2; exit 2 ;;
    *) device="$1"; shift ;;
  esac
done

if [ -z "$device" ]; then
  echo "usage: scripts/ios/sim-catalog.sh <device name or udid> [--pages <a,b,c>] [--page-ms <n>] [--keep] [--timeout <seconds>] [--no-build]" >&2
  exit 2
fi

root="$(cd "$(dirname "$0")/../.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$root/target}"
out="$target_dir/ios-smoke"
shots="$HOME/Library/Caches/ferroui-ios-smoke"
bundle_id=org.ferroui.control-catalog
console="$out/catalog-console.log"
udid_pattern='[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}'

if printf '%s' "$device" | grep -Eq "^$udid_pattern\$"; then
  udid="$device"
else
  udid="$(xcrun simctl list devices available | grep -F "    $device (" | head -n 1 | grep -Eo "$udid_pattern" | head -n 1)"
fi
if [ -z "${udid:-}" ]; then
  echo "sim-catalog.sh: no available simulator device named \"$device\"; the devices:" >&2
  xcrun simctl list devices available >&2
  exit 2
fi
line="$(xcrun simctl list devices | grep -F "$udid" | head -n 1)"
if [ -z "$line" ]; then
  echo "sim-catalog.sh: no simulator device with the identifier $udid" >&2
  exit 2
fi
echo "Device:$line"

mkdir -p "$out" "$shots"
rm -f "$console" "$shots"/catalog-*.png "$out"/catalog-screenshot-*.log

# Build and bundle first: nothing is booted for a build that fails.
app="$("$root/scripts/ios/bundle.sh" --package control-catalog-ios --bin control-catalog-ios \
  --bundle-id "$bundle_id" --name ControlCatalog ${build_args[@]+"${build_args[@]}"} | tail -n 1)"
if [ -z "$app" ] || [ ! -d "$app" ]; then
  echo "sim-catalog.sh: the application was not bundled" >&2
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
    echo "sim-catalog.sh: the device did not boot" >&2
    exit 2
  fi
fi
if ! xcrun simctl bootstatus "$udid" -b >/dev/null; then
  echo "sim-catalog.sh: the device did not finish booting" >&2
  cleanup
  exit 2
fi

if ! xcrun simctl install "$udid" "$app"; then
  echo "sim-catalog.sh: the application was not installed" >&2
  cleanup
  exit 2
fi

echo "Launching $bundle_id: the pages $pages, $page_ms ms each (at most $timeout seconds)..."
SIMCTL_CHILD_FERROUI_SMOKE_PAGES="$page_ms" SIMCTL_CHILD_FERROUI_SMOKE_PAGE_NAMES="$pages" \
  SIMCTL_CHILD_RUST_BACKTRACE=1 \
  xcrun simctl launch --console-pty --terminate-running-process "$udid" "$bundle_id" \
  > "$console" 2>&1 &
launch=$!

# A page is on the screen for `page_ms` after its line "Selecting <header>"; its screenshot is taken
# a second before the next page is selected, so that the page had the time to load and draw.
settle=$(( (page_ms - 1000) / 1000 ))
if [ "$settle" -lt 1 ]; then settle=1; fi

started=$SECONDS
taken=0
timed_out=0
startup=0
while kill -0 "$launch" 2>/dev/null; do
  # What the application shows when it has started, before any page is selected.
  if [ "$startup" = 0 ] && grep -q '^Pages (' "$console" 2>/dev/null; then
    startup=1
  fi
  selected="$(tr -d '\r' < "$console" 2>/dev/null | grep -c '^Selecting ')"
  if [ "$selected" -gt "$taken" ]; then
    taken=$((taken + 1))
    header="$(tr -d '\r' < "$console" | grep '^Selecting ' | sed -n "${taken}p" | sed 's/^Selecting //')"
    name="$(printf '%s' "$header" | tr -c 'A-Za-z0-9' '-' | tr 'A-Z' 'a-z')"
    sleep "$settle"
    file="$shots/catalog-$taken-$name.png"
    if xcrun simctl io "$udid" screenshot "$file" > "$out/catalog-screenshot-$taken.log" 2>&1; then
      echo "Page $taken, $header: $file"
    else
      echo "Page $taken, $header: no screenshot:"
      cat "$out/catalog-screenshot-$taken.log"
    fi
    continue
  fi
  if [ $((SECONDS - started)) -ge "$timeout" ]; then
    timed_out=1
    xcrun simctl terminate "$udid" "$bundle_id" >/dev/null 2>&1
    kill "$launch" 2>/dev/null
    break
  fi
  sleep 0.2
done
wait "$launch" 2>/dev/null
launch_status=$?

echo
echo "--- console ($console) ---"
tr -d '\r' < "$console"
echo "--- end of console ---"
echo

expected="$(printf '%s' "$pages" | tr ',' '\n' | grep -c .)"
written="$(ls "$shots"/catalog-*.png 2>/dev/null | wc -l | tr -d ' ')"
echo "Screenshots ($written of $expected pages) in $shots:"
ls -l "$shots"/catalog-*.png 2>/dev/null

status=1
if [ "$timed_out" = 1 ]; then
  echo "RESULT: the application did not exit within $timeout seconds"
elif [ "$written" -ge "$expected" ] && tr -d '\r' < "$console" | grep -q '^Selected every page'; then
  echo "RESULT: CATALOG SHOWN ($written pages; simctl launch exited with $launch_status)"
  status=0
elif [ "$startup" = 0 ]; then
  echo "RESULT: the catalog did not start (simctl launch exited with $launch_status)"
  echo "The crash reports of the simulator, if any: ~/Library/Logs/DiagnosticReports/control-catalog-ios-*.ips"
else
  echo "RESULT: $written of $expected pages have a screenshot (simctl launch exited with $launch_status)"
fi

cleanup
exit "$status"
