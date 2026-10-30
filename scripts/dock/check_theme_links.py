#!/usr/bin/env python3
"""Verify the Simple project's original shared Controls/Assets link semantics."""
from __future__ import annotations
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
FLUENT_URI = 'ferres://FerroDock.Ferro.Themes.Fluent/'
SIMPLE_URI = 'ferres://FerroDock.Ferro.Themes.Simple/'


def check_links(fluent: Path, simple: Path) -> list[str]:
    """Reject missing, extra or altered linked files, allowing only their URI owner."""
    errors: list[str] = []
    for directory in ['Controls', 'Assets']:
        original = {p.relative_to(fluent) for p in (fluent / directory).rglob('*') if p.is_file()}
        linked = {p.relative_to(simple) for p in (simple / directory).rglob('*') if p.is_file()}
        for path in sorted(original - linked):
            errors.append(f'missing linked file: {path}')
        for path in sorted(linked - original):
            errors.append(f'extra linked file: {path}')
        for path in sorted(original & linked):
            expected = (fluent / path).read_bytes()
            if path.suffix == '.xaml':
                expected = expected.replace(FLUENT_URI.encode(), SIMPLE_URI.encode())
            if (simple / path).read_bytes() != expected:
                errors.append(f'changed linked file: {path}')
    return errors


def main() -> int:
    errors = check_links(ROOT / 'src/Dock/FerroDock.Ferro.Themes.Fluent', ROOT / 'src/Dock/FerroDock.Ferro.Themes.Simple')
    for error in errors:
        print(f'ERROR: {error}', file=sys.stderr)
    if errors:
        return 1
    print('Simple theme Controls/Assets preserve the original shared inputs and assembly-local URIs.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
