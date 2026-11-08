#!/usr/bin/env python3
"""Capture catalog pages in separate processes to avoid asynchronous snapshot/navigation races."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import struct
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PAGES = 'Home,TextBlock,TextBox,Image,Visuals,Composition brushes,Shadows and lights,Effect brushes,Acrylic brush,Blend and colour effects,Colour effects,Image effects,Shader effects,Interactive effects,Page transitions'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--pages', default=DEFAULT_PAGES)
    parser.add_argument('--interval-ms', type=int, default=2000)
    parser.add_argument('--window-position', help='Desktop X,Y for a fixed capture display (requires the updated catalog)')
    parser.add_argument('--expected-pixel-size', help='Fail if a PNG is not WIDTHxHEIGHT; catches mixed-DPI captures')
    parser.add_argument('--macos-bundle', action='store_true',
                        help='Wrap the unchanged executable in a high-resolution-capable macOS app bundle')
    args = parser.parse_args()
    expected_size = None
    if args.expected_pixel_size:
        try:
            expected_size = tuple(int(value) for value in args.expected_pixel_size.split('x'))
            if len(expected_size) != 2 or min(expected_size) <= 0:
                raise ValueError()
        except ValueError:
            parser.error('--expected-pixel-size must be positive WIDTHxHEIGHT')
    binary = args.binary.resolve()
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    source_binary = binary
    binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    if args.macos_bundle:
        contents = directory / 'Capture.app' / 'Contents'
        (contents / 'MacOS').mkdir(parents=True, exist_ok=True)
        binary = contents / 'MacOS' / 'control-catalog-desktop'
        shutil.copy2(source_binary, binary)
        info = plistlib.loads((ROOT / 'samples/ControlCatalog.Desktop/Info.plist').read_bytes())
        info.update(CFBundleExecutable=binary.name, CFBundlePackageType='APPL', NSHighResolutionCapable=True)
        (contents / 'Info.plist').write_bytes(plistlib.dumps(info))
    results = []
    for page in (p.strip() for p in args.pages.split(',') if p.strip()):
        slug = re.sub(r'[^a-z0-9]', '-', page.lower())
        output = directory / slug
        output.mkdir(exist_ok=True)
        env = os.environ.copy()
        env.update(FERROUI_SMOKE_SCREENSHOTS=str(output), FERROUI_SMOKE_SCREENSHOT_PAGES=page,
                   FERROUI_SMOKE_PAGES=str(args.interval_ms), FERROUI_SMOKE_WINDOW_SIZE='1100x800',
                   FERROUI_SMOKE_EXIT_MS='30000', FERROUI_METAL_STATS='1')
        if args.window_position:
            env['FERROUI_SMOKE_WINDOW_POSITION'] = args.window_position
        started = time.monotonic()
        log = output / 'run.log'
        with log.open('w') as stream:
            process = subprocess.run([str(binary)], cwd=ROOT, env=env, stdout=stream,
                                     stderr=subprocess.STDOUT, timeout=45)
        text = log.read_text()
        images = [output / f'01-{slug}{suffix}.png' for suffix in ('', '-composed')]
        if process.returncode or any(not image.is_file() for image in images):
            raise RuntimeError(f'{page}: process={process.returncode}; inspect {log}')
        if expected_size:
            for image in images:
                size = struct.unpack('>II', image.read_bytes()[16:24])
                if size != expected_size:
                    raise RuntimeError(f'{page}: {image.name} has size {size}, expected {expected_size}; inspect {log}')
        if 'Quartz Metal error:' in text or 'METAL_ABORT' in text:
            raise RuntimeError(f'{page}: GPU error; inspect {log}')
        results.append({'page': page, 'seconds_including_startup_and_smoke_wait': round(time.monotonic()-started, 3),
                        'images': [str(image) for image in images], 'log': str(log)})
        print(f'{page}: captured both render paths', flush=True)
        (directory / 'captures.json').write_text(json.dumps({
            'binary': str(binary), 'source_binary': str(source_binary),
            'binary_sha256': binary_sha256, 'macos_bundle': args.macos_bundle,
            'window_position': args.window_position, 'expected_pixel_size': expected_size,
            'captures': results}, indent=2)+'\n')


if __name__ == '__main__':
    main()
