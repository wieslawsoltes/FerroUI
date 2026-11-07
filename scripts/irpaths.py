#!/usr/bin/env python3
"""The shortest reference chains in a whole-program LLVM IR module, from `main` to the symbols
that match patterns: why the linker keeps a function or a static (docs/binary-size.md, "Finding
what roots code").

    python3 scripts/irpaths.py <module.ll> <regex> [<regex> ...]

The module comes from a fat-LTO build that also emits IR, for example:

    cargo rustc --release -p hello-world --bin hello-world -- -C lto=fat --emit=llvm-ir,link

and is `<target>/release/deps/hello_world-<hash>.ll`. The graph has an edge from each function
or global to every global it names (in a function body, on a `define` line, or in a global's
initialiser); the roots are `main` and the contents of `llvm.global_ctors`, `llvm.used` and
`llvm.compiler.used`. A breadth-first search gives, for each reachable symbol that matches a
pattern, one shortest chain from a root, demangled with `rustfilt` or `c++filt` when installed.

ld64 (`-why_live`) and lld (`--why-live`) answer the same question for one symbol at link time,
but Apple's current linker ignores `-why_live`; the IR graph also shows chains through
anonymous tables the linker reports only by address.
"""

import collections
import re
import subprocess
import sys

IDENTIFIER = re.compile(r'@("(?:[^"\\]|\\.)*"|[-a-zA-Z$._][-a-zA-Z$._0-9]*)')
ROOT_LISTS = ('llvm.global_ctors', 'llvm.used', 'llvm.compiler.used')


def graph(path):
    """The edges of the module at `path` and its roots."""
    edges = collections.defaultdict(set)
    roots = {'main'}
    current = None
    with open(path, errors='replace') as module:
        for line in module:
            if current is not None:
                if line.startswith('}'):
                    current = None
                    continue
                for match in IDENTIFIER.finditer(line):
                    edges[current].add(match.group(1).strip('"'))
                continue
            if line.startswith('define '):
                match = IDENTIFIER.search(line)
                if match:
                    current = match.group(1).strip('"')
                    # The personality and prefix data on the `define` line.
                    for named in IDENTIFIER.finditer(line[match.end():]):
                        edges[current].add(named.group(1).strip('"'))
                continue
            if line.startswith('@'):
                match = IDENTIFIER.match(line)
                if not match:
                    continue
                name = match.group(1).strip('"')
                for named in IDENTIFIER.finditer(line[match.end():]):
                    edges[name].add(named.group(1).strip('"'))
                if name in ROOT_LISTS:
                    roots |= edges[name]
    return edges, roots


def demangle(names):
    for tool, prepare in (('rustfilt', lambda n: n), ('c++filt', lambda n: '_' + n if n.startswith('_ZN') else n)):
        try:
            out = subprocess.run([tool], input='\n'.join(prepare(n) for n in names),
                                 capture_output=True, text=True).stdout.split('\n')
        except FileNotFoundError:
            continue
        if len(out) >= len(names):
            return out[:len(names)]
    return names


def short(name):
    name = re.sub(r'::h[0-9a-f]{16}$', '', name)
    return name if len(name) < 180 else name[:177] + '...'


def main(argv):
    if len(argv) < 3:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    patterns = [re.compile(pattern) for pattern in argv[2:]]
    edges, roots = graph(argv[1])
    previous = {root: None for root in roots}
    queue = collections.deque(roots)
    while queue:
        node = queue.popleft()
        for named in edges.get(node, ()):
            if named not in previous:
                previous[named] = node
                queue.append(named)
    targets = sorted(name for name in previous if any(p.search(name) for p in patterns))
    print(f'{len(edges)} nodes with edges, {len(previous)} reachable, {len(targets)} targets')
    for target in targets:
        chain = []
        node = target
        while node is not None:
            chain.append(node)
            node = previous[node]
        chain.reverse()
        print('=' * 100)
        for name in demangle(chain):
            print('  ' + short(name))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
