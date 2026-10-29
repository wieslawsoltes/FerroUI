#!/usr/bin/env python3
"""Build an isolated desktop demo and apply admitted patches through its UI channel.

This acceptance driver exclusively owns the source tree and compiler. It never
edits the repository. External framework inputs are fingerprinted before/after
builds; this is not yet the production watcher/restart supervisor.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

from probe import ROOT, events, patch_input, read, run, select_tip_artifacts


def dependency_fingerprint():
    digest = hashlib.sha256()
    paths = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock']
    for directory in [ROOT / 'src', ROOT / '.cargo']:
        if directory.exists():
            paths.extend(path for path in directory.rglob('*') if path.is_file())
    for path in sorted(paths):
        digest.update(str(path.relative_to(ROOT)).encode())
        digest.update(b'\0')
        digest.update(hashlib.sha256(path.read_bytes()).digest())
    return digest.hexdigest()


def snapshot(root, dependencies):
    sources = {'@context/dependencies': dependencies}
    for path in sorted(root.rglob('*')):
        relative = path.relative_to(root)
        if relative.parts[0] == 'target' or not path.is_file():
            continue
        data = path.read_bytes()
        sources[relative.as_posix()] = data.decode('utf-8') if path.suffix == '.rs' else data.hex()
    return sources


def main():
    dx = os.environ.get('FERROUI_DX', 'dx')
    pinned = json.loads(Path(__file__).with_name('toolchain.json').read_text())
    assert run(['rustc', '--version'], ROOT).split()[1] == pinned['rust']
    assert run([dx, '--version'], ROOT).split()[1] == pinned['dioxus_cli']
    metadata = json.loads(run(['cargo', 'metadata', '--no-deps', '--format-version', '1'], ROOT))
    workspace = {package['name']: Path(package['manifest_path']).parent for package in metadata['packages']}
    sample = ROOT / 'samples/HotReload'
    with tempfile.TemporaryDirectory(prefix='ferroui-desktop-patch-') as directory:
        outer = Path(directory)
        root = outer / 'app'
        root.mkdir()
        for source in sample.iterdir():
            if source.suffix in ('.rs', '.xaml'):
                shutil.copy2(source, root / source.name)
        dependencies = ['ferroui-base', 'ferroui-controls', 'ferroui-themes-simple',
            'ferroui-markup-xaml', 'ferroui-hot-reload', 'ferroui-hot-patch', 'ferroui-desktop']
        manifest = '[package]\nname="hot-reload-demo"\nversion="0.1.0"\nedition="2021"\n'
        manifest += '[lib]\npath="lib.rs"\n[[bin]]\nname="hot-reload-demo"\npath="program.rs"\n'
        manifest += '[features]\ndefault=["desktop","patch-control"]\ndesktop=[]\npatch-control=[]\n'
        manifest += '[dependencies]\nserde_json="1"\n'
        for name in dependencies:
            manifest += f'{name}={{path={json.dumps(str(workspace[name]))}}}\n'
        manifest += '[build-dependencies]\nferroui-build={path=' + json.dumps(str(workspace['ferroui-build'])) + '}\n'
        (root / 'Cargo.toml').write_text(manifest)
        dependency_state = dependency_fingerprint()
        build_env = dict(os.environ, CARGO_TARGET_DIR=str(root / 'target'))
        print('Building isolated desktop fat binary...', flush=True)
        built = run([dx, 'build', '--fat-binary', '--platform', 'server', '--json-output'], root, timeout=1200, env=build_env)
        assert dependency_fingerprint() == dependency_state, 'Framework inputs changed during build'
        client = next(event['BuildsFinished']['client'] for event in events(built) if 'BuildsFinished' in event)
        client = select_tip_artifacts(client, root, 'hot_reload_demo', 'program.rs')
        baseline = outer / 'baseline.json'
        baseline.write_text(json.dumps(snapshot(root, dependency_state)))
        with (outer / 'demo.log').open('w') as log:
            process = subprocess.Popen([client['exe']], cwd=root, stdin=subprocess.PIPE,
                stdout=subprocess.PIPE, stderr=log, text=True,
                env=dict(os.environ, FERROUI_PATCH_BASELINE=str(baseline)))
            def response():
                while True:
                    line = read(process)
                    if line.startswith('FERROUI_DEV='):
                        return json.loads(line.removeprefix('FERROUI_DEV='))
            def command(message):
                process.stdin.write(json.dumps(message) + '\n')
                process.stdin.flush()
                return response()
            try:
                ready = response()
                assert ready['status'] == 'ready', ready
                initial = command({'op': 'click'})
                assert initial['counter'] == 1, initial
                source = root / 'program.rs'
                original = source.read_text(encoding='utf-8')
                expected = 1
                view = (root / 'View.xaml').read_text(encoding='utf-8')
                uri = 'ferres://hot_reload_demo/View.xaml'
                for increment in [2, 3, 4]:
                    source.write_text(original.replace('hot_reload_demo::increment(1)',
                        f'hot_reload_demo::increment({increment})'), encoding='utf-8')
                    sources = snapshot(root, dependency_state)
                    # The supervisor sends a full markup snapshot even when only
                    # a handler changed. The third patch must retain the child graph.
                    markup = {uri: view.replace('Spacing="12"', f'Spacing="{min(increment, 3) * 12}"')}
                    admitted = command({'op': 'admit', 'sources': sources,
                        'markup': {uri: '<'} if increment == 2 else markup})
                    assert admitted['status'] == 'admitted', admitted
                    assert admitted['views'] == [], admitted
                    print(f'Building admitted desktop patch {increment}...', flush=True)
                    try:
                        output = run([dx, 'tools', 'hotpatch', '--aslr-reference', str(ready['aslr']),
                            '--platform', 'server', '--json-output'], root, json.dumps(patch_input(client)), env=build_env)
                        assert snapshot(root, dependency_fingerprint()) == sources, 'Superseded build inputs'
                        table = next(event['Hotpatch']['jump_table'] for event in events(output) if 'Hotpatch' in event)
                        # First prove invalid markup blocks machine-code installation; then
                        # prove a load failure rolls back the newly committed UI graph.
                        if increment < 4:
                            failing_table = dict(table, lib=str(root / 'missing-patch-library')) if increment == 3 else table
                            rejected = command({'op': 'apply', 'request': admitted['request'],
                                'table': failing_table, 'sources': sources})
                            assert rejected['status'] == 'rejected', rejected
                            expected += increment - 1
                            retained = command({'op': 'click'})
                            assert retained['counter'] == expected, retained
                            assert retained['spacing'] == (increment - 1) * 12, retained
                            admitted = command({'op': 'admit', 'sources': sources, 'markup': markup})
                            assert admitted['status'] == 'admitted', admitted
                        applied = command({'op': 'apply', 'request': admitted['request'], 'table': table, 'sources': sources})
                        assert applied['status'] == 'applied', applied
                    except BaseException:
                        command({'op': 'cancel'})
                        raise
                    expected += increment
                    previous = initial if increment == 2 else current
                    current = command({'op': 'click'})
                    assert current['counter'] == expected, current
                    assert current['spacing'] == min(increment, 3) * 12, current
                    assert (current['host'], current['type']) == (initial['host'], initial['type']), current
                    if increment == 4:
                        assert current['content'] is not None, current
                        assert (current['content'], current['markup_revision']) == (
                            previous['content'], previous['markup_revision']), (previous, current)
                print(f'PASS: desktop PID {process.pid}, live root, type identity and existing handler survived two combined Rust/XAML revisions and both rollback paths; handler-only patch retained child identity and markup revision', flush=True)
                callback_source = source.read_text(encoding='utf-8')
                for body, accepted in [
                    ('    "<".to_string()\n}', False),
                    ('    source.replace("Spacing=\\"36\\"", "Spacing=\\"48\\"")\n}', True),
                ]:
                    source.write_text(callback_source.replace('    source\n}', body), encoding='utf-8')
                    sources = snapshot(root, dependency_state)
                    admitted = command({'op': 'admit', 'sources': sources, 'markup': markup})
                    assert admitted['status'] == 'admitted' and admitted['views'] == ['main'], admitted
                    output = run([dx, 'tools', 'hotpatch', '--aslr-reference', str(ready['aslr']),
                        '--platform', 'server', '--json-output'], root, json.dumps(patch_input(client)), env=build_env)
                    assert snapshot(root, dependency_fingerprint()) == sources, 'Superseded builder inputs'
                    table = next(event['Hotpatch']['jump_table'] for event in events(output) if 'Hotpatch' in event)
                    applied = command({'op': 'apply', 'request': admitted['request'], 'table': table, 'sources': sources})
                    assert applied['status'] == ('applied' if accepted else 'rejected'), applied
                    previous = current
                    current = command({'op': 'click'})
                    expected += 4
                    assert current['counter'] == expected and current['spacing'] == (48 if accepted else 36), current
                    assert (current['host'], current['type']) == (initial['host'], initial['type']), current
                    if not accepted:
                        assert (current['content'], current['markup_revision']) == (
                            previous['content'], previous['markup_revision']), (previous, current)
                    else:
                        assert current['content'] != previous['content'], (previous, current)
                # A handler patch after a derived view must not overwrite it with
                # the supervisor's unchanged authored XAML snapshot.
                source.write_text(source.read_text(encoding='utf-8').replace('hot_reload_demo::increment(4)',
                    'hot_reload_demo::increment(5)'), encoding='utf-8')
                sources = snapshot(root, dependency_state)
                admitted = command({'op': 'admit', 'sources': sources, 'markup': markup})
                assert admitted['status'] == 'admitted' and admitted['views'] == [], admitted
                output = run([dx, 'tools', 'hotpatch', '--aslr-reference', str(ready['aslr']),
                    '--platform', 'server', '--json-output'], root, json.dumps(patch_input(client)), env=build_env)
                assert snapshot(root, dependency_fingerprint()) == sources, 'Superseded handler inputs'
                table = next(event['Hotpatch']['jump_table'] for event in events(output) if 'Hotpatch' in event)
                assert command({'op': 'apply', 'request': admitted['request'], 'table': table,
                    'sources': sources})['status'] == 'applied'
                previous = current
                current = command({'op': 'click'})
                expected += 5
                assert current['counter'] == expected and current['spacing'] == 48, current
                assert (current['content'], current['markup_revision']) == (
                    previous['content'], previous['markup_revision']), (previous, current)
                print('PASS: registered view builder rebuilt the live view, rejected invalid candidate markup with code/UI rollback, and retained the derived view through a subsequent handler-only patch', flush=True)
            except BaseException:
                print((outer / 'demo.log').read_text())
                raise
            finally:
                process.terminate()
                process.wait(timeout=10)


if __name__ == '__main__':
    main()
