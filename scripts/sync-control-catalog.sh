#!/bin/sh
# Converts the markup of the upstream ControlCatalog sample into samples/ControlCatalog and
# copies its assets (pass --check to verify that the converted files and the assets are up to date).
# usage: scripts/sync-control-catalog.sh <upstream checkout> [--check] [--commit=<revision>]
#
# Portable (POSIX sh, BSD and GNU tools): runs as it is on macOS and Linux. The upstream checkout
# is required; `--check` may come before or after it. Without `--commit` the files of the
# checkout's working tree are read; with `--commit=<revision>` the files of that revision of the
# checkout's repository are read instead (extracted with `git archive` into a temporary
# directory), so that the catalog can be held to the tracked upstream commit whatever the
# checkout has checked out.
set -e
USAGE="usage: sync-control-catalog.sh <upstream checkout> [--check] [--commit=<revision>]"
UPSTREAM=""
CHECK=""
COMMIT=""
for argument in "$@"; do
    case "$argument" in
        --check) CHECK="--check" ;;
        --commit=?*) COMMIT="${argument#--commit=}" ;;
        -*) echo "unknown option: $argument" >&2; echo "$USAGE" >&2; exit 2 ;;
        *)
            if [ -n "$UPSTREAM" ]; then
                echo "more than one upstream checkout given" >&2; echo "$USAGE" >&2; exit 2
            fi
            UPSTREAM="$argument"
            ;;
    esac
done
if [ -z "$UPSTREAM" ]; then
    echo "no upstream checkout given" >&2
    echo "$USAGE" >&2
    exit 2
fi
if [ ! -d "$UPSTREAM/samples/ControlCatalog" ]; then
    echo "not an upstream checkout (no samples/ControlCatalog): $UPSTREAM" >&2
    exit 2
fi
# Absolute, so that no path below can be read as an option of the tools.
UPSTREAM="$(cd "$UPSTREAM" && pwd)"
if [ -n "$COMMIT" ]; then
    REVISION="$(git -C "$UPSTREAM" rev-parse --verify --quiet "$COMMIT^{commit}")" || {
        echo "not a commit of the upstream checkout: $COMMIT" >&2
        exit 2
    }
    EXTRACTED="$(mktemp -d "${TMPDIR:-/tmp}/sync-control-catalog.XXXXXX")"
    trap 'rm -rf "$EXTRACTED"' EXIT
    git -C "$UPSTREAM" archive --format=tar "$REVISION" samples/ControlCatalog build/Assets |
        tar -x -f - -C "$EXTRACTED"
    echo "reading the upstream files of $REVISION"
    UPSTREAM="$EXTRACTED"
fi
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$UPSTREAM/samples/ControlCatalog"
TARGET="$ROOT/samples/ControlCatalog"

# The assets of the upstream project file: `Assets/**`, the shared application icons linked
# into `Assets/`, and the embedded resource `Pages/teapot.bin`.
if [ -n "$CHECK" ]; then
    # Both halves are checked and reported before the result.
    status=0
    python3 "$ROOT/scripts/convert_catalog_xaml.py" "$SOURCE" "$TARGET" --check || status=1
    # FerroUICatalogSymbols.ttf is the port's own (scripts/winui-controls/catalog_symbol_font.py).
    diff -r -x icon.ico -x icon-32.png -x FerroUICatalogSymbols.ttf "$SOURCE/Assets" "$TARGET/Assets" || status=1
    cmp "$UPSTREAM/build/Assets/icon.ico" "$TARGET/Assets/icon.ico" || status=1
    cmp "$UPSTREAM/build/Assets/icon-32.png" "$TARGET/Assets/icon-32.png" || status=1
    cmp "$SOURCE/Pages/teapot.bin" "$TARGET/Pages/teapot.bin" || status=1
    if [ "$status" -eq 0 ]; then
        echo "markup and assets checked"
    else
        echo "the catalog differs from the upstream checkout (above)" >&2
    fi
    exit "$status"
else
    python3 "$ROOT/scripts/convert_catalog_xaml.py" "$SOURCE" "$TARGET"
    mkdir -p "$TARGET/Assets" "$TARGET/Pages"
    cp -R "$SOURCE/Assets/." "$TARGET/Assets/"
    cp "$UPSTREAM/build/Assets/icon.ico" "$UPSTREAM/build/Assets/icon-32.png" "$TARGET/Assets/"
    cp "$SOURCE/Pages/teapot.bin" "$TARGET/Pages/teapot.bin"
    echo "assets copied"
fi
