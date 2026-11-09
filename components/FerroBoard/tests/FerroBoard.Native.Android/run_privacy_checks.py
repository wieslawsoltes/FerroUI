#!/usr/bin/env python3
"""Run the native password fixture; fail on the first incomplete/failed run, no retries."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adb', type=Path, required=True)
    parser.add_argument('--serial', required=True)
    parser.add_argument('--log', type=Path, required=True)
    parser.add_argument('--repeat', type=int, default=1)
    parser.add_argument('--package', default='org.ferroboard.privacyconsumer')
    parser.add_argument('--instrumentation', default='org.ferroboard.consumer.PrivacyChecks')
    parser.add_argument('--expected-checks', type=int, default=23)
    args = parser.parse_args()
    log = args.log.resolve()
    summary = log.with_name(log.name + '.summary.json')
    paths = [log.with_name(log.stem + '-run-' + str(i + 1) + log.suffix)
             for i in range(args.repeat)]
    if (args.repeat < 1 or args.expected_checks < 1
            or not all(re.fullmatch(r'[A-Za-z_][A-Za-z_0-9]*(?:\.[A-Za-z_][A-Za-z_0-9]*)+', value)
                       for value in [args.package, args.instrumentation])
            or not log.is_relative_to(Path('/tmp/ferrokeyboard').resolve())
            or any(path.exists() for path in paths + [log, summary])):
        parser.error('positive repeat count and new temporary-directory logs are required')
    log.parent.mkdir(parents=True, exist_ok=True)
    command = [str(args.adb), '-s', args.serial, 'shell', 'am', 'instrument', '-w', '-r',
               args.package + '/' + args.instrumentation]
    record = {'status': 'running', 'requested_runs': args.repeat, 'expected_checks': args.expected_checks,
              'package': args.package, 'instrumentation': args.instrumentation,
              'retry_failed_runs': False, 'runs': []}
    for index, output in enumerate(paths):
        start = time.monotonic()
        try:
            with output.open('w') as stream:
                result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, timeout=240)
            code = result.returncode
        except subprocess.TimeoutExpired:
            subprocess.run([str(args.adb), '-s', args.serial, 'shell', 'am', 'force-stop',
                            args.package], timeout=15, check=True)
            code = None
        text = output.read_text()
        passed = (code == 0 and re.findall(r'^INSTRUMENTATION_RESULT: status=(.*)$', text, re.M) == ['passed']
                  and re.findall(r'^INSTRUMENTATION_RESULT: passed=(\d+)\s*$', text, re.M) == [str(args.expected_checks)]
                  and re.findall(r'^INSTRUMENTATION_CODE: (-?\d+)\s*$', text, re.M) == ['-1']
                  and 'INSTRUMENTATION_RESULT: failure=' not in text
                  and 'INSTRUMENTATION_RESULT: cleanup_failure=' not in text
                  and 'INSTRUMENTATION_FAILED' not in text)
        record['runs'].append({'run': index + 1, 'status': 'passed' if passed else 'failed',
                               'command': command, 'returncode': code,
                               'seconds': round(time.monotonic() - start, 3), 'log': str(output),
                               'sha256': hashlib.sha256(output.read_bytes()).hexdigest()})
        record['status'] = 'running' if passed else 'failed'
        summary.write_text(json.dumps(record, indent=2) + '\n')
        print('Native instrumentation run ' + str(index + 1) + ': ' + ('PASS' if passed else 'FAIL'), flush=True)
        if not passed:
            print(text[-4000:], file=sys.stderr)
            return 1
    record['status'] = 'passed'
    record['assertions'] = len(record['runs']) * args.expected_checks
    summary.write_text(json.dumps(record, indent=2) + '\n')
    log.write_text('\n'.join(Path(row['log']).read_text() for row in record['runs']))
    return 0


if __name__ == '__main__':
    sys.exit(main())
