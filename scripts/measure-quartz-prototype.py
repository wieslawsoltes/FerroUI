#!/usr/bin/env python3
"""Build the same HelloWorld source with each macOS backend and measure shipped bytes.

Requires Xcode command-line tools and Ninja for this checkout's patched Skia.
Run from any directory: python3 scripts/measure-quartz-prototype.py
Only the backend feature changes. Themes, pages, assets, optimizations and panic
strategy are identical. The Quartz prototype is not a rendering parity claim.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def output(args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--package', choices=('hello-world', 'control-catalog-desktop'), default='hello-world')
    parser.add_argument('--native-backend', choices=('quartz', 'quartz-metal', 'quartz-runtime-shaders'), default='quartz-metal')
    parser.add_argument('--output', type=Path, default=ROOT / 'target' / 'quartz-prototype')
    parser.add_argument('--skip-build', action='store_true', help='Measure already copied package-backend executables')
    parser.add_argument('--stock-baseline', type=Path, help='Also record the untouched upstream executable')
    args = parser.parse_args()
    if args.package == 'control-catalog-desktop' and args.native_backend == 'quartz-runtime-shaders':
        parser.error('ControlCatalog already enables runtime shaders with quartz-metal')
    if platform.system() != 'Darwin':
        parser.error('This prototype comparison runs on macOS')
    args.output.mkdir(parents=True, exist_ok=True)
    target = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target'))
    if not target.is_absolute():
        target = ROOT / target
    result = {'commit': output(['git', 'rev-parse', 'HEAD']),
              'working_tree_modified': bool(output(['git', 'status', '--porcelain'])),
              'os': output(['sw_vers', '-productVersion']), 'architecture': platform.machine(),
              'rustc': output(['rustc', '-V']), 'profile': 'dist', 'package': args.package, 'native_backend': args.native_backend,
              'commands': {}, 'measurements': {},
              'build_environment': {key: os.environ.get(key) for key in
                  ('MACOSX_DEPLOYMENT_TARGET', 'RUSTFLAGS', 'SKIA_LIBRARY_SEARCH_PATH', 'SKIA_NINJA_COMMAND')}}
    tracked = output(['git', 'ls-files', '-co', '--exclude-standard', '--',
                      'Cargo.toml', 'Cargo.lock', 'src/Quartz', 'src/FerroUI.Desktop',
                      'samples/HelloWorld', 'src/FerroUI.Native/metal.rs',
                      'src/FerroUI.Native/window_impl.rs', 'native/FerroUI.Native/src/OSX',
                      'src/FerroUI.Base/rendering/platform_render_interface_context_manager.rs']).splitlines()
    result['source_sha256'] = {path: hashlib.sha256((ROOT / path).read_bytes()).hexdigest()
                               for path in sorted(set(tracked)) if (ROOT / path).is_file()}
    for backend in ('skia', args.native_backend):
        binary = args.output / f'{args.package}-{backend}'
        command = ['cargo', 'build', '--locked', '--profile', 'dist', '-p',
                   args.package, '--no-default-features', '--features', backend]
        result['commands'][backend] = command
        if not args.skip_build:
            subprocess.run(command, cwd=ROOT, check=True)
            shutil.copy2(target / 'dist' / args.package, binary)
        data = binary.read_bytes()
        result['measurements'][backend] = {'path': str(binary.resolve()), 'bytes': len(data),
            'gzip_bytes': len(gzip.compress(data, mtime=0)), 'sha256': hashlib.sha256(data).hexdigest(),
            'dynamic_libraries': output(['otool', '-L', str(binary)]).splitlines()[1:]}
        print(f'{backend}: {len(data):,} bytes ({len(data)/1_000_000:.2f} MB)')
    if args.stock_baseline:
        data = args.stock_baseline.read_bytes()
        result['stock_baseline'] = {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}
    before = result['measurements']['skia']['bytes']
    after = result['measurements'][args.native_backend]['bytes']
    result['saved_bytes'] = before - after
    result['saved_percent'] = round(100 * (before - after) / before, 2)
    tree = output(['cargo', 'tree', '--locked', '-p', args.package,
                   '--no-default-features', '--features', args.native_backend, '-e', 'normal'])
    forbidden = ('ferroui-skia ', 'skia-safe ', 'skia-bindings ', 'harfbuzz-sys ',
                 'ferroui-harfbuzz ', 'wgpu ', 'ferroui-vello ', 'mozangle ')
    included = [name for name in forbidden if name in tree]
    if included:
        raise RuntimeError(f'Native-only graph includes unexpected dependencies: {included}')
    result['native_dependency_exclusion_verified'] = True
    result['runtime_shader_compiler_included'] = 'naga v' in tree
    if args.package == 'hello-world' and args.native_backend != 'quartz-runtime-shaders' and result['runtime_shader_compiler_included']:
        raise RuntimeError('The size-first backend unexpectedly includes the optional shader compiler')
    (args.output / 'native-dependencies.txt').write_text(tree + '\n')
    report = args.output / 'sizes.json'
    report.write_text(json.dumps(result, indent=2) + '\n')
    print(f'Saved {before-after:,} bytes ({result["saved_percent"]}%). Report: {report}')


if __name__ == '__main__':
    main()
