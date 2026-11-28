#!/usr/bin/env python3
"""Count named MarkupType records and directly referenced member invokers in a Mach-O image."""
import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "themes/full-opcode-spike"))
from catalog_allocation import macho

FAMILIES = {
    "properties": ("MarkupProperty", ["get", "set"]),
    "static_properties": ("MarkupProperty", ["get", "set"]),
    "methods": ("MarkupMethod", ["invoke"]),
    "events": ("MarkupEvent", ["add"]),
    "constructors": ("MarkupConstructor", ["invoke"]),
    "fields": ("MarkupField", ["get"]),
    "enum_members": ("MarkupEnumMember", []),
}


def demangle(symbols):
    result = subprocess.run(["xcrun", "llvm-cxxfilt", "-n"], input="\n".join(s[1:] for s in symbols),
                            text=True, capture_output=True, check=True)
    names = result.stdout.splitlines()
    assert len(names) == len(symbols)
    return names


def census(image_path, linkmap, layout_path, method_details=False):
    image = image_path.read_bytes()
    sections = macho(image)[1]
    data_start, data_length, _ = sections["__DATA_CONST", "__const"]
    code_start, code_length, _ = sections["__TEXT", "__text"]
    data_end = data_start + data_length
    layout = {}
    for line in layout_path.read_text().splitlines():
        type_name, field, offset = line.split()
        layout[type_name, field] = int(offset)
    header_size = layout["MarkupType", "size"]
    live = linkmap.read_text(errors="replace").split("# Dead Stripped Symbols:", 1)[0]
    pattern = r"^(0x\w+)\s+(0x\w+)\s+\[\s*\d+\]\s+(.+)$"
    rows = [(int(a, 16), int(n, 16), sym) for a, n, sym in re.findall(pattern, live, re.M) if int(n, 16)]
    text = {a: (n, sym) for a, n, sym in rows if code_start <= a < code_start + code_length}
    assert re.search(rf"^0x{code_start:X}\s+0x0*{code_length:X}\s+__TEXT\s+__text$", live, re.M | re.I)
    assert re.search(rf"^0x{data_start:X}\s+0x0*{data_length:X}\s+__DATA_CONST\s+__const$", live, re.M | re.I)

    def read(address, length):
        for start, size, offset in sections.values():
            if start <= address and address + length <= start + size:
                position = offset + address - start
                return image[position:position + length]
        raise ValueError(f"Unmapped byte range {address:#x}+{length}")

    def word(address):
        return struct.unpack("<Q", read(address, 8))[0]

    def pair(address):
        return struct.unpack("<QQ", read(address, 16))

    def name(address):
        start, length = pair(address)
        assert 0 < length < 512
        return read(start, length).decode()

    def code_counts(addresses):
        return {"unique_code_addresses": len(addresses), "direct_text_bytes": sum(text[a][0] for a in addresses)}

    owners, member_rows, direct = {}, defaultdict(set), defaultdict(set)
    signatures, by_method, palettes = defaultdict(set), defaultdict(set), defaultdict(set)
    method_records = []
    example_signature = None
    for address, size, symbol in rows:
        if not (data_start <= address < data_end and size == header_size and symbol.endswith("6MARKUP")):
            continue
        owner = name(address + layout["MarkupType", "name"])
        owners[address] = owner
        for family, (ty, dispatch_fields) in FAMILIES.items():
            base, length = pair(address + layout["MarkupType", family])
            stride = layout[ty, "size"]
            if length:
                assert data_start <= base < base + length * stride <= data_end
            for i in range(length):
                row = base + i * stride
                member_rows[family].add(row)
                member = name(row + layout[ty, "name"]) if ty != "MarkupConstructor" else ""
                for field in dispatch_fields:
                    entry = word(row + layout[ty, field])
                    if not entry:
                        continue
                    assert entry in text
                    direct[family].add(entry)
                    if family == "static_properties" and owner in ("Colors", "Brushes"):
                        palettes[owner].add(entry)
                    if family == "methods":
                        by_method[member].add(entry)
                        params, count = pair(row + layout[ty, "parameters"])
                        args = tuple(word(params + 8 * j) for j in range(count))
                        signature = (read(row + layout[ty, "is_static"], 1)[0], word(row + layout[ty, "return_type"]), args)
                        signatures[signature].add((address, row, entry))
                        if method_details:
                            method_records.append((symbol, i, member, signature, entry))
                        if member == "OnClearLog" and owner == "DrawerPageEventsPage":
                            assert example_signature is None
                            example_signature = signature

    # Verify the physical data reported below does not overlap, including headers.
    spans = {(a, a + header_size) for a in owners}
    for family, (ty, _) in FAMILIES.items():
        for a in member_rows[family]:
            size = layout[ty, "size"]
            assert (a, a + size) not in spans
            spans.add((a, a + size))
    ranges = sorted(spans)
    assert all(left[1] <= right[0] for left, right in zip(ranges, ranges[1:]))
    addresses = set.union(*direct.values())
    codes = sorted((a, a + text[a][0]) for a in addresses)
    assert all(left[1] <= right[0] for left, right in zip(codes, codes[1:]))
    assert example_signature is not None
    is_static, result, args = example_signature
    assert not is_static and not result and len(args) == 2
    arg_names = demangle([text[a][1] for a in args])
    assert "AnyValue" in arg_names[0] and "IRoutedEventArgs" in arg_names[1]
    cohort = signatures[example_signature]
    if method_details:
        # Resolve direct arm64 B/BL targets from each registered method to the shared adapter.
        adapter_targets = {a for a, (_, sym) in text.items() if "invoke_routed_handler" in sym}
        raw = {sym for sym, _, _, _, _ in method_records}
        for _, _, _, (_, result, params), _ in method_records:
            raw.update(text[a][1] for a in params)
            if result:
                raw.add(text[result][1])
        labels = sorted(raw)
        decoded = dict(zip(labels, demangle(labels)))
        methods = {}
        for sym, index, member, signature, entry in method_records:
            is_static, result, params = signature
            called = set()
            for n, (instruction,) in enumerate(struct.iter_unpack("<I", read(entry, text[entry][0]))):
                if instruction & 0x7c000000 != 0x14000000:
                    continue
                delta = instruction & 0x3ffffff
                if delta & 0x2000000:
                    delta -= 0x4000000
                target = entry + n * 4 + delta * 4
                if target in adapter_targets:
                    called.add(target)
            key = (decoded[sym], index, member)
            assert key not in methods
            methods[key] = {
                "signature": (is_static, decoded[text[result][1]] if result else None,
                              tuple(decoded[text[a][1]] for a in params)),
                "routed": signature == example_signature,
                "uses_adapter": bool(called),
                "entry_address": entry,
                "entry_bytes": text[entry][0],
            }
        return methods, code_counts(adapter_targets)
    return {
        "image_bytes": len(image), "image_sha256": hashlib.sha256(image).hexdigest(),
        "named_markup_type_headers": len(owners), "header_stride": header_size,
        "header_bytes": len(owners) * header_size,
        "families": {
            family: {"unique_rows": len(member_rows[family]), "row_bytes": len(member_rows[family]) * layout[ty, "size"],
                     **code_counts(direct[family])} for family, (ty, _) in FAMILIES.items()
        },
        "disjoint_headers_and_member_rows_bytes": sum(b - a for a, b in ranges),
        "direct_member_functions_after_deduplication": code_counts(addresses),
        "void_instance_routed_event_methods": {
            "parameter_types": arg_names, "owner_count": len({a for a, _, _ in cohort}),
            "method_count": len({r for _, r, _ in cohort}), **code_counts({e for _, _, e in cohort})
        },
        "all_Add_methods_including_overloads": code_counts(by_method["Add"]),
        "named_color_and_brush_static_getters": code_counts(set.union(*palettes.values())),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path)
    parser.add_argument("linkmap", type=Path)
    parser.add_argument("--layout", type=Path, default=HERE / "layout.txt")
    args = parser.parse_args()
    print(json.dumps(census(args.image, args.linkmap, args.layout), indent=2))
