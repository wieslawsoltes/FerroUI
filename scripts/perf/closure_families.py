#!/usr/bin/env python3
"""The generic functions of an executable that have a copy per closure type (#998).

Reads the symbols of an unstripped executable mangled with v0. An instance is closure-generic when a
generic argument or the self type of its impl names a closure. Instances are grouped by the API they
belong to: the path of the definition, with the closures inside it counted with it. `shareable` is
the most that sharing one body could save: the bytes of the family less its largest instance. Code
of the closures inlined into the copies has to stay somewhere, so the real saving is smaller.

    closure_families.py <executable> [--top N] [--crates a,b] [--json FILE]
    closure_families.py --self-test
"""
import argparse
import collections
import json
import os
import re
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
from generic_instances import V0Path, symbols_of_binary  # noqa: E402

MIB = 1 << 20


class ClosureArguments(V0Path):
    """The path of a v0 name, noting whether its generic arguments or self type name a closure."""

    def __init__(self, mangled):
        super().__init__(mangled)
        self.depth = 0
        self.names_closure = False

    def path(self):
        if self.depth and self.text[self.at:self.at + 2] == "NC":
            self.names_closure = True
        return super().path()

    def type(self):
        self.depth += 1
        try:
            super().type()
        finally:
            self.depth -= 1

    def generic_arg(self):
        self.depth += 1
        try:
            super().generic_arg()
        finally:
            self.depth -= 1


def read(symbol):
    """(path of the definition, whether it is generic over a closure), or None for another name."""
    name = symbol.lstrip("_")
    if not name.startswith("R"):
        return None
    reader = ClosureArguments(name[1:])
    try:
        return reader.path(), reader.names_closure
    except (ValueError, IndexError, RecursionError):
        return None


def api_of(path):
    """The function a path belongs to: closures and shims inside it count with it."""
    return re.sub(r"(::\{(?:closure|[A-Z])#\d+\}[A-Za-z_]*)+$", "", path)


def crate_of(path):
    match = re.match(r"<?([A-Za-z_][A-Za-z0-9_]*)", path)
    if match and match.group(1) == "_":
        match = re.search(r" as <?([A-Za-z_][A-Za-z0-9_]*)", path)
    return match.group(1) if match else "?"


def families(symbols):
    """API -> [instances, bytes, largest], for the closure-generic instances; and the totals."""
    found = collections.defaultdict(lambda: [0, 0, 0])
    rust = unread = 0
    for name, size in symbols:
        if not name.lstrip("_").startswith("R"):
            continue
        result = read(name)
        if result is None:
            unread += 1
            continue
        rust += size
        path, closure = result
        if closure:
            entry = found[api_of(path)]
            entry[0] += 1
            entry[1] += size
            entry[2] = max(entry[2], size)
    return found, rust, unread


def report(path, top, crates, json_path):
    found, rust, unread = families(symbols_of_binary(path))
    total = sum(v[1] for v in found.values())
    print(f"{os.path.basename(path)}: Rust {rust / MIB:.2f} MiB; closure-generic {total / MIB:.2f} MiB "
          f"({100 * total / max(rust, 1):.1f} %) in {sum(v[0] for v in found.values())} instances of {len(found)} "
          f"functions; {unread} v0 names not read")
    by_crate = collections.defaultdict(lambda: [0, 0, 0])
    for api, (count, size, largest) in found.items():
        entry = by_crate[crate_of(api)]
        entry[0] += count
        entry[1] += size
        entry[2] += size - largest
    print("  crate                              instances       KB  shareable KB")
    for crate, (count, size, shareable) in sorted(by_crate.items(), key=lambda item: -item[1][1])[:14]:
        print(f"  {crate[:34]:34} {count:9} {size / 1024:8.1f} {shareable / 1024:13.1f}")
    print("  instances       KB  largest KB  shareable KB  function")
    rows = [(api, v) for api, v in found.items() if not crates or crate_of(api) in crates]
    for api, (count, size, largest) in sorted(rows, key=lambda item: -(item[1][1] - item[1][2]))[:top]:
        print(f"  {count:9} {size / 1024:8.1f} {largest / 1024:11.1f} {(size - largest) / 1024:13.1f}  {api[:110]}")
    if json_path:
        out = [{"function": api, "crate": crate_of(api), "instances": v[0], "bytes": v[1], "largest": v[2]}
               for api, v in sorted(found.items(), key=lambda item: -item[1][1])]
        with open(json_path, "w", encoding="utf-8") as file:
            json.dump(out, file, indent=1)


def self_test():
    # `a::g::<a::main::{closure#0}>`, generic over a closure.
    generic = "_RINvCs1_1a1gNCNvB2_4main0EB2_"
    assert read(generic) == ("a::g", True), read(generic)
    # `a::g::<u8>`, generic over something else.
    plain = "_RINvCs1_1a1ghEB2_"
    assert read(plain) == ("a::g", False), read(plain)
    # A closure of `a::main`, not generic over one.
    closure = "_RNCNvCs1_1a4main0B3_"
    assert read(closure) == ("a::main::{closure#0}", False), read(closure)
    assert api_of("a::g::{closure#0}::{closure#1}") == "a::g"
    assert crate_of("<a::{impl#0} as b::T>::f") == "a"
    assert crate_of("<_ as b::T>::f") == "b"
    found, rust, unread = families([(generic, 10), (generic, 30), (plain, 5), ("_notrust", 7)])
    assert found["a::g"] == [2, 40, 30] and rust == 45 and unread == 0
    print("self-test passed")


def main():
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("executable")
    parser.add_argument("--top", type=int, default=40)
    parser.add_argument("--crates", help="only these crates in the list of functions, comma separated")
    parser.add_argument("--json", help="write every function to this file")
    args = parser.parse_args()
    report(args.executable, args.top, set(args.crates.split(",")) if args.crates else None, args.json)


if __name__ == "__main__":
    main()
