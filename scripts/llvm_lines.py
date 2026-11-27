#!/usr/bin/env python3
"""LLVM IR lines by generic family, from the `.ll` files rustc writes with `--emit=llvm-ir` (#1001).

The count of `scripts/generic_instances.py` is made on the final binary, after LLVM and the
linker; this one is made on the IR that rustc hands to LLVM, which is also what each crate costs
to compile. A family is the code of one generic function, as in `generic_instances.py`: the
legacy-mangled path of an instance without its hash. Each `define` is one function; its size is
the number of lines of its body.

    llvm_lines.py <file.ll or folder> [...] [--top N] [--min-instances K] [--crate NAME] [--json FILE]

A folder is searched for `.ll` files. Several files (the crates of one build) are counted
together: an instance that more than one crate makes (generic code is instantiated by the crate
that uses it) is counted once per crate, which is the count of copies before link-time
optimisation merges them.

To write the IR of one crate without touching the fingerprints of a shared target folder:

    CARGO_TARGET_DIR=<a folder of its own> cargo rustc --release -p ferroui-base --lib -- --emit=llvm-ir
"""
import argparse
import collections
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from symsize import crate_of, crate_of_v0, decode_legacy  # noqa: E402

DEFINE = re.compile(r'^define [^@]*@("?)([^"(\s]+)\1\(')


def functions_of(path):
    """(symbol, lines) of every function defined in one `.ll` file."""
    out = []
    name, lines = None, 0
    with open(path, errors='replace') as f:
        for line in f:
            if name is None:
                m = DEFINE.match(line)
                if m:
                    name, lines = m.group(2), 0
            elif line.startswith('}'):
                out.append((name, lines))
                name = None
            else:
                lines += 1
    return out


def family_of(symbol):
    """The family of a symbol and its crate: the decoded legacy path without its hash; a v0 or
    unmangled name is its own family."""
    decoded = decode_legacy('_' + symbol if symbol.startswith('_ZN') else symbol)
    if decoded is not None:
        return decoded, crate_of(decoded)
    return symbol, crate_of_v0(symbol) or '?'


def ll_files(paths):
    for path in paths:
        if os.path.isdir(path):
            for root, _, files in os.walk(path):
                for name in sorted(files):
                    if name.endswith('.ll'):
                        yield os.path.join(root, name)
        else:
            yield path


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('paths', nargs='+')
    parser.add_argument('--top', type=int, default=40)
    parser.add_argument('--min-instances', type=int, default=1)
    parser.add_argument('--crate', help='only the families of this crate')
    parser.add_argument('--json', help='write every family to this file')
    args = parser.parse_args()

    families = collections.defaultdict(lambda: [0, 0, set()])
    total = 0
    files = list(ll_files(args.paths))
    for path in files:
        for symbol, lines in functions_of(path):
            family, crate = family_of(symbol)
            entry = families[family]
            entry[0] += 1
            entry[1] += lines
            entry[2].add(crate)
            total += lines

    rows = [(lines, count, family, sorted(crates)[0]) for family, (count, lines, crates) in families.items()
            if count >= args.min_instances and (args.crate is None or args.crate in crates)]
    rows.sort(reverse=True)
    generic = sum(lines for lines, count, _, _ in rows if count > 1)
    print(f'{len(files)} files; {total} lines of IR in function bodies; '
          f'{generic} in families of two or more instances ({100 * generic / max(total, 1):.1f} %)')
    print()
    print(f'{"lines":>10} {"instances":>9}  family')
    for lines, count, family, _ in rows[:args.top]:
        print(f'{lines:>10} {count:>9}  {family}')
    if args.json:
        with open(args.json, 'w') as f:
            json.dump([{'family': family, 'crate': crate, 'instances': count, 'lines': lines}
                       for lines, count, family, crate in rows], f)


if __name__ == '__main__':
    main()
