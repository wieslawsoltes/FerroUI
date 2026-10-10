#!/bin/sh
# shellcheck disable=SC3043  # `local`: dash and bash have it
# Prints the accessibility tree of the applications on the accessibility bus of the session, read
# with gdbus (the package libglib2.0-bin) as any client of AT-SPI reads it: the address of the bus
# from org.a11y.Bus on the session bus, the applications from the root of the registry, then
# GetChildren, GetRoleName and the property Name of every object, depth first.
#
# usage: atspi-tree.sh [toolkit] [maximum number of objects]
#   toolkit: only applications whose ToolkitName is this (default: FerroUI; "" for all)
# Run inside the session (DBUS_SESSION_BUS_ADDRESS). docs/porting/atspi.md, "Verification".
toolkit="${1-FerroUI}"
budget="${2:-400}"
count_file="$(mktemp)"
echo 0 > "$count_file"

address="$(gdbus call --session --dest org.a11y.Bus --object-path /org/a11y/bus --method org.a11y.Bus.GetAddress \
  | sed "s/^('//; s/',)\$//")"
if [ -z "$address" ]; then
  echo "no accessibility bus: org.a11y.Bus did not answer"
  exit 1
fi
echo "accessibility bus: $address"

# Standard input is kept away from gdbus: the calls are made inside loops that read their lines from it.
call() {
  local call_dest="$1" call_path="$2" call_method="$3"
  shift 3
  gdbus call --address "$address" --dest "$call_dest" --object-path "$call_path" --method "$call_method" "$@" 2>&1 < /dev/null
}

# The value of a reply with one string: ('text',) or (<'text'>,)
text() {
  sed "s/^(<\\{0,1\\}['\"]//; s/['\"]>\\{0,1\\},)\$//"
}

# The references of a reply, one per line: "name path". gdbus writes the type of the path
# ("objectpath") for the first element of an array only.
references() {
  grep -oE "\('[^']*', (objectpath )?'[^']*'\)" | sed -E "s/^\('//; s/', (objectpath )?'/ /; s/'\)\$//"
}

walk() {
  local dest="$1" path="$2" indent="$3" count role name child_dest child_path
  count=$(($(cat "$count_file") + 1))
  echo "$count" > "$count_file"
  if [ "$count" -gt "$budget" ]; then
    return
  fi
  role="$(call "$dest" "$path" org.a11y.atspi.Accessible.GetRoleName | text)"
  name="$(call "$dest" "$path" org.freedesktop.DBus.Properties.Get org.a11y.atspi.Accessible Name | text)"
  echo "$indent$role \"$name\" $path"
  call "$dest" "$path" org.a11y.atspi.Accessible.GetChildren | references | while read -r child_dest child_path; do
    walk "$child_dest" "$child_path" "$indent  "
  done
}

found=0
applications="$(call org.a11y.atspi.Registry /org/a11y/atspi/accessible/root org.a11y.atspi.Accessible.GetChildren | references)"
echo "applications the registry lists: $(echo "$applications" | grep -c .)"
echo "$applications" | while read -r app_dest app_path; do
  [ -n "$app_dest" ] || continue
  app_toolkit="$(call "$app_dest" "$app_path" org.freedesktop.DBus.Properties.Get org.a11y.atspi.Application ToolkitName | text)"
  if [ -n "$toolkit" ] && [ "$app_toolkit" != "$toolkit" ]; then
    continue
  fi
  echo "application $app_dest (toolkit $app_toolkit):"
  walk "$app_dest" "$app_path" "  "
  echo found >> "$count_file.found"
done
objects="$(cat "$count_file")"
if [ -f "$count_file.found" ]; then
  found=1
fi
rm -f "$count_file" "$count_file.found"
echo "objects: $objects"
if [ "$found" = 0 ]; then
  echo "no application of the toolkit \"$toolkit\" on the accessibility bus"
  exit 1
fi
