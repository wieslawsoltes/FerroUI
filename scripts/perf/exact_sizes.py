#!/usr/bin/env python3
"""The exact sizes of the executables of the workflow `Performance`, ref against baseline.

    python3 scripts/perf/exact_sizes.py <folder>

The folder holds `exact-sizes-<profile>-<side>.tsv` (`release` or `dist`, `ref` or `baseline`), one
line for each application: its name, the bytes of the stripped file and the bytes of
`__TEXT,__text`. For each profile it prints those bytes side by side with the signed difference,
ref minus baseline. An application a side did not measure shows `-`, and has no difference.
"""
import os
import sys

PROFILES = ('release', 'dist')


def read(path):
    """The sizes of `path` by application, in the order of the file; empty when it is missing."""
    sizes = {}
    if not os.path.isfile(path):
        return sizes
    with open(path, encoding='utf-8') as file:
        for number, line in enumerate(file, 1):
            fields = line.rstrip('\n').split('\t')
            if len(fields) != 3 or not fields[1].isdigit() or not fields[2].isdigit():
                raise SystemExit(f'{path}:{number}: expected an application and two byte counts: {line!r}')
            sizes[fields[0]] = (int(fields[1]), int(fields[2]))
    return sizes


def bytes_text(value):
    return '-' if value is None else f'{value:,}'


def difference(ref, baseline):
    if ref is None or baseline is None:
        return '-'
    change = ref - baseline
    return '0' if change == 0 else f'{change:+,}'


def table(ref, baseline):
    """The lines of the table of the applications of `ref` and `baseline`."""
    names = list(ref) + [name for name in baseline if name not in ref]
    width = max([len('application')] + [len(name) for name in names])
    lines = [
        f"{'application':{width}} {'stripped base':>14} {'stripped ref':>14} {'difference':>11} "
        f"{'__text base':>14} {'__text ref':>14} {'difference':>11}"
    ]
    for name in names:
        old = baseline.get(name, (None, None))
        new = ref.get(name, (None, None))
        lines.append(
            f'{name:{width}} {bytes_text(old[0]):>14} {bytes_text(new[0]):>14} {difference(new[0], old[0]):>11} '
            f'{bytes_text(old[1]):>14} {bytes_text(new[1]):>14} {difference(new[1], old[1]):>11}'
        )
    return lines


def main():
    if len(sys.argv) != 2:
        raise SystemExit(__doc__)
    folder = sys.argv[1]
    printed = False
    for profile in PROFILES:
        ref = read(os.path.join(folder, f'exact-sizes-{profile}-ref.tsv'))
        baseline = read(os.path.join(folder, f'exact-sizes-{profile}-baseline.tsv'))
        if not ref and not baseline:
            continue
        printed = True
        print(f'## macOS, {profile}: exact bytes (difference: ref minus baseline)')
        print()
        print('```')
        print('\n'.join(table(ref, baseline)))
        print('```')
        print()
    if not printed:
        print('No exact sizes were measured.')
    return 0


if __name__ == '__main__':
    sys.exit(main())
