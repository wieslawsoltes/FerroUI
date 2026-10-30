#!/usr/bin/env python3
"""Native placement/navigation/application-state restoration, without a Rust patch.

Build first: cargo build -p hot-reload-demo --features patch-control
This complements supervisor_probe.py's build-before-retire coverage: both processes
here use the same executable, so state restoration needs only one ordinary build.
"""
import os
from pathlib import Path
import tempfile
import time
import sys

from supervisor import ROOT, Application, stable_capture


def settled(application, placement):
    deadline = time.monotonic() + 10
    while True:
        response = application.command({'op': 'state'})
        assert response['status'] == 'state', response
        state = response['state']
        if state['windows'] == [placement]:
            return state
        if time.monotonic() >= deadline:
            raise AssertionError(f'Window did not settle at {placement}: {state}')
        time.sleep(.05)


def main():
    target = Path(os.environ.get('CARGO_TARGET_DIR', str(ROOT / 'target')))
    executable = (target / 'debug' / ('hot-reload-demo.exe' if os.name == 'nt' else 'hot-reload-demo')).resolve()
    if not executable.is_file():
        raise RuntimeError('Build hot-reload-demo with --features patch-control first')
    files = stable_capture(ROOT)
    client = {'exe': str(executable)}
    with tempfile.TemporaryDirectory(prefix='ferroui-restart-state-') as directory:
        directory = Path(directory)
        initial_dir, replacement_dir = directory / 'initial', directory / 'replacement'
        initial_dir.mkdir()
        replacement_dir.mkdir()
        original = replacement = None
        try:
            original = Application(client, ROOT, files, initial_dir)
            assert original.command({'op': 'click'})['counter'] == 1
            assert original.command({'op': 'click'})['counter'] == 2
            placement = {'key': 'main', 'position': [80, 100], 'size': [520.0, 300.0], 'maximized': False}
            assert original.command({'op': 'place_window', 'placement': placement})['status'] == 'placed'
            settled(original, placement)
            invalid = dict(placement, size=[-1.0, 300.0])
            assert original.command({'op': 'place_window', 'placement': invalid})['status'] == 'rejected'
            before = settled(original, placement)
            frozen = original.command({'op': 'prepare_restart'})
            assert frozen['state'] == before
            assert frozen['interaction_enabled'] is False
            assert original.command({'op': 'place_window', 'placement': placement})['status'] == 'rejected'
            replacement = Application(client, ROOT, files, replacement_dir, before)
            after = settled(replacement, placement)
            assert original.process.poll() is None, 'Old app exited before replacement validation'
            assert after == before, (before, after)
            assert after['navigation'] == 'main'
            assert replacement.command({'op': 'click'})['counter'] == 3
            print('PASS: native restart restored non-default window placement, navigation and counter; invalid/frozen placement changes were rejected', flush=True)
        except BaseException:
            for log in directory.glob('*/application.log'):
                print(f'{log.parent.name} application log:\n' + log.read_text(encoding='utf-8', errors='replace')[-12000:], file=sys.stderr)
            raise
        finally:
            if replacement is not None:
                replacement.close()
            if original is not None:
                original.close()


if __name__ == '__main__':
    main()
