#!/usr/bin/env python3
"""What the export trie of a Mach-O executable holds (#1049).

Prints the bytes of the trie and its symbols. With the ld64 link map of the same link, each symbol
is attributed to the library or object that defines it; without one, to a family read from its
name. The trie is read from the load commands, so a stripped executable can be read.

    export_trie.py <executable> [--map <ld64 map>] [--list]
    export_trie.py --self-test
"""
import argparse
import collections
import os
import re
import struct
import sys

MH_MAGIC_64 = 0xFEEDFACF
LC_SEGMENT_64 = 0x19
LC_DYLD_INFO = 0x22
LC_DYLD_INFO_ONLY = 0x80000022
LC_DYLD_EXPORTS_TRIE = 0x80000033


def uleb(data, at):
    value = shift = 0
    while True:
        byte = data[at]
        at += 1
        value |= (byte & 0x7F) << shift
        shift += 7
        if not byte & 0x80:
            return value, at


def trie_of(data):
    """The bytes of the export trie of a thin 64-bit Mach-O (empty when it has none)."""
    magic, _, _, _, count = struct.unpack_from("<5I", data, 0)
    if magic != MH_MAGIC_64:
        raise ValueError("not a thin 64-bit Mach-O")
    at = 32
    for _ in range(count):
        command, size = struct.unpack_from("<2I", data, at)
        if command == LC_DYLD_EXPORTS_TRIE:
            offset, length = struct.unpack_from("<2I", data, at + 8)
            return data[offset:offset + length]
        if command in (LC_DYLD_INFO, LC_DYLD_INFO_ONLY):
            offset, length = struct.unpack_from("<2I", data, at + 40)
            return data[offset:offset + length]
        at += size
    return b""


def symbols_of(trie):
    """The names in a trie; each node is visited once."""
    if not trie:
        return []
    names = []
    stack = [(0, "")]
    seen = set()
    while stack:
        node, prefix = stack.pop()
        if (node, prefix) in seen:
            continue
        seen.add((node, prefix))
        terminal, at = uleb(trie, node)
        if terminal:
            names.append(prefix)
        at += terminal
        children = trie[at]
        at += 1
        for _ in range(children):
            end = trie.index(0, at)
            edge = trie[at:end].decode("utf-8", "replace")
            child, at = uleb(trie, end + 1)
            stack.append((child, prefix + edge))
    return sorted(names)


def family_by_name(name):
    if name in ("_main", "__mh_execute_header"):
        return "entry point and header"
    if name.startswith(("_OBJC_CLASS_$_", "_OBJC_METACLASS_$_", "_OBJC_EHTYPE_$_", "_OBJC_IVAR_$_")):
        return "Objective-C classes"
    if name.startswith("_hb_") or "hb_" in name[:12]:
        return "HarfBuzz (by name)"
    if name.startswith("_C_"):
        return "skia-bindings shims (by name)"
    if name.startswith("_ferroui_skia"):
        return "ferroui-skia helpers (by name)"
    if name.startswith("__Z"):
        return "other C++"
    return "other"


def definitions_of_map(text):
    """For a ld64 link map: symbol name -> the library or object that defines it."""
    files = {}
    start = text.find("# Object files:")
    end = text.find("# Sections:", start)
    for match in re.finditer(r"^\[\s*(\d+)\]\s+(.+)$", text[start:end], re.M):
        path = match.group(2).strip()
        library = path.split("(", 1)[0]
        base = os.path.basename(library)
        base = re.sub(r"-[0-9a-f]{16}(?=\.)", "", base)
        files[int(match.group(1))] = base
    defined = {}
    start = text.find("# Symbols:")
    for match in re.finditer(r"^0x[0-9A-Fa-f]+\s+0x[0-9A-Fa-f]+\s+\[\s*(\d+)\]\s+(\S.*)$", text[start:], re.M):
        defined.setdefault(match.group(2).strip(), files.get(int(match.group(1)), "?"))
    return defined


def report(path, map_path=None, listing=False):
    with open(path, "rb") as file:
        data = file.read()
    trie = trie_of(data)
    names = symbols_of(trie)
    print(f"{os.path.basename(path):30} file {len(data):>10}  trie {len(trie):>7}  symbols {len(names):>6}")
    if map_path:
        with open(map_path, errors="replace") as file:
            defined = definitions_of_map(file.read())
        groups = collections.Counter(defined.get(name, "(not in the map)") for name in names)
    else:
        groups = collections.Counter(family_by_name(name) for name in names)
    if map_path or listing:
        for group, count in groups.most_common(14):
            print(f"    {count:6}  {group[:70]}")
    if listing:
        for name in names:
            print(f"    {name}")


def self_test():
    # A trie of `_a` and `_ab`: the root, the node of `_a` at 6 and the node of `_ab` at 13.
    trie = bytearray([0, 1]) + b"_a\0" + bytes([6])
    trie += bytes([2, 0, 5, 1]) + b"b\0" + bytes([13])
    trie += bytes([2, 0, 1, 0])
    names = symbols_of(bytes(trie))
    assert names == ["_a", "_ab"], names
    assert family_by_name("_C_SkPaint_new") == "skia-bindings shims (by name)"
    assert family_by_name("_OBJC_CLASS_$_Foo") == "Objective-C classes"
    sample = ("# Object files:\n[  1] /x/libfoo-0123456789abcdef.rlib(a.o)\n[  2] /y/b.o\n# Sections:\n"
              "# Symbols:\n# Address Size File Name\n0x100 0x8 [  1] _one\n0x108 0x8 [  2] _two\n")
    assert definitions_of_map(sample) == {"_one": "libfoo.rlib", "_two": "b.o"}
    print("self-test passed")


def main():
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("executable")
    parser.add_argument("--map", help="the ld64 link map of the same link")
    parser.add_argument("--list", action="store_true", help="print every symbol")
    args = parser.parse_args()
    report(args.executable, args.map, args.list)


if __name__ == "__main__":
    main()
