import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

from probe import run
import windows_job


class LaunchGateTests(unittest.TestCase):
    def test_header_does_not_consume_compiler_input(self):
        # The protocol can be tested on every platform without Win32 APIs.
        command = [sys.executable, '-c', 'import sys; '
            'sys.stdin.reconfigure(encoding="utf-8"); '
            'sys.stdout.reconfigure(encoding="utf-8"); print(repr(sys.stdin.read()))']
        payload = 'first\nsecond λ\n'
        result = subprocess.run([sys.executable, windows_job.__file__, '--child'],
            input=json.dumps(command) + '\n' + payload, text=True, encoding='utf-8',
            capture_output=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), repr(payload))

    @unittest.skipUnless(os.name == 'nt', 'Windows job assignment')
    def test_assignment_failure_never_starts_requested_command(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / 'started'
            command = [sys.executable, '-c',
                'from pathlib import Path; Path("started").write_text("unexpected")']
            with patch.object(windows_job.Job, 'assign', side_effect=OSError('assignment rejected')):
                with self.assertRaisesRegex(OSError, 'assignment rejected'):
                    run(command, directory)
            self.assertFalse(marker.exists())

    @unittest.skipUnless(os.name == 'nt', 'Windows kill-on-close ownership')
    def test_supervisor_death_stops_owned_compiler(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'worker.py').write_text(
                'import time\n'
                'while True:\n'
                '    with open("heartbeat", "ab") as output: output.write(b"x")\n'
                '    time.sleep(.02)\n')
            owner = subprocess.Popen([sys.executable, '-c',
                'import sys; sys.path.insert(0, sys.argv[1]); from probe import run; '
                'run([sys.executable, "worker.py"], ".", timeout=30)',
                str(Path(__file__).resolve().parent)], cwd=root,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                deadline = time.monotonic() + 10
                while not (root / 'heartbeat').exists():
                    self.assertIsNone(owner.poll())
                    self.assertLess(time.monotonic(), deadline)
                    time.sleep(.02)
                owner.kill()
                owner.wait(timeout=5)
                time.sleep(.2)
                before = (root / 'heartbeat').read_bytes()
                time.sleep(.2)
                self.assertEqual((root / 'heartbeat').read_bytes(), before)
            finally:
                if owner.poll() is None:
                    owner.kill()
                owner.wait(timeout=5)


if __name__ == '__main__':
    unittest.main()
