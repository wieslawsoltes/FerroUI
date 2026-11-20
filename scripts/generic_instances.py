#!/usr/bin/env python3
"""Generic instances of a binary, counted by family (#986).

A family is the code of one generic function or one method of a generic impl: in the legacy Rust
mangling every instance keeps the path of its definition (the generic parameters of an impl as
they are written, `<alloc::vec::Vec<T,A> as core::ops::drop::Drop>::drop`) and differs only in its
hash, so the symbols of one path are the instances of one family. A family with one symbol is not
generic (or has one instance in this binary).

Input: an ld64 link map (`-Wl,-map,<file>`), a GNU/LLVM link map (`-Wl,-Map=<file>`), or an
unstripped Mach-O or ELF binary (sizes from the distance to the next symbol, as
`scripts/symsize.py`). Output: the families with the most bytes, and per crate the bytes of its
code that are in families of two or more instances.

    generic_instances.py <map or binary> [--top N] [--min-instances K] [--crate NAME] [--json FILE]
"""
import argparse
import collections
import json
import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from symsize import crate_of, crate_of_v0, decode_legacy  # noqa: E402


def symbols_of_ld64_map(text):
    """(name, size) of every symbol of an ld64 map (the `# Symbols:` table)."""
    start = text.find('# Symbols:')
    if start < 0:
        return None
    out = []
    for line in text[start:].split('\n')[2:]:
        m = re.match(r'0x[0-9A-Fa-f]+\s+0x([0-9A-Fa-f]+)\s+\[\s*\d+\]\s+(\S.*)$', line)
        if m:
            out.append((m.group(2).strip(), int(m.group(1), 16)))
        elif line.startswith('# '):
            break
    return out


def symbols_of_gnu_map(text):
    """(name, size) from a GNU ld or lld map: the input sections of .text (`-ffunction-sections`
    gives one per function: `.text._ZN...` with its size)."""
    out = []
    for m in re.finditer(r'^\s*\.text\.(\S+)\s+0x[0-9a-fA-F]+\s+0x([0-9a-fA-F]+)', text, re.M):
        out.append((m.group(1), int(m.group(2), 16)))
    for m in re.finditer(r'^\s*[0-9a-fA-F]+\s+[0-9a-fA-F]+\s+([0-9a-fA-F]+)\s+\d+\s+\.text\.(\S+)$', text, re.M):
        out.append((m.group(2), int(m.group(1), 16)))
    return out or None


def symbols_of_binary(path):
    """(name, size) of the text symbols of an unstripped binary, sizes from the next address."""
    listing = subprocess.run(['nm', '-n', path], capture_output=True, text=True, check=True).stdout
    entries = []
    for line in listing.split('\n'):
        parts = line.split(' ', 2)
        if len(parts) == 3 and parts[1] in ('t', 'T'):
            entries.append((int(parts[0], 16), parts[2]))
    out = []
    for (address, name), (following, _) in zip(entries, entries[1:]):
        if following > address:
            out.append((name, following - address))
    return out


def family_of(symbol):
    """The family name of a symbol and its crate, or None for a symbol that is not Rust."""
    decoded = decode_legacy(symbol)
    if decoded is not None:
        crate = crate_of(decoded)
        # `<T as Trait>::f`, a blanket impl: the crate of the trait.
        if re.fullmatch(r'[A-Z][A-Za-z0-9]?', crate) and ' as ' in decoded:
            crate = crate_of(decoded.split(' as ', 1)[1])
        return decoded, crate
    if symbol.lstrip('_').startswith('R'):
        crate = crate_of_v0(symbol)
        return (symbol, crate) if crate else None
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('input', help='a link map or an unstripped binary')
    parser.add_argument('--top', type=int, default=40, help='families listed (default 40)')
    parser.add_argument('--min-instances', type=int, default=2, help='instances a family needs to be listed (default 2)')
    parser.add_argument('--crate', help='only families of this crate')
    parser.add_argument('--json', help='write every family to this file')
    args = parser.parse_args()

    with open(args.input, 'rb') as file:
        head = file.read(4)
    if head[:2] in (b'\xcf\xfa', b'\xca\xfe') or head == b'\x7fELF':
        symbols = symbols_of_binary(args.input)
    else:
        with open(args.input, encoding='utf-8', errors='replace') as file:
            text = file.read()
        symbols = symbols_of_ld64_map(text) or symbols_of_gnu_map(text)
        if symbols is None:
            sys.exit(f'{args.input}: neither a link map nor a binary this script reads')

    families = collections.defaultdict(lambda: [0, 0, 0])  # instances, bytes, largest
    crate_of_family = {}
    total = 0
    other = 0
    for name, size in symbols:
        total += size
        found = family_of(name)
        if found is None:
            other += size
            continue
        family, crate = found
        entry = families[family]
        entry[0] += 1
        entry[1] += size
        entry[2] = max(entry[2], size)
        crate_of_family[family] = crate

    by_crate = collections.defaultdict(lambda: [0, 0])  # all bytes, bytes in generic families
    for family, (count, size, _) in families.items():
        crate = crate_of_family[family]
        by_crate[crate][0] += size
        if count >= 2:
            by_crate[crate][1] += size

    generic = sum(size for count, size, _ in families.values() if count >= 2)
    rust = sum(size for _, size, _ in families.values())
    mb = 1 << 20
    print(f'{args.input}')
    print(f'  symbols {total / mb:8.2f} MB; Rust {rust / mb:8.2f} MB; in families of two or more instances '
          f'{generic / mb:8.2f} MB ({100 * generic / max(rust, 1):.1f} % of Rust); not Rust {other / mb:8.2f} MB')
    print()
    print('  crate                                      Rust MB  generic MB  generic %')
    for crate, (size, gen) in sorted(by_crate.items(), key=lambda item: -item[1][1])[:25]:
        print(f'  {crate[:40]:<40} {size / mb:9.2f} {gen / mb:11.2f} {100 * gen / max(size, 1):9.1f}')
    print()
    print('  instances      KB   largest KB  family')
    listed = [(f, v) for f, v in families.items() if v[0] >= args.min_instances
              and (args.crate is None or crate_of_family[f] == args.crate)]
    for family, (count, size, largest) in sorted(listed, key=lambda item: -item[1][1])[:args.top]:
        print(f'  {count:9} {size / 1024:7.1f} {largest / 1024:11.1f}  {family[:150]}')

    if args.json:
        rows = [{'family': f, 'crate': crate_of_family[f], 'instances': v[0], 'bytes': v[1], 'largest': v[2]}
                for f, v in sorted(families.items(), key=lambda item: -item[1][1])]
        with open(args.json, 'w', encoding='utf-8') as file:
            json.dump(rows, file, indent=1)


if __name__ == '__main__':
    main()
