#!/usr/bin/env python3
"""Verify native glide observations against frozen unchanged-C# classifier results."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def parse_records(text):
    lines = [line for line in text.splitlines() if line]
    starts = [i for i, line in enumerate(lines) if line.startswith('BEGIN\t')]
    if not starts:
        raise ValueError('Missing native glide record start')
    lines = lines[starts[-1]:]
    token = lines[0].split('\t')[1]
    if not lines or lines[-1] != 'END\t' + token:
        raise ValueError('Incomplete native glide record')
    layout = []
    popup = {}
    cases = {name: {'name': name, 'points': [], 'available': [], 'observed_rank': None}
             for name in ['batch', 'unbatched', 'cancel']}
    for line in lines[1:-1]:
        parts = line.split('\t')
        if parts[0] == 'KEY' and len(parts) == 6:
            layout.append({'c': int(parts[1]), 'r': parts[2:]})
        elif parts[0] == 'POPUP' and len(parts) == 3:
            if popup:
                raise ValueError('Duplicate native popup record')
            popup = {'selectedIndex': int(parts[1]), 'choices':
                     [[int(c) for c in word.split(',')] for word in parts[2].split('|')]}
        elif parts[0] == 'POPUPCOMMIT' and len(parts) == 2:
            if not popup or 'committed' in popup:
                raise ValueError('Missing/duplicate native popup commit')
            popup['committed'] = list(map(int, parts[1].split(',')))
        elif parts[0] == 'POINT' and len(parts) == 4:
            cases[parts[1]]['points'].append(parts[2:])
        elif parts[0] == 'RAW' and len(parts) == 4:
            cases[parts[1]]['available'].append(parts[2:])
        elif parts[0] == 'MODE' and len(parts) == 3:
            name = parts[1]
            if name not in ['shift', 'caps', 'geometry', 'logical'] or name in cases:
                raise ValueError('Duplicate/unknown cased glide input')
            cases[name] = {'name': name, 'points': [], 'available': [],
                           'observed_rank': None, 'shift': int(parts[2])}
        elif parts[0] == 'LAYOUT' and len(parts) == 8:
            case = cases[parts[1]]
            key = {'c': int(parts[3]), 'r': parts[4:]}
            if parts[2] == 'before':
                case.setdefault('initialLayout', []).append(key)
            elif parts[2] == 'after' and case.get('layoutUpdates'):
                case['layoutUpdates'][0]['layout'].append(key)
            else:
                raise ValueError('Unexpected logical layout stage')
        elif parts[0] == 'UPDATE' and len(parts) == 3:
            case = cases[parts[1]]
            if 'layoutUpdates' in case:
                raise ValueError('Duplicate logical layout update')
            case['layoutUpdates'] = [{'after': int(parts[2]), 'layout': []}]
        elif parts[0] == 'SAMPLES' and len(parts) == 4:
            case = cases[parts[1]]
            if 'observed_transition_counts' in case:
                raise ValueError('Duplicate logical layout sample counts')
            case['observed_transition_counts'] = list(map(int, parts[2:]))
        elif parts[0] == 'AFTER' and len(parts) == 3:
            case = cases[parts[1]]
            if 'shift_after' in case:
                raise ValueError('Duplicate post-glide state')
            case['shift_after'] = int(parts[2])
        elif parts[0] == 'COMMIT' and len(parts) == 3:
            case = cases[parts[1]]
            if 'committed' in case:
                raise ValueError('Duplicate committed glide word')
            case['committed'] = list(map(int, parts[2].split(',')))
        elif parts[0] == 'RANK' and len(parts) == 3:
            case = cases[parts[1]]
            if case['observed_rank'] is not None:
                raise ValueError('Duplicate native candidate result')
            case['observed_rank'] = [[int(c) for c in word.split(',')]
                                     for word in parts[2].split('|')]
        else:
            raise ValueError('Unexpected native glide record: ' + line)
    if len(layout) != 26 or len({row['c'] for row in layout}) != 26:
        raise ValueError('Incomplete/duplicate native layout')
    if any(len(case['points']) < 10 for case in cases.values()):
        raise ValueError('Missing native gesture samples')
    if any(case['observed_rank'] is None for name, case in cases.items() if name != 'cancel'):
        raise ValueError('Missing committed native rankings')
    for case in cases.values():
        suffix = case['points'][1:]
        if case['available'][-len(suffix):] != suffix:
            raise ValueError('Available native samples were lost/reordered after glide activation')
    if any(len(cases[name]['available']) < 32 for name in ['batch', 'cancel', 'shift', 'caps'] if name in cases):
        raise ValueError('Native MOVE history missing')
    for name in ['shift', 'caps', 'geometry', 'logical']:
        if name in cases and not {'shift_after', 'committed'} <= cases[name].keys():
            raise ValueError('Missing committed cased glide state')
    if 'logical' in cases:
        case = cases['logical']
        updates = case.get('layoutUpdates', [])
        initial = case.get('initialLayout', [])
        if (len(updates) != 1 or not 2 <= updates[0]['after'] < len(case['points'])
                or initial == updates[0]['layout']
                or len(case.get('observed_transition_counts', [])) != 2):
            raise ValueError('Missing/invalid logical layout transition')
        for rows in [initial, updates[0]['layout']]:
            if len(rows) != 26 or {r['c'] for r in rows} != {r['c'] for r in layout}:
                raise ValueError('Incomplete/duplicate logical layout')
        if 'committed' not in popup or not 0 <= popup['selectedIndex'] < len(popup['choices']):
            raise ValueError('Missing native popup commit/selection')
    result = {'schema': 1, 'layout': layout, 'cases': list(cases.values())}
    if popup:
        result['popupKey'] = popup
    return result


def verify_reference(actual, expected, live):
    if expected.get('sourceRevision') != '3694995fabbebc45a801a455880e3d31888367f1':
        raise ValueError('Unexpected C# source revision')
    if actual['layout'] != expected['layout']:
        raise ValueError('Native logical layout changed')
    if len(actual['cases']) != len(expected['cases']):
        raise ValueError('Native/reference case count mismatch')
    if any(case['name'] == 'logical' for case in actual['cases']) and 'popupKey' not in actual:
        raise ValueError('Missing native popup layout behavior')
    if 'popupKey' in actual:
        source = expected.get('popupKey', {})
        if (any(source.get(k) != v for k, v in actual['popupKey'].items())
                or not source.get('changed') or not source.get('retained')
                or source.get('during') != '' or source.get('popupAfter')
                or source.get('pressedAfter') != 0 or source.get('activeTimers') != 0):
            raise ValueError('Native/source popup layout behavior mismatch')
    for a, e in zip(actual['cases'], expected['cases']):
        if a['name'] != e['name'] or a['points'] != e['points']:
            raise ValueError('Native samples differ from ' + ('live oracle input' if live else
                             'frozen input; supply --oracle for native delivery variation'))
        for key in ['initialLayout', 'layoutUpdates']:
            if a.get(key) != e.get(key):
                raise ValueError('Native/source logical layout transition mismatch')
        if 'observed_transition_counts' in a:
            states = e.get('transitionStates', [])
            if (len(states) != 1 or a['observed_transition_counts'] !=
                    [states[0]['beforeCount'], states[0]['afterCount']]
                    or not states[0]['active'] or states[0]['text'] != ''):
                raise ValueError('Native/source retained layout samples mismatch')
        if a['observed_rank'] is not None and a['observed_rank'] != e['ranked']:
            raise ValueError('Native ranked candidates differ from unchanged C#')
        if 'shift' in a and a['shift'] != e['shift']:
            raise ValueError('Native/source starting shift mismatch')
        if 'shift_after' in a and a['shift_after'] != e['shiftAfter']:
            raise ValueError('Native/source final shift mismatch')
        if 'committed' in a and a['committed'] != e['committed']:
            raise ValueError('Native/source committed casing mismatch')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--adb', type=Path, required=True)
    parser.add_argument('--serial', required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--repeat', type=int, default=1)
    parser.add_argument('--expected-checks', type=int, default=38)
    parser.add_argument('--record-only', action='store_true')
    parser.add_argument('--oracle', type=Path, help='built unchanged-C# reference harness DLL')
    parser.add_argument('--dotnet', type=Path, default=Path('/Volumes/SSD/dotnet/dotnet'))
    parser.add_argument('--reference', type=Path,
                        default=Path(__file__).parents[1] / 'fixtures/native-layout-change.json')
    args = parser.parse_args()
    out = args.out.resolve()
    if (args.repeat < 1 or args.record_only and args.repeat != 1
            or not out.is_relative_to(Path('/Volumes/SSD/temp/ferrokeyboard')) or out.exists()):
        parser.error('new SSD output directory and positive repeat count required; record mode is one run')
    expected = None if args.record_only or args.oracle else json.loads(args.reference.read_text())
    out.mkdir(parents=True)
    summary = {'status': 'running', 'source_rankings_verified': False,
               'retry_failed_runs': False, 'runs': []}
    for index in range(args.repeat):
        log = out / f'functional-{index + 1}.log'
        command = [sys.executable, str(Path(__file__).with_name('run_privacy_checks.py')),
                   '--adb', str(args.adb), '--serial', args.serial,
                   '--package', 'org.ferroboard.glideconsumer',
                   '--instrumentation', 'org.ferroboard.consumer.GlideChecks',
                   '--expected-checks', str(args.expected_checks), '--log', str(log)]
        result = subprocess.run(command, check=False)
        if result.returncode != 0:
            summary['status'] = 'failed'
            (out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
            return 1
        native_log = out / f'glide-{index + 1}.log'
        text = subprocess.check_output([str(args.adb), '-s', args.serial, 'logcat', '-d',
                                       '-v', 'raw', '-s', 'FerroBoardGlide:I', '*:S'], text=True, timeout=30)
        native_log.write_text(text)
        record = out / f'glide-{index + 1}.json'
        try:
            actual = parse_records(text)
            record.write_text(json.dumps(actual, indent=2) + '\n')
            reference = expected
            if args.oracle and not args.record_only:
                source = out / f'source-{index + 1}.json'
                with source.open('w') as output:
                    subprocess.run([str(args.dotnet), str(args.oracle), str(record)],
                                   check=True, stdout=output, timeout=120)
                reference = json.loads(source.read_text())
            if not args.record_only:
                verify_reference(actual, reference, args.oracle is not None)
        except (ValueError, KeyError, subprocess.SubprocessError) as failure:
            summary['status'] = 'failed'
            summary['failure'] = str(failure)
            (out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
            print(str(failure), file=sys.stderr)
            return 1
        summary['runs'].append({'run': index + 1, 'functional_checkpoints': args.expected_checks,
                                'native_record': str(record),
                                'sha256': hashlib.sha256(record.read_bytes()).hexdigest()})
        (out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    summary['status'] = 'recorded' if args.record_only else 'passed'
    summary['source_rankings_verified'] = not args.record_only
    summary['reference_method'] = ('record_only' if args.record_only else
                                   'live_unchanged_csharp' if args.oracle else 'frozen_unchanged_csharp')
    summary['functional_checkpoints'] = args.expected_checks * args.repeat
    (out / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print('Native glide ' + summary['status'] + ': ' + str(summary['functional_checkpoints'])
          + ' checkpoints; source ranking verification=' + str(summary['source_rankings_verified']))
    return 0


if __name__ == '__main__':
    sys.exit(main())
