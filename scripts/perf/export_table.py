#!/usr/bin/env python3
"""What an ELF or PE executable exports, by origin (#1049).

For ELF: the bytes of `.dynsym`, `.dynstr` and `.gnu.hash`, and how many entries of `.dynsym` the
executable defines; the others are imports, which it needs. For PE: the export directory. With the
link map of the same link (lld `-Map`, MSVC `/MAP`), each defined name is attributed to the library
or object that defines it.

    export_table.py <executable> [--map <link map>]
    export_table.py --self-test
"""
import argparse
import collections
import os
import re
import struct
import sys

SHT_DYNSYM = 11
SHT_GNU_HASH = 0x6FFFFFF6


def c_string(data, at):
    end = data.index(0, at)
    return data[at:end].decode("utf-8", "replace")


def elf_table(data):
    """(names the executable defines, imports, bytes by table) of a 64-bit little-endian ELF."""
    if data[:4] != b"\x7fELF" or data[4] != 2 or data[5] != 1:
        raise ValueError("not a 64-bit little-endian ELF")
    section_offset = struct.unpack_from("<Q", data, 0x28)[0]
    entry_size, count = struct.unpack_from("<HH", data, 0x3A)
    sections = [struct.unpack_from("<IIQQQQII", data, section_offset + i * entry_size) for i in range(count)]
    bytes_of = {".dynsym": 0, ".dynstr": 0, ".gnu.hash": 0}
    defined, imports = [], 0
    for _, kind, _, _, offset, size, link, _ in sections:
        if kind == SHT_GNU_HASH:
            bytes_of[".gnu.hash"] = size
        if kind != SHT_DYNSYM:
            continue
        strings = sections[link][4]
        bytes_of[".dynsym"] = size
        bytes_of[".dynstr"] = sections[link][5]
        for at in range(offset + 24, offset + size, 24):
            name, _, _, index = struct.unpack_from("<IBBH", data, at)
            if index == 0:
                imports += 1
            else:
                defined.append(c_string(data, strings + name))
    return sorted(defined), imports, bytes_of


def pe_table(data):
    """(exported names, 0, bytes by table) of a PE32+ image; no names without an export directory."""
    if data[:2] != b"MZ":
        raise ValueError("not a PE image")
    header = struct.unpack_from("<I", data, 0x3C)[0]
    if data[header:header + 4] != b"PE\0\0":
        raise ValueError("not a PE image")
    section_count = struct.unpack_from("<H", data, header + 6)[0]
    optional_size = struct.unpack_from("<H", data, header + 20)[0]
    optional = header + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x20B:
        raise ValueError("not a PE32+ image")
    rva, size = struct.unpack_from("<II", data, optional + 112)
    if not rva:
        return [], 0, {"export directory": 0}
    table = optional + optional_size
    sections = [struct.unpack_from("<IIII", data, table + 40 * i + 8) for i in range(section_count)]

    def offset(address):
        for virtual_size, virtual, raw_size, raw in sections:
            if virtual <= address < virtual + max(virtual_size, raw_size):
                return raw + address - virtual
        raise ValueError(f"the address {address:#x} is in no section")

    directory = offset(rva)
    name_count = struct.unpack_from("<I", data, directory + 24)[0]
    names_at = offset(struct.unpack_from("<I", data, directory + 32)[0]) if name_count else 0
    names = [c_string(data, offset(struct.unpack_from("<I", data, names_at + 4 * i)[0])) for i in range(name_count)]
    return sorted(names), 0, {"export directory": size}


def origin(path):
    library = path.split("(", 1)[0].split(":", 1)[0] if not re.match(r"[A-Za-z]:", path) else path
    base = re.split(r"[\\/]", library)[-1]
    return re.sub(r"-[0-9a-f]{16}(?=\.)", "", base)


def definitions_of_lld(text):
    """Symbol name -> file, from an lld map (`VMA LMA Size Align Out In Symbol`)."""
    defined = {}
    current = None
    for line in text.splitlines():
        match = re.match(r"\s*[0-9a-f]+\s+[0-9a-f]+\s+[0-9a-f]+\s+\d+\s+(\S.*)$", line)
        if not match:
            continue
        field = match.group(1)
        if ":(" in field:
            current = origin(field.split(":(", 1)[0])
        elif current and " " not in field:
            defined.setdefault(field, current)
    return defined


