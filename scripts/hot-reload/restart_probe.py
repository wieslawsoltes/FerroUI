#!/usr/bin/env python3
"""Check desktop startup restoration and acknowledgement; requires a built demo."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def main():
    binary = Path(os.environ.get('FERROUI_HOT_DEMO', str(
        ROOT / 'target/debug' / ('hot-reload-demo.exe' if os.name == 'nt' else 'hot-reload-demo'))))
    with tempfile.TemporaryDirectory(prefix='ferroui-restart-probe-') as directory:
        directory = Path(directory)
        state = directory / 'state.json'
        ready = directory / 'ready'
        state.write_text(json.dumps({
            'format_version': 1, 'application': 'hot-reload-demo', 'application_version': 1,
            'navigation': 'main', 'application_state': {'counter': 41},
            'windows': [{'key': 'main', 'position': [80, 90], 'size': [600, 400], 'maximized': False}],
        }))
        env = dict(os.environ, FERROUI_RESTART_STATE=str(state), FERROUI_RESTART_READY=str(ready),
                   FERROUI_SMOKE_EXIT_MS='2500')
        with (directory / 'demo.log').open('w') as log:
            process = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 15
                while not ready.exists():
                    if process.poll() is not None or time.monotonic() > deadline:
                        raise RuntimeError((directory / 'demo.log').read_text())
                    time.sleep(0.025)
                assert ready.read_bytes() == b'ferroui-ready-v1'
                assert process.poll() is None, 'Replacement exited before handoff'
                assert process.wait(timeout=15) == 0
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=10)
        assert 'status: Applied' in (directory / 'demo.log').read_text()
        print('PASS: desktop replacement restored compatible state and acknowledged opened window')


if __name__ == '__main__':
    main()
