#!/usr/bin/env python3
"""Package the actual Cargo desktop executable as a macOS app, without launching it."""
from __future__ import annotations
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import tempfile

ROOT = Path(__file__).resolve().parents[2]
IDENTIFIER = 'org.ferrodock.reactiveui-sample'

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT / 'target/debug/ferrodock-reactiveui-sample')
    parser.add_argument('--output', type=Path, default=ROOT / 'target/FerroDock.app')
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    app = args.output.expanduser().absolute()
    info_path = app / 'Contents/Info.plist'
    if app.exists():
        if not info_path.is_file() or plistlib.loads(info_path.read_bytes()).get('CFBundleIdentifier') != IDENTIFIER:
            parser.error(f'refusing to replace an unrelated app: {app}')
    executable_dir = app / 'Contents/MacOS'
    executable_dir.mkdir(parents=True, exist_ok=True)
    # A physical, atomic copy is intentional: the launchable bundle must contain
    # its executable rather than a symlink out to a build directory.
    descriptor, temporary = tempfile.mkstemp(prefix='.ferrodock-', dir=executable_dir)
    os.close(descriptor)
    try:
        shutil.copy2(binary, temporary)
        Path(temporary).replace(executable_dir / 'ferrodock-reactiveui-sample')
    finally:
        Path(temporary).unlink(missing_ok=True)
    info_path.write_bytes(plistlib.dumps({
        'CFBundleIdentifier': IDENTIFIER,
        'CFBundleName': 'FerroDock',
        'CFBundleDisplayName': 'FerroDock',
        'CFBundleExecutable': 'ferrodock-reactiveui-sample',
        'CFBundlePackageType': 'APPL',
        'CFBundleShortVersionString': '0.1.0',
        'CFBundleVersion': '1',
        'NSHighResolutionCapable': True,
        'LSMinimumSystemVersion': '11.0',
    }))
    print(app)

if __name__ == '__main__':
    main()
