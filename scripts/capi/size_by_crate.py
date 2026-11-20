#!/usr/bin/env python3
"""Sizes of a linked program by crate, from the link map of the linker (docs/binary-size.md,
section on the C ABI; #976, #978).

    python3 scripts/capi/size_by_crate.py <map> [--top N] [--json]

Reads the symbol table of an ld64 map (`-Wl,-map,<file>`, Apple) or a GNU ld / lld map
(`-Wl,-Map=<file>`), and adds the size of each symbol in a text or data section to the crate the
symbol belongs to: the first path component of a Rust symbol (legacy `_ZN` or v0 `_R` mangling,
the crate that defines the item, also for generic code instantiated elsewhere), `C` for the
program's own symbols and `system` for symbols without a crate (C and C++ libraries such as Skia
and HarfBuzz are named after their object files where the map has them). Prints MB per crate,
largest first.
"""

import argparse
import json
import re
import sys
from collections import defaultdict


def legacy_crate(name):
    match = re.match(r"_?_ZN(\d+)", name)
    if not match:
        return None
    length = int(match.group(1))
    start = match.end()
    return name[start:start + length]


def v0_crate(name):
    match = re.match(r"_?_R[^C]*C(?:s[0-9a-zA-Z]*_)?(\d+)", name)
    if not match:
        return None
    length = int(match.group(1))
    start = match.end()
    return name[start:start + length].lstrip("_")


def rust_crate(first):
    """The crate of the first path component of a Rust symbol: `<A as B>` and `<A>` name the
    crate of the type (`A`), or of the trait for a blanket implementation (`<T as B>`)."""
    if first.startswith("_$LT$"):
        inner = first[len("_$LT$"):]
        parts = inner.split("$u20$as$u20$")
        head = parts[0]
        if re.fullmatch(r"[A-Z]", head) and len(parts) > 1:
            head = parts[1]
        head = re.sub(r"^(\$RF\$|\$BP\$|mut\$u20\$|dyn\$u20\$|\$u5b\$|\$LP\$)+", "", head)
        return re.split(r"\.\.|\$", head)[0] or "generic"
    return first.split("..")[0]


def object_crate(source):
    """The crate of an object file of the map: the crate of a Rust code generation unit in a
    static library (`<lib>-<hash>.<crate>-<hash>.<crate>.<hash>-cgu.N.rcgu.o`), `native` for
    the C and C++ objects of a static library (Skia, HarfBuzz, the codecs), `C` for an object of
    the program."""
    member = re.search(r"\(([^()]+)\)$", source)
    if not member:
        return "C" if source.endswith(".o") else "linker"
    member = member.group(1)
    match = re.match(r"[a-z0-9_]+-[0-9a-f]{16}\.([a-z0-9_]+)-[0-9a-f]{16}\.", member)
    if match:
        return match.group(1)
    match = re.match(r"([a-z0-9_]+)-[0-9a-f]{16}\.", member)
    if match and member.endswith(".rcgu.o"):
        return match.group(1)
    return "native (Skia, HarfBuzz, codecs)"


def crate_of(symbol, objects, file_index):
    is_rust = bool(re.search(r"17h[0-9a-f]{16}E", symbol)) or symbol.startswith(("_R", "__R"))
    crate = legacy_crate(symbol) or v0_crate(symbol)
    if crate and is_rust:
        return rust_crate(crate)
    return object_crate(objects.get(file_index, ""))


def read_ld64(lines):
    objects, sizes = {}, defaultdict(int)
    section = None
    for line in lines:
        if line.startswith("# Object files:"):
            section = "objects"
            continue
        if line.startswith("# Sections:"):
            section = "sections"
            continue
        if line.startswith("# Symbols:"):
            section = "symbols"
            continue
        if line.startswith("# Dead Stripped Symbols:"):
            break
        if section == "objects":
            match = re.match(r"\[\s*(\d+)\]\s+(.*)$", line.rstrip())
            if match:
                objects[int(match.group(1))] = match.group(2)
        elif section == "symbols":
            match = re.match(r"0x[0-9A-Fa-f]+\s+0x([0-9A-Fa-f]+)\s+\[\s*(\d+)\]\s+(.*)$", line.rstrip())
            if match:
                size = int(match.group(1), 16)
                sizes[crate_of(match.group(3), objects, int(match.group(2)))] += size
    return sizes


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("map")
    parser.add_argument("--top", type=int, default=20)
    parser.add_argument("--json", action="store_true")
    options = parser.parse_args()
    with open(options.map, encoding="utf-8", errors="replace") as file:
        sizes = read_ld64(file)
    ranked = sorted(sizes.items(), key=lambda item: -item[1])
    total = sum(sizes.values())
    if options.json:
        json.dump({"total": total, "crates": dict(ranked)}, sys.stdout, indent=1)
        print()
        return
    print(f"{'crate':<40} {'MB':>8}")
    for crate, size in ranked[:options.top]:
        print(f"{crate:<40} {size / 1e6:>8.2f}")
    print(f"{'total (symbols)':<40} {total / 1e6:>8.2f}")


if __name__ == "__main__":
    main()
