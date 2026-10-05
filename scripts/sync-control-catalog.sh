#!/bin/sh
# Converts the markup of the upstream ControlCatalog sample into samples/ControlCatalog and
# copies its assets (pass --check to verify that the converted files and the assets are up to date).
# usage: scripts/sync-control-catalog.sh <upstream checkout> [--check]
set -e
UPSTREAM="${1:?usage: sync-control-catalog.sh <upstream checkout> [--check]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$UPSTREAM/samples/ControlCatalog"
TARGET="$ROOT/samples/ControlCatalog"
python3 "$ROOT/scripts/convert_catalog_xaml.py" "$SOURCE" "$TARGET" $2

# The assets of the upstream project file: `Assets/**`, the shared application icons linked
# into `Assets/`, and the embedded resource `Pages/teapot.bin`.
if [ "$2" = "--check" ]; then
    diff -r "$SOURCE/Assets" "$TARGET/Assets" -x icon.ico -x icon-32.png
    cmp "$UPSTREAM/build/Assets/icon.ico" "$TARGET/Assets/icon.ico"
    cmp "$UPSTREAM/build/Assets/icon-32.png" "$TARGET/Assets/icon-32.png"
    cmp "$SOURCE/Pages/teapot.bin" "$TARGET/Pages/teapot.bin"
    echo "assets checked"
else
    mkdir -p "$TARGET/Assets" "$TARGET/Pages"
    cp -R "$SOURCE/Assets/." "$TARGET/Assets/"
    cp "$UPSTREAM/build/Assets/icon.ico" "$UPSTREAM/build/Assets/icon-32.png" "$TARGET/Assets/"
    cp "$SOURCE/Pages/teapot.bin" "$TARGET/Pages/teapot.bin"
    echo "assets copied"
fi
