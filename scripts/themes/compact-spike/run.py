"""Build comparable workloads in an isolated Cargo target directory and replay the timings."""
import argparse
import json
import os
import random
import shutil
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

from encode import HERE, ROOT, encode

PACKAGES = ['ferroui-resource-benchmarks', 'hello-world', 'control-catalog-desktop']
VARIANTS = ['stock', 'fixed', 'packed']


def build(tag, package, output, env, driver=None):
    label = driver or package
    command = ['cargo', 'rustc' if driver else 'build', '--locked', '--profile', 'dist', '-p', package, '--bin', package]
    if driver:
        env = dict(env, RUSTC_WRAPPER=str(HERE / 'bench_wrapper.py'))
        command += ['--', '--cfg', f'compact_{driver}_bench']
    with (output / f'{tag}-{label}.log').open('w') as log:
        subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    artifact = Path(env['CARGO_TARGET_DIR']) / 'dist' / package
    destination = output / f'{tag}-{label}'
    shutil.copy2(artifact, destination)
    if sys.platform == 'darwin':
        # Normalize size across ordinary Cargo and custom rustc wrapper invocations.
        subprocess.run(['strip', destination], check=True)
        subprocess.run(['codesign', '--verify', destination], check=True)


def binding_driver(tag, output, target):
    # Link the notifying-model probe against the exact base library used by the gallery.
    dist = target / 'dist'
    fingerprints = dist / '.fingerprint'
    app, = fingerprints.glob('control-catalog-desktop-*/bin-control-catalog-desktop.json')
    base_hash, = [entry[3] for entry in json.loads(app.read_text())['deps'] if entry[1] == 'ferroui_base']
    fingerprint, = [p for p in fingerprints.glob('ferroui-base-*/lib-ferroui_base')
                    if int.from_bytes(bytes.fromhex(p.read_text()), 'little') == base_hash]
    suffix = fingerprint.parent.name.split('-')[-1]
    lib = dist / 'deps' / f'libferroui_base-{suffix}.rlib'
    subprocess.run(['rustc', HERE / 'binding_bench.rs', '--edition=2021', '-C', 'panic=abort',
                    '-C', 'opt-level=3', '-C', 'lto=fat', '-C', 'codegen-units=1',
                    '-L', f'dependency={dist / "deps"}', '--extern', f'ferroui_base={lib}',
                    '-o', output / f'{tag}-binding'], check=True)


def metadata(output, target, env):
    base = target / 'shadow/FerroUI.Base'
    shutil.copytree(ROOT / 'src/FerroUI.Base', base)
    macros = base / 'metadata/markup_macros.rs'
    source = macros.read_text()
    hook = '(typed; $($rest:tt)*) => {\n        $crate::__ferro_markup_typed_path!($($rest)*)'
    assert source.count(hook) == 1
    macros.write_text(source.replace(hook, '(typed; $($rest:tt)*) => {\n        ::std::option::Option::None'))
    env = dict(env, CARGO_TARGET_DIR=str(target), SPIKE_BASE_COPY=str(base),
               RUSTC_WRAPPER=str(HERE / 'metadata_wrapper.py'))
    for package in PACKAGES[1:]:
        build('metadata', package, output, env)
    binding_driver('metadata', output, target)


def variants(output, target, env, modes=VARIANTS):
    reference = (ROOT / 'src/FerroUI.Themes.WinUI/compiled_xaml.rs').read_text()
    for tag in modes:
        if tag != 'stock':
            generated = list(target.glob('dist/build/ferroui-themes-winui-*/out/xaml/compiled_xaml.rs'))
            assert generated, 'Build stock first to generate every feature configuration'
            source, stats = encode(reference, tag)
            for file in generated:
                previous = file.read_text()
                assert previous == reference or '\nmod compact_spike {' in previous, file
                file.write_text(source)
            print(tag, stats, flush=True)
        for package in PACKAGES:
            build(tag, package, output, env)
            if package == PACKAGES[0]:
                build(tag, package, output, env, driver='template')
        if tag == 'stock':
            binding_driver(tag, output, target)


def measure(output, runs):
    names = ['theme_registration', 'first_button_theme_and_template_lookup',
             'first_button_template_build', 'repeated_button_template_build']
    results = {tag: [] for tag in VARIANTS}
    rng = random.Random(9890)
    for i in range(runs + 1):
        order = VARIANTS.copy()
        rng.shuffle(order)
        for tag in order:
            result = subprocess.run([output / f'{tag}-template'], capture_output=True, check=True, text=True)
            times = list(map(int, result.stdout.strip().splitlines()[-1].split()))
            assert len(times) == len(names)
            if i:
                results[tag].append(dict(zip(names, times)))
    summary = {tag: {name: statistics.median(row[name] for row in rows) for name in names}
               for tag, rows in results.items()}
    sizes = {tag: {package: (output / f'{tag}-{package}').stat().st_size for package in PACKAGES}
             for tag in VARIANTS}
    if (output / 'metadata-control-catalog-desktop').is_file():
        sizes['metadata'] = {p: (output / f'metadata-{p}').stat().st_size for p in PACKAGES[1:]}
    data = dict(sample_unit='ns', fresh_processes_per_variant=runs, repeat_count=128,
                binary_bytes=sizes, medians=summary, timings=results)
    if (output / 'metadata-binding').is_file() and (output / 'stock-binding').is_file():
        costs = {'stock': [], 'metadata': []}
        for i in range(runs + 1):
            order = list(costs)
            rng.shuffle(order)
            for tag in order:
                result = subprocess.check_output([output / f'{tag}-binding'], text=True).strip().splitlines()[-1]
                typed, push_ns, pull_ns = result.split()
                assert (typed == 'true') == (tag == 'stock')
                if i:
                    costs[tag].append(dict(source_to_target_ns=int(push_ns), target_to_source_ns=int(pull_ns)))
        data['binding_timings'] = costs
    (output / 'results.json').write_text(json.dumps(data, indent=2) + '\n')
    print(json.dumps(dict(bytes=sizes, median_ns=summary), indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--runs', type=int, default=31)
    parser.add_argument('--measure-only', action='store_true')
    parser.add_argument('--metadata', action='store_true', help='also probe optional typed metadata hooks')
    args = parser.parse_args()
    if args.runs < 1:
        parser.error('--runs must be positive')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if not args.measure_only:
        with tempfile.TemporaryDirectory(prefix='ferroui-theme-spike-') as directory:
            target = Path(directory) / 'styles'
            env = dict(os.environ, CARGO_TARGET_DIR=str(target))
            variants(output, target, env)
            if args.metadata:
                metadata(output, Path(directory) / 'metadata', env)
    measure(output, args.runs)

if __name__ == '__main__':
    main()
