#!/usr/bin/env bash
# Re-imports the native macOS backend sources and the interop IDL from an
# upstream Avalonia checkout and applies the FerroUI renames.
#
#   scripts/sync-native.sh /path/to/Avalonia      # sync sources + IDL
#   scripts/sync-native.sh --filter < in > out    # only apply the renames to a stream
#
# Copied:
#   <upstream>/native/Avalonia.Native/inc/*.h      -> native/FerroUI.Native/inc/
#       (except the generated avalonia-native.h: the header is produced at
#        build time by microcom-codegen from the IDL)
#   <upstream>/native/Avalonia.Native/src/OSX/*.{h,mm} -> native/FerroUI.Native/src/OSX/
#   <upstream>/src/Avalonia.Native/avn.idl         -> src/FerroUI.Native/frn.idl
#
# Renames (case-sensitive, applied in this order to file contents and file names):
#   Avalonia.Native -> FerroUI.Native
#   Avalonia -> Ferro      avalonia -> ferro      AVALONIA -> FERRO
#   Avn      -> Frn        avn      -> frn        AVN      -> FRN
# Special cases handled before the general rules:
#   AvNS<Class> (ObjC category names, e.g. on NSScreen) -> FrnNS<Class>
#   links to the upstream issue trackers -> "upstream issues <n>" /
#   "upstream tracker issue <id>" (a renamed URL would point nowhere)
# In addition the per-file upstream copyright/licence comment lines are removed
# from the sources; they are reproduced in native/FerroUI.Native/NOTICE.md.
#
# Local modifications, if any, live in native/FerroUI.Native/patches/*.patch and
# are applied (in name order) after the rename.
#
# This script and the NOTICE files are the only places where the upstream name
# may appear.
set -euo pipefail

rename_filter() {
    sed -E \
        -e '/Copyright .*Avalonia/d' \
        -e '/Licensed under the MIT license\. See licence\.md/d' \
        -e 's#https?://github\.com/AvaloniaUI/Avalonia/(issues|pull)/([0-9]+)#upstream \1 \2#g' \
        -e 's#https?://yt\.avaloniaui\.net/issue/([A-Za-z0-9-]+)#upstream tracker issue \1#g' \
        -e 's/AvNS/FrnNS/g' \
        -e 's/Avalonia\.Native/FerroUI.Native/g' \
        -e 's/Avalonia/Ferro/g' -e 's/avalonia/ferro/g' -e 's/AVALONIA/FERRO/g' \
        -e 's/Avn/Frn/g' -e 's/avn/frn/g' -e 's/AVN/FRN/g'
}

rename_name() {
    printf '%s' "$1" | sed -E \
        -e 's/Avalonia\.Native/FerroUI.Native/g' \
        -e 's/Avalonia/Ferro/g' -e 's/avalonia/ferro/g' -e 's/AVALONIA/FERRO/g' \
        -e 's/Avn/Frn/g' -e 's/avn/frn/g' -e 's/AVN/FRN/g'
}

if [[ "${1:-}" == "--filter" ]]; then
    rename_filter
    exit 0
fi

if [[ $# -ne 1 || ! -d "$1/native/Avalonia.Native/src/OSX" ]]; then
    echo "usage: $0 <path to upstream Avalonia checkout> | --filter" >&2
    exit 2
fi

UPSTREAM=$(cd "$1" && pwd)
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
SRC_NATIVE="$UPSTREAM/native/Avalonia.Native"
DST_NATIVE="$ROOT/native/FerroUI.Native"
DST_IDL="$ROOT/src/FerroUI.Native/frn.idl"

copy_renamed() { # <source file> <destination dir>
    local dst="$2/$(rename_name "$(basename "$1")")"
    LC_ALL=C rename_filter < "$1" > "$dst"
}

rm -rf "$DST_NATIVE/inc" "$DST_NATIVE/src/OSX"
mkdir -p "$DST_NATIVE/inc" "$DST_NATIVE/src/OSX"

for f in "$SRC_NATIVE"/inc/*.h; do
    [[ "$(basename "$f")" == "avalonia-native.h" ]] && continue
    copy_renamed "$f" "$DST_NATIVE/inc"
done
for f in "$SRC_NATIVE"/src/OSX/*.h "$SRC_NATIVE"/src/OSX/*.mm; do
    copy_renamed "$f" "$DST_NATIVE/src/OSX"
done
LC_ALL=C rename_filter < "$UPSTREAM/src/Avalonia.Native/avn.idl" > "$DST_IDL"

if compgen -G "$DST_NATIVE/patches/*.patch" > /dev/null; then
    for p in "$DST_NATIVE"/patches/*.patch; do
        echo "applying $(basename "$p")"
        patch -s -p1 -d "$ROOT" < "$p"
    done
fi

REV=$(git -C "$UPSTREAM" rev-parse HEAD 2>/dev/null || echo unknown)
echo "$REV" > "$DST_NATIVE/UPSTREAM_REVISION"

# Nothing but the NOTICE may still carry the upstream name.
if grep -rniE 'avalon|avn' "$DST_NATIVE/inc" "$DST_NATIVE/src" "$DST_IDL"; then
    echo "error: upstream names left after rename (see above)" >&2
    exit 1
fi
if find "$DST_NATIVE/inc" "$DST_NATIVE/src" | grep -iE 'avalon|avn'; then
    echo "error: upstream names left in file names (see above)" >&2
    exit 1
fi
echo "synced $(find "$DST_NATIVE/inc" "$DST_NATIVE/src" -type f | wc -l | tr -d ' ') files from $UPSTREAM @ $REV"
