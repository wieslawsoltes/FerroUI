#!/bin/sh
# Converts the markup of the upstream Fluent theme into src/FerroUI.Themes.Fluent
# (pass --check to verify that the converted files are up to date).
# usage: scripts/sync-fluent-theme.sh <upstream checkout> [--check]
set -e
UPSTREAM="${1:?usage: sync-fluent-theme.sh <upstream checkout> [--check]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
python3 "$ROOT/scripts/convert_theme_xaml.py" \
    "$UPSTREAM/src/Avalonia.Themes.Fluent" "$ROOT/src/FerroUI.Themes.Fluent" \
    --exclude-list "$ROOT/src/FerroUI.Themes.Fluent/Controls/excluded.txt" $2
