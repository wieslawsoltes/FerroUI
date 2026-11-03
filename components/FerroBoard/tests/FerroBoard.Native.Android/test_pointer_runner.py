"""False-success guards for the native instrumentation runner, without a device."""
import contextlib
import io
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import run_pointer_checks as runner
from run_pointer_checks import successful_result

COMPLETE = ('INSTRUMENTATION_RESULT: passed=31\n'
            'INSTRUMENTATION_RESULT: status=passed\n'
            'INSTRUMENTATION_CODE: -1\n')


class NativeResultTests(unittest.TestCase):
    def test_requires_complete_native_result(self):
        self.assertTrue(successful_result(0, COMPLETE))
        self.assertFalse(successful_result(0, ''))
        self.assertFalse(successful_result(1, COMPLETE))
        self.assertFalse(successful_result(0, COMPLETE.replace('passed=31', 'passed=30')))
        self.assertFalse(successful_result(0, COMPLETE.replace('passed=31\n', '')))
        self.assertFalse(successful_result(0, COMPLETE.replace('CODE: -1', 'CODE: 0')))

    def test_rejects_failures_and_contradictory_results(self):
        for extra in ['INSTRUMENTATION_RESULT: failure=bad\n',
                      'INSTRUMENTATION_RESULT: cleanup_failure=bad\n',
                      'INSTRUMENTATION_RESULT: status=failed\n',
                      'INSTRUMENTATION_RESULT: status=passed\n',
                      'INSTRUMENTATION_CODE: -1\n',
                      'INSTRUMENTATION_FAILED: aborted\n']:
            with self.subTest(extra=extra):
                self.assertFalse(successful_result(0, COMPLETE + extra))

    def test_timeout_stops_only_owned_package_and_records_failure(self):
        self.check_timeout(subprocess.CompletedProcess([], 0, 'stopped', ''))

    def test_cleanup_timeout_is_recorded_as_failure(self):
        self.check_timeout(subprocess.TimeoutExpired([], 15, output=b'cleanup partial'))

    def check_timeout(self, cleanup_result):
        with tempfile.TemporaryDirectory(prefix='pointer-runner-', dir=runner.TEMP) as folder:
            log = Path(folder) / 'result.json'
            args = ['run_pointer_checks.py', '--adb', '/mock/adb', '--serial', 'owned-test',
                    '--log', str(log), '--repeat', '3', '--timeout', '0.01']
            with patch('sys.argv', args), patch.object(runner.subprocess, 'run', side_effect=[
                    subprocess.TimeoutExpired([], 0.01, output=b'partial native result'), cleanup_result]) as run, \
                    contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(runner.main(), 1)
            self.assertEqual(run.call_count, 2, 'failure stops remaining requested runs')
            self.assertEqual(run.call_args.args[0], ['/mock/adb', '-s', 'owned-test', 'shell',
                                                   'am', 'force-stop', 'org.ferroboard.consumer'])
            self.assertEqual(log.read_text(), 'partial native result\nDriver timeout\n')
            record = json.loads(log.with_name(log.name + '.summary.json').read_text())
            self.assertEqual(record['status'], 'failed')
            self.assertTrue(record['runs'][0]['timed_out'])
            self.assertEqual(record['runs'][0]['timeout_cleanup']['returncode'],
                             None if isinstance(cleanup_result, Exception) else 0)


if __name__ == '__main__':
    unittest.main()