def definitions_of_msvc(text):
    """Symbol name -> library or object, from a MSVC map."""
    defined = {}
    pattern = r"^\s*[0-9a-fA-F]{4}:[0-9a-fA-F]{8}\s+(\S+)\s+[0-9a-fA-F]{16}\s+(?:f\s+)?(?:i\s+)?(\S+)\s*$"
    for name, where in re.findall(pattern, text, re.M):
        defined.setdefault(name, where.split(":", 1)[0])
    return defined


def table_of(data):
    return elf_table(data) if data[:4] == b"\x7fELF" else pe_table(data)


def report(path, map_path=None):
    with open(path, "rb") as file:
        data = file.read()
    names, imports, bytes_of = table_of(data)
    tables = "  ".join(f"{key} {value}" for key, value in bytes_of.items())
    print(f"{os.path.basename(path):28} file {len(data):>10}  {tables}  imports {imports}  defined {len(names)}")
    if names and map_path:
        with open(map_path, errors="replace") as file:
            text = file.read()
        defined = definitions_of_lld(text) if data[:4] == b"\x7fELF" else definitions_of_msvc(text)
        groups = collections.Counter(defined.get(name, "(not in the map)") for name in names)
        for group, count in groups.most_common(14):
            print(f"    {count:6}  {group[:70]}")
    if names:
        print("    first names: " + " ".join(name[:40] for name in names[:6]))


def synthetic_elf():
    strings = b"\0imported\0defined\0"
    symbols = bytes(24) + struct.pack("<IBBHQQ", 1, 0x12, 0, 0, 0, 0) + struct.pack("<IBBHQQ", 10, 0x12, 0, 1, 16, 8)
    header = bytearray(64)
    header[:6] = b"\x7fELF\x02\x01"
    body = symbols + strings
    section_offset = 64 + len(body)
    struct.pack_into("<Q", header, 0x28, section_offset)
    struct.pack_into("<HH", header, 0x3A, 64, 3)
    sections = bytes(64)
    sections += struct.pack("<IIQQQQIIQQ", 0, SHT_DYNSYM, 2, 0, 64, len(symbols), 2, 1, 8, 24)
    sections += struct.pack("<IIQQQQIIQQ", 0, 3, 2, 0, 64 + len(symbols), len(strings), 0, 0, 1, 0)
    return bytes(header) + body + sections


def synthetic_pe(with_exports):
    header = 0x40
    optional = header + 24
    table = optional + 240
    raw = 0x200
    data = bytearray(raw + 0x100)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, header)
    data[header:header + 4] = b"PE\0\0"
    struct.pack_into("<H", data, header + 6, 1)
    struct.pack_into("<H", data, header + 20, 240)
    struct.pack_into("<H", data, optional, 0x20B)
    struct.pack_into("<IIII", data, table + 8, 0x100, 0x1000, 0x100, raw)
    if with_exports:
        struct.pack_into("<II", data, optional + 112, 0x1000, 0x40)
        struct.pack_into("<I", data, raw + 24, 1)
        struct.pack_into("<I", data, raw + 32, 0x1040)
        struct.pack_into("<I", data, raw + 0x40, 0x1050)
        data[raw + 0x50:raw + 0x56] = b"thing\0"
    return bytes(data)


def self_test():
    assert elf_table(synthetic_elf()) == (["defined"], 1, {".dynsym": 72, ".dynstr": 18, ".gnu.hash": 0})
    assert pe_table(synthetic_pe(False)) == ([], 0, {"export directory": 0})
    assert pe_table(synthetic_pe(True)) == (["thing"], 0, {"export directory": 0x40})
    lld = ("     VMA      LMA     Size Align Out     In      Symbol\n"
           "    1000     1000       10    16 .text\n"
           "    1000     1000       10    16         /x/libfoo-0123456789abcdef.rlib(a.o):(.text.one)\n"
           "    1000     1000        0     1                 one\n")
    assert definitions_of_lld(lld) == {"one": "libfoo.rlib"}
    msvc = " 0001:00000010       thing     0000000140001010 f   libbar.rlib:b.o\n"
    assert definitions_of_msvc(msvc) == {"thing": "libbar.rlib"}
    print("self-test passed")


def main():
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("executable")
    parser.add_argument("--map", help="the link map of the same link")
    args = parser.parse_args()
    report(args.executable, args.map)


if __name__ == "__main__":
    main()
