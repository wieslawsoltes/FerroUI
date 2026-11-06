"""CLI protocol checks only; fake adb output is never native input evidence."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class RunnerChecks(unittest.TestCase):
    def setUp(self):
        base = Path('/Volumes/SSD/temp/ferrokeyboard')
        base.mkdir(parents=True, exist_ok=True)
        self.folder = tempfile.TemporaryDirectory(prefix='instrumentation-runner-', dir=base)
        self.addCleanup(self.folder.cleanup)
        self.root = Path(self.folder.name)
        self.calls = self.root / 'calls.jsonl'
        self.adb = self.root / 'fake-adb'
        self.adb.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
with pathlib.Path(os.environ['FERROBOARD_RUNNER_CALLS']).open('a') as output:
    output.write(json.dumps(sys.argv[1:]) + '\\n')
print('INSTRUMENTATION_RESULT: status=passed')
print('INSTRUMENTATION_RESULT: passed=23')
print('INSTRUMENTATION_CODE: -1')
''')
        self.adb.chmod(0o755)
        self.env = dict(os.environ, FERROBOARD_RUNNER_CALLS=str(self.calls))

    def run_cli(self, *args):
        return subprocess.run([
            sys.executable, str(Path(__file__).with_name('run_privacy_checks.py')),
            '--adb', str(self.adb), '--serial', 'protocol-only',
            '--log', str(self.root / 'result.log'), *args,
        ], env=self.env, capture_output=True, text=True, timeout=10)

    def test_default_password_contract_is_unchanged(self):
        result = self.run_cli()
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = json.loads((self.root / 'result.log.summary.json').read_text())
        self.assertEqual(summary['expected_checks'], 23)
        self.assertEqual(summary['assertions'], 23)
        calls = [json.loads(line) for line in self.calls.read_text().splitlines()]
        self.assertEqual(calls, [['-s', 'protocol-only', 'shell', 'am', 'instrument', '-w', '-r',
            'org.ferroboard.privacyconsumer/org.ferroboard.consumer.PrivacyChecks']])

    def test_explicit_fixture_routes_and_rejects_wrong_count(self):
        result = self.run_cli('--package', 'org.ferroboard.emojiconsumer',
            '--instrumentation', 'org.ferroboard.consumer.EmojiChecks', '--expected-checks', '18')
        self.assertEqual(result.returncode, 1, result.stderr)
        summary = json.loads((self.root / 'result.log.summary.json').read_text())
        self.assertEqual(summary['status'], 'failed')
        self.assertFalse(summary['retry_failed_runs'])
        self.assertEqual(len(summary['runs']), 1)
        self.assertEqual(json.loads(self.calls.read_text())[-1],
            'org.ferroboard.emojiconsumer/org.ferroboard.consumer.EmojiChecks')

    def test_invalid_component_never_invokes_adb(self):
        result = self.run_cli('--package', 'org.ferroboard;invalid')
        self.assertEqual(result.returncode, 2)
        self.assertFalse(self.calls.exists())

    def test_nonpositive_check_count_never_invokes_adb(self):
        result = self.run_cli('--expected-checks', '0')
        self.assertEqual(result.returncode, 2)
        self.assertFalse(self.calls.exists())


if __name__ == '__main__':
    unittest.main()
