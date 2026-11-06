#!/usr/bin/env python3
"""Run native pointer checks without retries; shell exit 0 alone is insufficient."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import sys
import time

EXPECTED_CHECKS = 31
TEMP = Path('/Volumes/SSD/temp/ferrokeyboard')


def successful_result(returncode, output):
    """Require one complete success, not a partial suite or a contradictory result."""
    return (returncode == 0
            and re.findall(r'^INSTRUMENTATION_RESULT: status=(.*)$', output, re.M) == ['passed']
            and re.findall(r'^INSTRUMENTATION_CODE: (-?\d+)\s*$', output, re.M) == ['-1']
            and re.findall(r'^INSTRUMENTATION_RESULT: passed=(\d+)\s*$', output, re.M)
            == [str(EXPECTED_CHECKS)]
            and 'INSTRUMENTATION_RESULT: failure=' not in output
            and 'INSTRUMENTATION_RESULT: cleanup_failure=' not in output
            and 'INSTRUMENTATION_FAILED' not in output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adb', type=Path, required=True)
    parser.add_argument('--serial', required=True, help='Explicit task-owned test device')
    parser.add_argument('--log', type=Path, required=True)
    parser.add_argument('--repeat', type=int, default=1, help='Consecutive full runs; first failure stops')
    parser.add_argument('--timeout', type=float, default=180, help='Seconds allowed per instrumentation run')
    args = parser.parse_args()
    if args.repeat < 1 or not math.isfinite(args.timeout) or args.timeout <= 0:
        parser.error('repeat and timeout must be positive')
    log = args.log.resolve()
    summary = log.with_name(log.name + '.summary.json')
    logs = ([log] if args.repeat == 1 else
            [log.with_name(log.stem + '-run-' + str(i + 1) + log.suffix)
             for i in range(args.repeat)])
    if (not log.is_relative_to(TEMP) or log.exists() or summary.exists()
            or any(p.exists() for p in logs)):
        parser.error('all logs/summary must be new SSD-local files')
    log.parent.mkdir(parents=True, exist_ok=True)
    command = [str(args.adb), '-s', args.serial, 'shell', 'am', 'instrument',
               '-w', '-r', 'org.ferroboard.consumer/org.ferroboard.consumer.PointerChecks']
    record = {'status': 'running', 'serial': args.serial, 'expected_checks_per_run': EXPECTED_CHECKS,
              'runs_requested': args.repeat, 'retry_failed_runs': False, 'runs': []}
    for index, output_path in enumerate(logs):
        start = time.monotonic()
        timed_out = False
        timeout_cleanup = None
        try:
            result = subprocess.run(command, capture_output=True, text=True, timeout=args.timeout)
            output, code = result.stdout + result.stderr, result.returncode
        except subprocess.TimeoutExpired as failure:
            timed_out = True
            def decode(value):
                return value.decode(errors='replace') if isinstance(value, bytes) else (value or '')
            output = decode(failure.stdout) + decode(failure.stderr) + '\nDriver timeout\n'
            code = None
            # The instrumentation process may still own contacts after the adb client
            # times out. Stop only this test package on the explicit owned device.
            try:
                cleanup = subprocess.run([str(args.adb), '-s', args.serial, 'shell', 'am',
                                          'force-stop', 'org.ferroboard.consumer'],
                                         capture_output=True, text=True, timeout=15)
                timeout_cleanup = {'returncode': cleanup.returncode,
                                   'output': cleanup.stdout + cleanup.stderr}
            except subprocess.TimeoutExpired as cleanup_failure:
                timeout_cleanup = {'returncode': None, 'timed_out': True,
                                   'output': decode(cleanup_failure.stdout) + decode(cleanup_failure.stderr)}
        output_path.write_text(output)
        passed = not timed_out and successful_result(code, output)
        holds = [int(value) for value in re.findall(r'held_ms=(\d+)', output)]
        dispatch = [int(value) for value in re.findall(r'dispatch_ms=(\d+)', output)]
        record['runs'].append({'run': index + 1, 'status': 'passed' if passed else 'failed',
                               'command': command, 'returncode': code, 'timed_out': timed_out,
                               'timeout_cleanup': timeout_cleanup,
                               'seconds': round(time.monotonic() - start, 3),
                               'log': str(output_path),
                               'sha256': hashlib.sha256(output_path.read_bytes()).hexdigest(),
                               'max_injection_call_ms': max(dispatch) if dispatch else None,
                               'max_contact_duration_ms': max(holds) if holds else None})
        record['status'] = 'running' if passed else 'failed'
        summary.write_text(json.dumps(record, indent=2) + '\n')
        print('Native Android pointer run ' + str(index + 1) + ': ' + ('PASS' if passed else 'FAIL'), flush=True)
        if not passed:
            if args.repeat > 1:
                log.write_text('\n'.join(Path(row['log']).read_text() for row in record['runs']))
            print('Inspect ' + str(output_path), file=sys.stderr)
            return 1
    record['status'] = 'passed'
    record['completed_runs'] = len(record['runs'])
    record['total_assertions'] = len(record['runs']) * EXPECTED_CHECKS
    summary.write_text(json.dumps(record, indent=2) + '\n')
    if args.repeat > 1:
        log.write_text('\n'.join(Path(row['log']).read_text() for row in record['runs']))
    return 0


if __name__ == '__main__':
    sys.exit(main())
