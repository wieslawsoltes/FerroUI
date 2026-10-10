#!/bin/sh
# Converts the markup of the upstream Simple theme into src/FerroUI.Themes.Simple
# (pass --check to verify that the converted files are up to date).
# usage: scripts/sync-simple-theme.sh <upstream checkout> [--check]
set -e
UPSTREAM="${1:?usage: sync-simple-theme.sh <upstream checkout> [--check]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
python3 "$ROOT/scripts/convert_theme_xaml.py" \
    "$UPSTREAM/src/Avalonia.Themes.Simple" "$ROOT/src/FerroUI.Themes.Simple" \
    --link "$UPSTREAM/src/Avalonia.Themes.Fluent/Strings/InvariantResources.xaml=Strings/InvariantResources.xaml" \
    --exclude-list "$ROOT/src/FerroUI.Themes.Simple/Controls/excluded.txt" $2
