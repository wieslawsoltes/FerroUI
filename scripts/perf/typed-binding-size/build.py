#!/usr/bin/env python3
"""Build the four binding source comparisons in a disposable macOS worktree."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

from warm_probe import source as warm_source

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent
BASE = '933fd46325772fd1cdec81add9d4912dda2fe931'
PR = '8678e15b8ae738931237c0e70ef66f61a6ecbfd2'
SOURCE = 'src/FerroUI.Base/data/core/typed_binding_expression.rs'


def run(args, **kwargs):
    return subprocess.run(args, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path, help='new directory for the worktree, target, maps and binaries')
    parser.add_argument('--arms', nargs='+', choices=['stock', 'pr', 'history', 'axes', 'value_only'],
                        default=['stock', 'pr', 'history', 'axes'])
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    tree = out / 'workspace'
    run(['git', 'worktree', 'add', '--detach', str(tree), BASE], cwd=ROOT)
    env = dict(os.environ, CARGO_TARGET_DIR=str(out / 'target'))
    env.setdefault('SDKROOT', subprocess.check_output(['xcrun', '--sdk', 'macosx', '--show-sdk-path'], text=True).strip())
    for arm in args.arms:
        revision = {'stock': BASE, 'pr': PR, 'history': '86159449b', 'axes': PR, 'value_only': PR}[arm]
        (tree / SOURCE).write_bytes(subprocess.check_output(['git', 'show', revision + ':' + SOURCE], cwd=ROOT))
        if arm in ('axes', 'value_only'):
            run(['git', 'apply', str(HERE / 'controller.patch')], cwd=tree)
        if arm == 'value_only':
            run(['git', 'apply', str(HERE / 'without-source-slot.patch')], cwd=tree)
        if arm == 'axes':
            with (out / 'tests.log').open('w') as log:
                run(['cargo', 'test', '--locked', '-p', 'ferroui-base', '--lib', '--features', 'hot-reload', 'binding'],
                    cwd=tree, env=env, stdout=log, stderr=subprocess.STDOUT)
        log = out / (arm + '-cargo.jsonl')
        with log.open('w') as stdout, (out / (arm + '-build.log')).open('w') as stderr:
            run(['cargo', 'rustc', '--locked', '--profile', 'dist', '--message-format=json', '-p',
                 'control-catalog-desktop', '--bin', 'control-catalog-desktop', '--',
                 '-C', 'link-arg=-Wl,-map,' + str(out / (arm + '.map'))], cwd=tree, env=env, stdout=stdout, stderr=stderr)
        shutil.copy2(out / 'target/dist/control-catalog-desktop', out / arm)
        artifacts = [json.loads(line) for line in log.read_text().splitlines() if line.startswith('{')]
        libraries = {Path(path) for item in artifacts if item.get('reason') == 'compiler-artifact'
                     and item['target']['name'] == 'ferroui_base' and item['profile']['opt_level'] == '3'
                     and set(item['features']) == {'full-themes', 'markup-functions', 'type-tables'}
                     for path in item['filenames'] if path.endswith('.rlib')}
        if len(libraries) != 1:
            raise RuntimeError(f'Expected one production Base library, found {libraries} in {log}')
        library = libraries.pop()
        original_probe = ROOT / 'scripts/themes/compact-spike/binding_bench.rs'
        warm = out / 'warm.rs'
        warm.write_text(warm_source(original_probe))
        for probe in ['binding', 'cohort', 'lifecycle', 'ownership', 'warm']:
            driver = {'binding': original_probe, 'warm': warm}.get(probe, HERE / (probe + '.rs'))
            run(['rustc', str(driver), '--edition=2021', '-C', 'opt-level=3', '-C', 'panic=abort',
                 '-C', 'codegen-units=1', '-C', 'lto=fat', '-C', 'strip=symbols',
                 '--extern', 'ferroui_base=' + str(library), '-L', 'dependency=' + str(library.parent),
                 '-o', str(out / (arm + '-' + probe))], env=dict(env, CARGO_CRATE_NAME='binding_bench'))
        with (out / (arm + '-families.json')).open('w') as stdout:
            run(['python3', str(HERE / 'map_families.py'), str(out / (arm + '.map'))], stdout=stdout)


if __name__ == '__main__':
    main()
