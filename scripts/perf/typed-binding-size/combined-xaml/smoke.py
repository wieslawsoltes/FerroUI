"""Capture the four frozen Catalog executables and verify matching framework pixels on macOS."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import tempfile


def frame(path, temporary):
    subprocess.run(['sips', '-s', 'format', 'bmp', str(path), '--out', str(temporary)],
                   check=True, stdout=subprocess.DEVNULL)
    raw = temporary.read_bytes()
    width, signed_height, planes, bits = struct.unpack_from('<iiHH', raw, 18)
    offset, = struct.unpack_from('<I', raw, 10)
    pixels = raw[offset:]
    assert planes == 1 and bits == 32 and len(pixels) == width * abs(signed_height) * 4
    return (width, signed_height), pixels


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binaries', type=Path)
    parser.add_argument('captures', type=Path, help='new directory for the captures and logs')
    parser.add_argument('arms', nargs='+', help='first arm provides the reference pixels')
    args = parser.parse_args()
    captures = args.captures.resolve()
    captures.mkdir(exist_ok=False, parents=True)
    pages = ['Buttons', 'ListBox', 'Data Validation', 'TreeDataGrid']
    settings = dict(FERROUI_CATALOG_THEME_VARIANT='light', FERROUI_SMOKE_WINDOW_SIZE='950x680',
                    FERROUI_SMOKE_EXTEND_CLIENT_AREA='0', FERROUI_SMOKE_WINDOW_POSITION='20,40',
                    FERROUI_SMOKE_SCREENSHOT_PAGES=','.join(pages), FERROUI_SMOKE_PAGES='1800')
    report = dict(reference=args.arms[0], settings=settings, captures={})
    references = {}
    for arm in args.arms:
        folder = captures / arm
        folder.mkdir()
        env = dict(os.environ, FERROUI_SMOKE_SCREENSHOTS=str(folder), **settings)
        with (folder / 'run.log').open('w') as log:
            subprocess.run([str((args.binaries / arm).resolve())], check=True, env=env, timeout=90,
                           stdout=log, stderr=subprocess.STDOUT)
        capture = report['captures'][arm] = {}
        framework_frames = sorted(path for path in folder.glob('*.png')
                                  if not path.stem.endswith(('-composed', '-window')))
        assert len(framework_frames) == len(pages), framework_frames
        with tempfile.TemporaryDirectory() as temp:
            for path in framework_frames:
                size, pixels = frame(path, Path(temp) / 'frame.bmp')
                ref_size, ref_pixels = references.setdefault(path.name, (size, pixels))
                mismatch = None
                if size == ref_size:
                    mismatch = 0 if pixels == ref_pixels else sum(a != b for a, b in zip(
                        memoryview(pixels).cast('I'), memoryview(ref_pixels).cast('I')))
                capture[path.name] = dict(size=[size[0], abs(size[1])], pixels_sha256=hashlib.sha256(pixels).hexdigest(),
                                          differing_pixels=mismatch)
        print(f'{arm}: {capture}', flush=True)
        (captures / 'samples.json').write_text(json.dumps(report, indent=2) + '\n')
    if any(item['differing_pixels'] != 0 for capture in report['captures'].values() for item in capture.values()):
        raise SystemExit('At least one frame differs from reference')


if __name__ == '__main__':
    main()
