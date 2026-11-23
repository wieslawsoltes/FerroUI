#!/usr/bin/env python3
"""Generic instances of a binary, counted by family (#986).

A family is the code of one generic function or one method of a generic impl: in the legacy Rust
mangling every instance keeps the path of its definition (the generic parameters of an impl as
they are written, `<alloc::vec::Vec<T,A> as core::ops::drop::Drop>::drop`) and differs only in its
hash, so the symbols of one path are the instances of one family. A family with one symbol is not
generic (or has one instance in this binary). A v0 name (`_R...`) spells out the generic arguments,
so its family is the path of its definition, read from the name (`V0Path`); the closures of one
function are separate definitions there, and `drop_in_place` is one family.

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


class V0Path:
    """The path of the definition of a symbol mangled with the v0 scheme (`_R...`, RFC 2603).

    A v0 name spells out the generic arguments and the self type of an instance, so the name of
    an instance is not the name of its family. This walks the grammar and keeps what the instances
    of one definition share: the path, with `{impl#n}` for an impl (the disambiguator tells the
    impls of one module apart), and no generic arguments.
    """

    BASIC_TYPES = set('abcdefhijlmnopstuvxyz')

    def __init__(self, mangled):
        self.text = mangled
        self.at = 0

    def peek(self):
        return self.text[self.at] if self.at < len(self.text) else ''

    def take(self):
        char = self.peek()
        if not char:
            raise ValueError('the name ends early')
        self.at += 1
        return char

    def eat(self, char):
        if self.peek() == char:
            self.at += 1
            return True
        return False

    def base62(self):
        if self.eat('_'):
            return 0
        value = 0
        while not self.eat('_'):
            char = self.take()
            if char.isdigit():
                digit = ord(char) - ord('0')
            elif 'a' <= char <= 'z':
                digit = ord(char) - ord('a') + 10
            elif 'A' <= char <= 'Z':
                digit = ord(char) - ord('A') + 36
            else:
                raise ValueError('not a base-62 number')
            value = value * 62 + digit
        return value + 1

    def disambiguator(self):
        return self.base62() + 1 if self.eat('s') else 0

    def undisambiguated(self):
        self.eat('u')
        start = self.at
        while self.peek().isdigit():
            self.at += 1
        if start == self.at:
            raise ValueError('not an identifier')
        length = int(self.text[start:self.at])
        self.eat('_')
        name = self.text[self.at:self.at + length]
        self.at += length
        return name

    def backref(self, parse):
        target = self.base62()
        back = self.at
        self.at = target
        value = parse()
        self.at = back
        return value

    def path(self):
        tag = self.take()
        if tag == 'C':
            self.disambiguator()
            return self.undisambiguated()
        if tag == 'N':
            namespace = self.take()
            parent = self.path()
            index = self.disambiguator()
            name = self.undisambiguated()
            if namespace == 'C':
                return f'{parent}::{{closure#{index}}}'
            if namespace.isupper():
                return f'{parent}::{{{namespace}#{index}}}{name}'
            return f'{parent}::{name}'
        if tag == 'M':
            index = self.disambiguator()
            parent = self.path()
            self.type()
            return f'{parent}::{{impl#{index}}}'
        if tag == 'X':
            index = self.disambiguator()
            parent = self.path()
            self.type()
            trait = self.path()
            return f'<{parent}::{{impl#{index}}} as {trait}>'
        if tag == 'Y':
            self.type()
            return f'<_ as {self.path()}>'
        if tag == 'I':
            inner = self.path()
            while not self.eat('E'):
                self.generic_arg()
            return inner
        if tag == 'B':
            return self.backref(self.path)
        raise ValueError(f'not a path: {tag}')

    def generic_arg(self):
        if self.eat('L'):
            self.base62()
        elif self.eat('K'):
            self.const()
        else:
            self.type()

    def binder(self):
        if self.eat('G'):
            self.base62()

    def type(self):
        char = self.peek()
        if char in self.BASIC_TYPES:
            self.at += 1
        elif char and char in 'CMXYNI':
            self.path()
        elif self.eat('A'):
            self.type()
            self.const()
        elif self.eat('S') or self.eat('P') or self.eat('O'):
            self.type()
        elif self.eat('T'):
            while not self.eat('E'):
                self.type()
        elif self.eat('R') or self.eat('Q'):
            if self.eat('L'):
                self.base62()
            self.type()
        elif self.eat('F'):
            self.binder()
            self.eat('U')
            if self.eat('K') and not self.eat('C'):
                self.undisambiguated()
            while not self.eat('E'):
                self.type()
            self.type()
        elif self.eat('D'):
            self.binder()
            while not self.eat('E'):
                self.path()
                while self.eat('p'):
                    self.undisambiguated()
                    self.type()
            if self.eat('L'):
                self.base62()
        elif self.eat('B'):
            self.backref(self.type)
        else:
            raise ValueError(f'not a type: {char}')

    def const(self):
        if self.eat('p'):
            return
        if self.eat('B'):
            self.backref(self.const)
        elif self.peek() and self.peek() in 'RQ':
            self.at += 1
            self.const()
        elif self.peek() and self.peek() in 'AT':
            self.at += 1
            while not self.eat('E'):
                self.const()
        elif self.peek() in self.BASIC_TYPES:
            self.at += 1
            self.eat('n')
            while not self.eat('_'):
                self.take()
        else:
            raise ValueError('a constant this reader does not know')


def family_of_v0(symbol):
    """The path of the definition of a v0 symbol, or None if it cannot be read."""
    name = symbol.lstrip('_')
    if not name.startswith('R'):
        return None
    try:
        return V0Path(name[1:]).path()
    except (ValueError, IndexError, RecursionError):
        return None


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
        if not crate:
            return None
        family = family_of_v0(symbol)
        # A name that cannot be read counts as a family of its own, never as part of another.
        return (family or symbol, crate)
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
    unread = sum(1 for name, _ in symbols if name.lstrip('_').startswith('R') and family_of_v0(name) is None)
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
          f'{generic / mb:8.2f} MB ({100 * generic / max(rust, 1):.1f} % of Rust); not Rust {other / mb:8.2f} MB; '
          f'{len(symbols)} functions, {sum(v[0] for v in families.values())} of them Rust in {len(families)} families, '
          f'{unread} v0 names not read')
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
