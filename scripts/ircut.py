#!/usr/bin/env python3
"""What an executable loses when references are cut in a whole-program LLVM IR module: how many
bytes a root keeps, before anything is changed (docs/binary-size.md, "Finding what roots code").

    python3 scripts/ircut.py <module.ll> <binary> <regex> [<regex> ...]
    python3 scripts/ircut.py <module.ll> <binary> --classes
    python3 scripts/ircut.py <module.ll> <binary> --referrers <regex> [<regex> ...]
    python3 scripts/ircut.py <module.ll> <binary> --edges <from> <to> [<from> <to> ...]

The module and the binary come from one fat-LTO build that also emits IR (`scripts/irpaths.py`
says how): `<target>/release/deps/<name>-<hash>.ll` and the file of the same name without the
extension. The graph and its roots are those of `irpaths.py`. The bytes of a symbol are the
distance to the next symbol of the binary, so a function the optimizer inlined everywhere counts
nothing; the totals are a lower bound of what a change saves.

  <regex>       the symbols reachable only through a symbol that matches: what is lost when
                everything that matches goes, with the largest of them
  --classes     the same for the `TYPE` of each class alone, the classes listed by what each keeps
  --referrers   the references from outside the symbols that match to a symbol that matches:
                where a family of symbols is entered
  --edges       what is lost when the references from a symbol matching <from> to a symbol
                matching <to> go and everything else stays: the effect of one change in the source
"""
import bisect
import collections
import importlib.util
import os
import re
import subprocess
import sys

# The `TYPE` static of a class, in a mangled name.
CLASS_TYPE = r'\$LT\$(.*)\$u20\$as\$u20\$ferroui_base\.\.type_system\.\.StaticType\$GT\$4TYPE4TYPE'


def load():
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'irpaths.py')
    spec = importlib.util.spec_from_file_location('irpaths', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sizes(binary):
    listing = subprocess.run(['nm', '-n', '--defined-only', binary], capture_output=True, text=True)
    lines = listing.stdout.splitlines()
    symbols = []
    for line in lines:
        parts = line.split(' ', 2)
        if len(parts) == 3:
            symbols.append((int(parts[0], 16), parts[2]))
    addresses = sorted({address for address, _ in symbols})
    result = {}
    for address, name in symbols:
        index = bisect.bisect_right(addresses, address)
        size = addresses[index] - address if index < len(addresses) else 0
        if size > 4_000_000:
            size = 0
        key = name[1:] if name.startswith('_') else name
        result[key] = max(result.get(key, 0), size)
    return result


def reach(edges, roots, cut):
    seen = set(root for root in roots if root not in cut)
    queue = collections.deque(seen)
    while queue:
        node = queue.popleft()
        for named in edges.get(node, ()):
            if named not in seen and named not in cut:
                seen.add(named)
                queue.append(named)
    return seen


def main(argv):
    if len(argv) < 4:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    irpaths = load()
    argv = [argv[0], None] + argv[1:]
    edges, roots = irpaths.graph(argv[2])
    size = sizes(argv[3])
    everything = reach(edges, roots, set())
    total = sum(size.get(name, 0) for name in everything)
    print(f'{len(everything)} reachable symbols, {total} bytes of them in the binary')
    if argv[4] == '--classes':
        pattern = re.compile(CLASS_TYPE)
        rows = []
        for name in sorted(everything):
            match = pattern.search(name)
            if not match:
                continue
            kept = reach(edges, roots, {name})
            lost = everything - kept
            rows.append((sum(size.get(n, 0) for n in lost), len(lost), match.group(1).replace('..', '::')))
        rows.sort(reverse=True)
        for lost_bytes, count, name in rows:
            print(f'{lost_bytes:9d} {count:6d} {name}')
        return 0
    if argv[4] == '--edges':
        # Pairs of patterns: the edges from a symbol matching the first to one matching the second go.
        pairs = [(re.compile(a), re.compile(b)) for a, b in zip(argv[5::2], argv[6::2])]
        removed = 0
        for source in list(edges):
            for a, b in pairs:
                if a.search(source):
                    gone = {named for named in edges[source] if b.search(named)}
                    removed += len(gone)
                    edges[source] = edges[source] - gone
        kept = reach(edges, roots, set())
        lost = everything - kept
        lost_bytes = sum(size.get(name, 0) for name in lost)
        print(f'{removed} edges removed: loses {len(lost)} symbols, {lost_bytes} bytes')
        classes = sorted(n for n in lost if 'StaticType$GT$4TYPE4TYPE' in n)
        names = (re.sub(r'.*\.\.([A-Za-z0-9_]+)\$u20\$as.*', r'\1', name) for name in classes)
        print('classes gone:', ' '.join(names))
        biggest = sorted(lost, key=lambda name: -size.get(name, 0))[:12]
        for name, shown in zip(biggest, irpaths.demangle(biggest)):
            print(f'  {size.get(name, 0):8d} {irpaths.short(shown)[:150]}')
        return 0
    if argv[4] == '--referrers':
        for text in argv[5:]:
            pattern = re.compile(text)
            print('=' * 100)
            print(text)
            pairs = sorted((source, named) for source in everything if not pattern.search(source)
                           for named in edges.get(source, ()) if pattern.search(named))
            names = [n for pair in pairs for n in pair]
            shown = irpaths.demangle(names)
            for index in range(len(pairs)):
                source, named = irpaths.short(shown[2 * index]), irpaths.short(shown[2 * index + 1])
                print(f'  {source[:120]}\n      -> {named[:120]}')
        return 0
    for text in argv[4:]:
        pattern = re.compile(text)
        cut = {name for name in everything if pattern.search(name)}
        kept = reach(edges, roots, cut)
        lost = everything - kept
        lost_bytes = sum(size.get(name, 0) for name in lost)
        print('=' * 100)
        print(f'{text}: cuts {len(cut)} symbols, loses {len(lost)} symbols, {lost_bytes} bytes')
        biggest = sorted(lost, key=lambda name: -size.get(name, 0))[:25]
        for name, shown in zip(biggest, irpaths.demangle(biggest)):
            print(f'  {size.get(name, 0):8d} {irpaths.short(shown)[:150]}')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
