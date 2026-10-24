import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

from probe import patch_input, run, select_tip_artifacts


class CompilerReplayTests(unittest.TestCase):
    def test_desktop_snapshot_preserves_rust_line_endings_and_utf8(self):
        from desktop_patch_probe import snapshot
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = '// café\r\nfn main() {}\r\n'
            (root / 'program.rs').write_bytes(source.encode('utf-8'))
            self.assertEqual(snapshot(root, 'dependencies')['program.rs'], source)

    @unittest.skipIf(os.name == 'nt', 'POSIX interrupt delivery')
    def test_interrupt_cancels_build_before_propagating(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'worker.py').write_text(
                'import os,time\nfrom pathlib import Path\n'
                'Path("worker.pid").write_text(str(os.getpid()))\n'
                'while True:\n'
                '    with open("heartbeat", "ab") as output: output.write(b"x")\n'
                '    time.sleep(.02)\n')
            runner = subprocess.Popen([sys.executable, '-c',
                'import sys; sys.path.insert(0, sys.argv[1]); from probe import run; '
                'run([sys.executable, "worker.py"], ".", timeout=60)',
                str(Path(__file__).resolve().parent)], cwd=root,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                deadline = time.monotonic() + 5
                while not (root / 'heartbeat').exists():
                    self.assertIsNone(runner.poll())
                    self.assertLess(time.monotonic(), deadline)
                    time.sleep(.02)
                runner.send_signal(signal.SIGINT)
                _, stderr = runner.communicate(timeout=5)
                self.assertNotEqual(runner.returncode, 0)
                self.assertIn(b'KeyboardInterrupt', stderr)
                before = (root / 'heartbeat').read_bytes()
                time.sleep(.2)
                self.assertEqual((root / 'heartbeat').read_bytes(), before)
            finally:
                if runner.poll() is None:
                    runner.kill()
                runner.communicate()
                pid_file = root / 'worker.pid'
                if pid_file.exists():
                    try:
                        os.kill(int(pid_file.read_text()), signal.SIGKILL)
                    except ProcessLookupError:
                        pass

    def test_build_runner_preserves_input_environment_and_diagnostics(self):
        with tempfile.TemporaryDirectory() as directory:
            command = [sys.executable, '-c',
                'import os,sys; print(os.environ["FERROUI_TEST_VALUE"] + sys.stdin.read())']
            self.assertEqual(run(command, directory, 'input', env=dict(os.environ,
                FERROUI_TEST_VALUE='prefix:')), 'prefix:input\n')
            with self.assertRaisesRegex(RuntimeError, 'diagnostic'):
                run([sys.executable, '-c', 'import sys; sys.stderr.write("diagnostic"); sys.exit(3)'], directory)
            # dx puts compiler diagnostics in JSON on stdout. Later build progress
            # must not hide the original error when a native build fails.
            with self.assertRaises(RuntimeError) as failure:
                run([sys.executable, '-c', 'import sys; '
                    'print("original compiler error"); print("x" * 7000); sys.exit(3)'], directory)
            self.assertIn('original compiler error', str(failure.exception))

    def test_timeout_stops_descendants_even_after_compiler_parent_exits(self):
        for parent_exits in [False, True]:
            with self.subTest(parent_exits=parent_exits), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'worker.py').write_text(
                    'import os,time\nfrom pathlib import Path\n'
                    'Path("worker.pid").write_text(str(os.getpid()))\n'
                    'while True:\n'
                    '    with open("heartbeat", "ab") as output: output.write(b"x")\n'
                    '    time.sleep(.02)\n')
                parent = ('import subprocess,sys,time; '
                    'subprocess.Popen([sys.executable, "worker.py"]); '
                    + ('sys.exit(0)' if parent_exits else 'time.sleep(60)'))
                try:
                    with self.assertRaises(subprocess.TimeoutExpired):
                        run([sys.executable, '-c', parent], root, timeout=1)
                    # Inherited stdout keeps communicate waiting even when the
                    # immediate compiler exits. Its process group still owns work.
                    heartbeat = root / 'heartbeat'
                    self.assertTrue(heartbeat.exists(), 'descendant must run before cancellation')
                    time.sleep(.1)
                    before = heartbeat.read_bytes()
                    time.sleep(.2)
                    self.assertEqual(heartbeat.read_bytes(), before, 'orphan compiler is still writing')
                finally:
                    pid_file = root / 'worker.pid'
                    if pid_file.exists():
                        try:
                            os.kill(int(pid_file.read_text()), signal.SIGTERM if os.name == 'nt' else signal.SIGKILL)
                        except ProcessLookupError:
                            pass
                        except OSError as error:
                            # Windows reports an already exited PID as invalid.
                            if os.name != 'nt' or error.winerror != 87:
                                raise

    def test_selects_application_capture_instead_of_build_script(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scope = root / 'target/dx/.captured-args/fat'
            scope.mkdir(parents=True)
            application = {'args': ['rustc', '--crate-name', 'demo', 'program.rs'],
                           'envs': [['CARGO_PKG_NAME', 'demo']]}
            (scope / 'demo.bin.json').write_text(json.dumps(application))
            (scope / 'build_script_build.bin.json').write_text(json.dumps({
                'args': ['rustc', '--crate-name', 'build_script_build', 'build.rs'], 'envs': []}))
            selected = select_tip_artifacts({'exe': 'unchanged', 'rustc_args': ['wrong']}, root, 'demo', 'program.rs')
            self.assertEqual(selected['rustc_args'], application['args'])
            self.assertEqual(selected['exe'], 'unchanged')
            with self.assertRaises(RuntimeError):
                select_tip_artifacts(selected, root, 'demo', 'other.rs')
            duplicate = scope.parent / 'other'
            duplicate.mkdir()
            (duplicate / 'demo.bin.json').write_text(json.dumps(application))
            with self.assertRaises(RuntimeError):
                select_tip_artifacts(selected, root, 'demo', 'program.rs')

    def test_only_process_local_linker_and_jobserver_values_are_removed(self):
        original = {'rustc_envs': [['DX_LINK_ARGS_FILE', '/stale'], ['CARGO_MAKEFLAGS', 'closed-fds'],
            ['CARGO_PKG_NAME', 'demo'], ['OUT_DIR', '/exact-build-output']], 'rustc_args': ['exact']}
        replay = patch_input(original)
        self.assertEqual(dict(replay['rustc_envs']), dict(original['rustc_envs'][2:]))
        self.assertEqual(len(original['rustc_envs']), 4)
        self.assertEqual(replay['rustc_args'], ['exact'])


if __name__ == '__main__':
    unittest.main()
