#!/usr/bin/env python3
"""Keep the passive inspector XAML identical to its original, except namespace ownership."""
from pathlib import Path
import sys
import re


def migrate_main_view(original: str) -> str:
    migrated = original.lstrip('\ufeff').replace('https://github.com/avaloniaui', 'https://github.com/ferroui')
    migrated = migrated.replace('Dock.Avalonia.Diagnostics', 'FerroDock.Ferro.Diagnostics').replace('Dock.Avalonia.Converters', 'FerroDock.Ferro.Converters').replace('assembly=Dock.Avalonia', 'assembly=FerroDock.Ferro')
    for namespace in ['Core', 'Controls']:
        migrated = migrated.replace(f'clr-namespace:Dock.Model.{namespace};assembly=Dock.Model', f'clr-namespace:FerroDock.Model.{namespace};assembly=FerroDock.Model')
    migrated = migrated.replace('xmlns:diagnostics=', 'xmlns:behaviors="using:FerroDock.Ferro.Diagnostics.Behaviors"\n             behaviors:RootDockDebugBehavior.IsEnabled="True"\n             xmlns:diagnostics=')
    migrated = re.sub(r'<(TreeDataTemplate|DataTemplate) DataType="([^"]+)"', r'<\1 DataType="\2" x:DataType="\2"', migrated)
    migrated = migrated.replace('<DockPanel DataContext="{Binding #Root}">', '<DockPanel DataContext="{Binding #Root}" x:DataType="diagnostics:RootDockDebug">')
    return migrated.replace('Click="OnClearEvents"', 'Command="{Binding ClearEventsCommand}"')

def check_views(source: Path, target: Path) -> list[str]:
    errors = []
    expected = {p.stem for p in source.glob('*DebugView.axaml')}
    if not expected:
        errors.append("original inspector inputs are absent")
    actual = {p.stem for p in target.glob('*DebugView.xaml')}
    for name in sorted(expected - actual):
        errors.append(f'missing original inspector: {name}')
    for name in sorted(actual - expected):
        errors.append(f'extra inspector: {name}')
    for name in sorted(expected & actual):
        original = (source / (name + '.axaml')).read_text()
        migrated = original.replace('https://github.com/avaloniaui', 'https://github.com/ferroui')
        migrated = migrated.replace('Dock.Avalonia.Diagnostics', 'FerroDock.Ferro.Diagnostics')
        for namespace in ['Core', 'Controls']:
            migrated = migrated.replace(f'clr-namespace:Dock.Model.{namespace};assembly=Dock.Model', f'clr-namespace:FerroDock.Model.{namespace};assembly=FerroDock.Model')
        if (target / (name + '.xaml')).read_text() != migrated:
            errors.append(f'original inspector markup differs: {name}')
    if (source / 'RootDockDebug.axaml').is_file():
        main = target / 'RootDockDebug.xaml'
        if not main.is_file() or main.read_text() != migrate_main_view((source / 'RootDockDebug.axaml').read_text()):
            errors.append('original main inspector markup differs beyond namespace, compiled-scope and passive-command migration')
    return errors


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    source = root.parent / 'Dock/src/Dock.Avalonia.Diagnostics/Controls'
    errors = check_views(source, root / 'src/Dock/FerroDock.Ferro.Diagnostics/controls')
    if len(list(source.glob('*DebugView.axaml'))) != 18:
        errors.append('the frozen source must contain all 18 original inspector views')
    for error in errors:
        print(f'ERROR: {error}', file=sys.stderr)
    if not errors:
        print('All 18 property views and the complete main inspector match their declared source migrations.')
    return int(bool(errors))


if __name__ == '__main__':
    raise SystemExit(main())
