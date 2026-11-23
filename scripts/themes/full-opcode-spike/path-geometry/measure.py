"""Measure constructing three real ControlCatalog icons using a headless geometry builder."""
import argparse
import json
import os
import shutil
from pathlib import Path
import re
import statistics
import subprocess


root = Path(__file__).resolve().parents[4]
parser = argparse.ArgumentParser(description=__doc__)
group = parser.add_mutually_exclusive_group(required=True)
group.add_argument('--binary', type=Path, help='use an already built probe')
group.add_argument('--build-in', type=Path, help='create and compile the probe in this directory')
parser.add_argument('output', type=Path)
args = parser.parse_args()
binary = args.binary
if args.build_in:
    build = args.build_in.resolve()
    build.mkdir(exist_ok=True, parents=True)
    template = root / 'target/full-opcode-spike/native'
    manifest = (template / 'Cargo.toml').read_text().replace('full-opcode-probe', 'path-geometry-probe')
    dependency = root / 'src/Markup/FerroUI.Markup'
    manifest = manifest.replace('[dependencies]', f'[dependencies]\nferroui-markup = {{ path = "{dependency}" }}')
    (build / 'Cargo.toml').write_text(manifest)
    shutil.copy2(template / 'Cargo.lock', build / 'Cargo.lock')
    shutil.copy2(Path(__file__).with_name('bench.rs'), build / 'main.rs')
    target = Path(os.environ.get('CARGO_TARGET_DIR', build / 'target')).resolve()
    subprocess.run(['cargo', 'build', '--offline', '--profile', 'dist', '--manifest-path', build / 'Cargo.toml'],
                   env=dict(os.environ, CARGO_TARGET_DIR=str(target), FERROUI_XAML_BACKEND='bytecode'), check=True)
    binary = target / 'dist/path-geometry-probe'
source = root / 'samples/ControlCatalog/Pages/DrawerPage/DrawerPageCustomizationPage.xaml'
icons = re.findall(r'<PathIcon[^>]* Data="([Mm][^"]+)"', source.read_text())
assert len(icons) == 3
results = {}
for index, icon in enumerate(icons):
    samples = {'native': [], 'bytecode': []}
    for pair in range(31):
        for mode in (samples if pair % 2 == 0 else reversed(samples)):
            values = subprocess.check_output([binary.resolve(), mode, icon], text=True).split()
            ns, count, native_bytes, packed_bytes = map(int, values)
            samples[mode].append(ns)
    results[index] = dict(path=icon, commands=count, native_table_bytes=native_bytes, packed_bytes=packed_bytes,
                          samples_ns=samples, paired_median_delta_ns=statistics.median(b-a for a,b in zip(*samples.values())))
args.output.write_text(json.dumps(results, indent=2)+'\n')
