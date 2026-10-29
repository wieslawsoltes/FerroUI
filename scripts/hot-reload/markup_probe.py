#!/usr/bin/env python3
"""Run the built desktop demo through valid, invalid, and atomic-save revisions.

Requires `cargo build -p hot-reload-demo` and a desktop session. This checks
native execution, reported transactions and save-to-render completion. It does
not measure physical display scan-out or inspect screenshot correctness.
"""
import os
from pathlib import Path
import subprocess
import tempfile
import time
from catalog_probe import save_and_render

ROOT = Path(__file__).resolve().parents[2]


def wait_for(log, text, process, timeout=15):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if text in log.read_text(encoding='utf-8', errors='replace'):
            return
        if process.poll() is not None:
            raise RuntimeError(f'Demo exited before {text}:\n{log.read_text(encoding="utf-8", errors="replace")}')
        time.sleep(0.025)
    raise RuntimeError(f'Timed out waiting for {text}:\n{log.read_text(encoding="utf-8", errors="replace")}')


def main():
    binary = Path(os.environ.get('FERROUI_HOT_DEMO', str(
        ROOT / 'target/debug' / ('hot-reload-demo.exe' if os.name == 'nt' else 'hot-reload-demo'))))
    source = (ROOT / 'samples/HotReload/View.xaml').read_text(encoding='utf-8')
    with tempfile.TemporaryDirectory(prefix='ferroui-markup-probe-') as directory:
        path = Path(directory) / 'View.xaml'
        log = Path(directory) / 'demo.log'
        for document in (ROOT / 'samples/HotReload').glob('*.xaml'):
            (Path(directory) / document.name).write_text(document.read_text(encoding='utf-8'), encoding='utf-8')
        path.write_text(source, encoding='utf-8')
        env = dict(os.environ, FERROUI_HOT_VIEW=str(path), FERROUI_SMOKE_EXIT_MS='30000', FERROUI_RELOAD_TIMINGS='1')
        with log.open('w', encoding='utf-8') as output:
            process = subprocess.Popen([str(binary)], env=env, stdout=output, stderr=output)
            try:
                wait_for(log, 'status: Applied', process)
                save_and_render(process, log, path, source.replace('Spacing="12"', 'Spacing="18"'))
                path.write_text('<StackPanel', encoding='utf-8')
                wait_for(log, 'revision: 3,', process)
                save_and_render(process, log, path, source.replace('Spacing="12"', 'Spacing="24"'), atomic=True)
                colors = Path(directory) / 'Colors.xaml'
                save_and_render(process, log, colors, colors.read_text(encoding='utf-8').replace('#2563EB', '#DB2777'))
                result = log.read_text(encoding='utf-8', errors='replace')
                assert result.count('status: Applied') == 4, result
                assert result.count('status: Failed') == 1, result
                print(result)
                print(f'PASS: PID {process.pid} applied edits and recovered from invalid markup')
            finally:
                if process.poll() is None:
                    process.terminate()
                process.wait(timeout=10)


if __name__ == '__main__':
    main()
