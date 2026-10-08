#!/bin/sh
# Converts the theme documents of the upstream colour picker library into
# src/FerroUI.Controls.ColorPicker (pass --check to verify that the converted files are up to date).
# usage: scripts/sync-color-picker-themes.sh <upstream checkout> [--check]
set -e
UPSTREAM="${1:?usage: sync-color-picker-themes.sh <upstream checkout> [--check]}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
python3 "$ROOT/scripts/convert_theme_xaml.py" \
    "$UPSTREAM/src/Avalonia.Controls.ColorPicker" "$ROOT/src/FerroUI.Controls.ColorPicker" $2
