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
    git -C "$UPSTREAM" archive --format=tar "$REVISION" samples/ControlCatalog |
        tar -x -f - -C "$EXTRACTED"
    echo "reading the upstream files of $REVISION"
    UPSTREAM="$EXTRACTED"
fi
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE="$UPSTREAM/samples/ControlCatalog"
TARGET="$ROOT/samples/ControlCatalog"

# The assets of the upstream project file: `Assets/**` and the embedded resource
# `Pages/teapot.bin`.
#
# The upstream assets the catalog leaves out, as names of a file or a directory below `Assets/`:
# the photographs of the sample applications of the NavigationPage, CarouselPage and DrawerPage
# galleries, which the catalog does not have (LEFT_OUT_DOCUMENTS of scripts/convert_catalog_xaml.py),
# the whole CJK font, of which the catalog ships a subset (Assets/FallbackFonts,
# scripts/fonts/catalog_fallback_fonts.py), and the brand assets of the upstream project, whose
# files in the catalog are the artwork of this project (scripts/browser/render-placeholder-assets.mjs;
# the application icons of the upstream project, `build/Assets/icon.ico` and `icon-32.png`, are not
# read either).
LEFT_OUT="Pulse Movies Sanctuary RetroGaming CurvedHeader ModernApp Restaurant WenQuanYiMicroHei-01.ttf
icon.ico icon-32.png banner-bg.png banner-logo.png logo1.png logo2.png logo3.png logo4.png"
left_out() {
    for name in $LEFT_OUT; do
        case "/$1/" in
            */"$name"/*) return 0 ;;
        esac
    done
    return 1
}
EXCLUDED=""
for name in $LEFT_OUT; do
    EXCLUDED="$EXCLUDED -x $name"
done
if [ -n "$CHECK" ]; then
    # Both halves are checked and reported before the result.
    status=0
    python3 "$ROOT/scripts/convert_catalog_xaml.py" "$SOURCE" "$TARGET" --check || status=1
    # FerroUICatalogSymbols.ttf is the port's own (scripts/winui-controls/catalog_symbol_font.py),
    # and so is FallbackFonts (scripts/fonts/catalog_fallback_fonts.py).
    # shellcheck disable=SC2086 # EXCLUDED is a list of options.
    diff -r -x FerroUICatalogSymbols.ttf -x FallbackFonts $EXCLUDED "$SOURCE/Assets" "$TARGET/Assets" || status=1
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
    (cd "$SOURCE/Assets" && find . -type f) | while IFS= read -r file; do
        file="${file#./}"
        if ! left_out "$file"; then
            mkdir -p "$TARGET/Assets/$(dirname "$file")"
            cp "$SOURCE/Assets/$file" "$TARGET/Assets/$file"
        fi
    done
    cp "$SOURCE/Pages/teapot.bin" "$TARGET/Pages/teapot.bin"
    echo "assets copied"
fi
