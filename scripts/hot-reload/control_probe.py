#!/usr/bin/env python3
"""Exercise the trusted supervisor channel; this does not build/apply machine code."""
import json
import os
from pathlib import Path
import queue
import subprocess
import tempfile
import threading

ROOT = Path(__file__).resolve().parents[2]

def main():
    sample = ROOT / 'samples/HotReload'
    sources = {str(path.relative_to(sample)): (path.read_text() if path.suffix == '.rs'
               else path.read_bytes().hex()) for path in sample.rglob('*') if path.is_file()}
    binary = ROOT / 'target/debug' / ('hot-reload-demo.exe' if os.name == 'nt' else 'hot-reload-demo')
    with tempfile.TemporaryDirectory(prefix='ferroui-control-') as directory:
        baseline = Path(directory) / 'baseline.json'
        baseline.write_text(json.dumps(sources))
        with (Path(directory) / 'stderr.log').open('w') as log:
            process = subprocess.Popen([str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=log, text=True, env=dict(os.environ, FERROUI_PATCH_BASELINE=str(baseline)))
            responses = queue.Queue()
            def read():
                for line in process.stdout:
                    if line.startswith('FERROUI_DEV='):
                        responses.put(json.loads(line.removeprefix('FERROUI_DEV=')))
                responses.put({'status': 'exited'})
            threading.Thread(target=read, daemon=True).start()
            def command(value):
                process.stdin.write(json.dumps(value) + '\n')
                process.stdin.flush()
                return responses.get(timeout=15)
            try:
                assert responses.get(timeout=15)['status'] == 'ready'
                initial = command({'op': 'click'})
                assert initial['counter'] == 1, initial
                rejected = command({'op': 'apply', 'request': 1})
                assert rejected['status'] == 'rejected', rejected
                changed = dict(sources)
                changed['program.rs'] = changed['program.rs'].replace('hot_reload_demo::increment(1)',
                    'hot_reload_demo::increment(2)')
                admitted = command({'op': 'admit', 'sources': changed})
                assert admitted['status'] == 'admitted', admitted
                assert command({'op': 'apply', 'request': admitted['request'] + 1})['status'] == 'rejected'
                assert command({'op': 'cancel'})['status'] == 'cancelled'
                assert command({'op': 'apply', 'request': admitted['request']})['status'] == 'rejected'
                changed['lib.rs'] += '\npub struct ChangedLayout;\n'
                assert command({'op': 'admit', 'sources': changed})['status'] == 'rejected'
                final = command({'op': 'click'})
                assert final['counter'] == 2 and final['host'] == initial['host'] and final['type'] == initial['type'], final
                state = command({'op': 'state'})
                assert state['state']['application_state']['counter'] == 2, state
                print('PASS: UI command channel gates admission/apply/cancel and retains the live handler, root and state')
            finally:
                process.terminate()
                process.wait(timeout=10)

if __name__ == '__main__':
    main()
