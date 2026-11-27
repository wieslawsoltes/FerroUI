#!/usr/bin/env python3
"""Interleave compiled binding probes and retain every timing sample."""
import argparse
import json
from pathlib import Path
import random
import statistics
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('directory', type=Path)
parser.add_argument('--arms', nargs='+', default=['stock', 'pr', 'history', 'axes'])
parser.add_argument('--warm', action='store_true', help='measure using 4 million warmup updates and 1 million per direction')
parser.add_argument('--rounds', type=int, default=31)
args = parser.parse_args()
directory = args.directory.resolve()
rng = random.Random(496)
result = {arm: {'binding_ns': [], 'lifecycle_ns': []} for arm in args.arms}
ownership = None
for arm in args.arms:
    row = result[arm]
    output = subprocess.check_output([directory / (arm + '-ownership')], text=True).strip()
    if ownership is None:
        ownership = output
    assert ownership == output, (arm, ownership, output)
    row['ownership_trace'] = output
    row['cohort_bytes'] = (directory / (arm + '-cohort')).stat().st_size
    subprocess.run([directory / (arm + '-cohort')], check=True)
for index in range(-3, args.rounds):
    order = args.arms.copy()
    rng.shuffle(order)
    for arm in order:
        values = result[arm]
        for name in ['binding', 'lifecycle']:
            binary = 'warm' if args.warm and name == 'binding' else name
            fields = subprocess.check_output([directory / (arm + '-' + binary)], text=True).split()
            assert fields[0] == 'true', (arm, fields)
            numbers = [int(value) for value in fields[1:]]
            if name == 'binding':
                assert len(numbers) == 2
                if index >= 0:
                    values['binding_ns'].append(numbers)
            else:
                attach, push, pull, _, _, retarget, retarget_alloc, cycle, cycle_alloc, size = numbers
                checked = {'first_attach_allocations': attach, 'source_50000_allocations': push,
                           'target_50000_allocations': pull, 'retarget_10000_allocations': retarget_alloc,
                           'attach_dispose_10000_allocations': cycle_alloc, 'expression_bytes': size}
                assert checked == values.setdefault('allocations_and_layout', checked), (arm, checked)
                if index >= 0:
                    values['lifecycle_ns'].append([retarget, cycle])
for values in result.values():
    for name in ['binding_ns', 'lifecycle_ns']:
        values[name + '_median'] = [statistics.median(series) for series in zip(*values[name])]
print(json.dumps({'rounds': args.rounds, 'warmups': 3, 'shuffle_seed': 496, 'warmed_propagation': args.warm,
                  'timing_units': 'ns/update; ns/retarget; ns/attach+dispose',
                  'arms': result}, indent=2))
