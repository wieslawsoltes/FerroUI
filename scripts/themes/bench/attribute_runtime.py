#!/usr/bin/env python3
"""Attribute the physical .text of a symbolized Rust ELF after LTO and linker folding.

Build without symbol stripping. Pass --match with the stripped shipping executable
to require identical .text addresses and bytes. Requires GNU nm and readelf.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
import os
import subprocess


def section_map(path):
    listing = subprocess.check_output(["readelf", "-SW", path], text=True)
    pattern = r"^\s*\[\s*\d+\]\s+(\S+)\s+\S+\s+(\w+)\s+(\w+)\s+(\w+)\s"
    return {name: tuple(int(value, 16) for value in fields)
            for name, *fields in re.findall(pattern, listing, re.M)}


def text_identity(path, section):
    address, offset, size = section
    with path.open("rb") as stream:
        stream.seek(offset)
        digest = hashlib.sha256(stream.read(size)).hexdigest()
    return {"address": address, "bytes": size, "sha256": digest}


def family(name):
    # Keep the receiver and trait of the outer <Receiver as Trait>::method.
    # Nested generic arguments describe instantiations of that same definition.
    outer_impl = name.startswith("<")
    depth = 0
    parts = []
    for index, char in enumerate(name):
        if char == "<":
            depth += 1
        elif char == ">" and depth and name[index - 1] != "-":
            depth -= 1
            if outer_impl and depth == 0:
                outer_impl = False
        elif depth <= int(outer_impl):
            parts.append(char)
    result = "".join(parts).replace("::::", "::").rstrip(":")
    return re.sub(r"\{(closure|constant|shim:vtable)#\d+\}", r"{\1}", result)


def category(name):
    if name.startswith(("core::ptr::drop_glue", "core::ptr::drop_in_place", "std::ptr::drop_in_place")):
        return "Compiler-generated destruction"
    if (" as core::ops::function::Fn" in name or "{shim:vtable" in name
            or re.search(r"::call_(once|mut)(::|$)", name)):
        return "Callable entries and trait shims"
    if name.startswith(("alloc::rc::", "alloc::sync::Arc", "alloc::sync::Weak")):
        return "Reference counting and weak handles"
    if name.startswith(("std::thread::local::", "std::sys::thread_local::", "std::sync::once::", "std::sync::once_lock::")):
        return "Thread-local and once initialization"
    if name.startswith(("hashbrown::", "std::collections::hash", "alloc::collections::", "alloc::vec::", "alloc::raw_vec::")):
        return "Collection primitives"
    if ("ferroui_base::type_system::" in name or
            re.search(r"::(?:build_vtable|class_init|__vtable|register_class)(?:::|$)", name)):
        return "Class projection, initialization and virtual dispatch"
    if ("ferroui_base::metadata::" in name or "ferroui_base::data::core::value_type" in name
            or "__markup_" in name or "__markup::" in name or "::register_types" in name):
        return "Reflection and value conversion"
    if (any(fragment in name for fragment in ("ferroui_base::property_store::", "ferroui_base::ferro_property",
            "ferroui_base::direct_property", "ferroui_base::attached_property", "ferroui_base::styled_property"))
            or re.search(r"::\w+_property(?:::|$)", name)
            or ("ferroui_base::ferro_object::" in name and any(part in name for part in
                ("value", "property", "binding", "inheritance", "notifying")))):
        return "Property registration, stores and observers"
    if ("ferroui_base::data::" in name or "ferroui_base::reactive::" in name
            or "ferroui_system::events::" in name):
        return "Bindings, reactive operators and events"
    if name.startswith("ferroui_themes_") and "::compiled_xaml::" in name:
        return "Generated native XAML functions"
    if name.startswith(("ferroui_markup", "ferroui_themes_", "ferroui_xaml_binary", "xamlx::")):
        return "Other XAML runtime, theme and codecs"
    if "{async" in name or " as core::future::future::Future" in name:
        return "Other async state machines"
    if "{closure}" in name:
        return "Other closure bodies"
    if " as core::ops::drop::Drop" in name:
        return "Other explicit Drop implementations"
    if name.startswith(("ferroui_", "winui_theme_probe")):
        return "Other framework and application code"
    return "Other Rust and native code"


def unwind_families(path, ranges):
    """Join FDEs to code by start address. GNU readelf decodes the DWARF pointers."""
    sections = section_map(path)
    groups_at = {}
    for (address, _), (group, _) in ranges.items():
        if address in groups_at and groups_at[address] != group:
            group = "Overlapping categories"
        groups_at[address] = group

    _, frame_offset, frame_size = sections[".eh_frame"]
    with path.open("rb") as stream:
        stream.seek(5)
        byteorder = {b"\x01": "little", b"\x02": "big"}[stream.read(1)]
        stream.seek(frame_offset)
        frame_data = stream.read(frame_size)
        header = b""
        if ".eh_frame_hdr" in sections:
            stream.seek(sections[".eh_frame_hdr"][1])
            header = stream.read(12)

    in_frames = False
    fdes = Counter()
    frame_bytes = Counter()
    record = re.compile(r"^\s*([0-9a-fA-F]+)\s+\w+\s+\w+\s+FDE cie=\w+ pc=([0-9a-fA-F]+)\.\.")
    with subprocess.Popen(["readelf", "--debug-dump=frames", "--wide", path],
                          stdout=subprocess.PIPE, text=True, env={**os.environ, "LC_ALL": "C"}) as proc:
        for line in proc.stdout:
            if line.startswith("Contents of the "):
                in_frames = line.startswith("Contents of the .eh_frame section")
            if in_frames and (match := record.match(line)):
                offset, pc = (int(n, 16) for n in match.groups())
                size = int.from_bytes(frame_data[offset:offset + 4], byteorder)
                length = (12 + int.from_bytes(frame_data[offset + 4:offset + 12], byteorder)
                          if size == 0xffffffff else 4 + size)
                if offset + length > frame_size:
                    raise ValueError("frame record extends past .eh_frame")
                group = groups_at.get(pc, "No sized symbol")
                frame_bytes[group] += length
                fdes[group] += 1
        if proc.wait():
            raise ValueError("readelf could not decode the unwind records")

    if not fdes:
        raise ValueError("no .eh_frame FDE records were decoded")
    output = {"frame_count": sum(fdes.values()), "fde_bytes": dict(frame_bytes.most_common()),
              "fdes_by_category": dict(fdes.most_common()),
              "cie_and_padding_bytes": frame_size - sum(frame_bytes.values())}
    # GNU x86-64 .eh_frame_hdr uses 32-bit data-relative pairs. Other headers
    # still get exact FDE totals without assuming a particular index encoding.
    if (header[:4] == bytes.fromhex("011b033b") and len(header) == 12
            and int.from_bytes(header[8:12], byteorder) == sum(fdes.values())
            and sections[".eh_frame_hdr"][2] == 12 + 8 * sum(fdes.values())):
        output["index_bytes"] = {group: count * 8 for group, count in fdes.items()}
        output["index_header_bytes"] = 12
    return output


def classify(symbols, identity, unwind_image=None):
    start = identity["address"]
    end = start + identity["bytes"]
    ranges = {}
    n_symbols = 0
    for line in symbols:
        fields = line.split(None, 3)
        if len(fields) != 4 or fields[2] not in {"t", "T", "w", "W", "i", "I"}:
            continue
        address, length = int(fields[0], 16), int(fields[1], 16)
        if not length or not start <= address < end:
            continue
        span = (address, min(address + length, end))
        name = family(fields[3].rstrip())
        group = category(name)
        n_symbols += 1
        if span in ranges:
            old_group, old_name = ranges[span]
            group = group if old_group == group else "Aliases from multiple categories"
            name = name if old_name == name else "Multiple function definitions"
        ranges[span] = (group, name)
    if not ranges:
        raise ValueError("no sized .text symbols found; supply the unstripped ELF")

    # Sweep the union of intervals so aliases, nested symbols and alignment
    # cannot inflate totals. Cross-category overlaps have their own bucket.
    boundaries = defaultdict(list)
    for (left, right), label in ranges.items():
        boundaries[left].append((label, 1))
        boundaries[right].append((label, -1))
    boundaries.setdefault(end, [])
    active = Counter()
    groups = Counter()
    families = Counter()
    instances = Counter(ranges.values())
    previous = start
    overlap = 0
    for position, changes in sorted(boundaries.items()):
        width = position - previous
        if width:
            if not active:
                label = ("No sized symbol", "Padding or unlabeled text")
            elif len(active) == 1:
                label = next(iter(active))
            else:
                kinds = {group for group, _ in active}
                label = (next(iter(kinds)) if len(kinds) == 1 else "Overlapping categories", "Overlapping definitions")
            if sum(active.values()) > 1:
                overlap += width
            groups[label[0]] += width
            families[label] += width
        for label, change in changes:
            active[label] += change
            if not active[label]:
                del active[label]
        previous = position
    assert sum(groups.values()) == identity["bytes"]
    result = {
        "text": identity, "symbol_records": n_symbols, "unique_ranges": len(ranges),
        "alias_records": n_symbols - len(ranges), "partial_overlap_bytes": overlap,
        "categories": dict(groups.most_common()),
        "leading_definitions": {
            kind: [{"family": name, "bytes": size, "distinct_ranges": instances[(kind, name)]}
                   for (group, name), size in families.most_common() if group == kind][:12]
            for kind in groups
        },
    }
    if unwind_image is not None:
        result["unwind"] = unwind_families(unwind_image, ranges)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("--unwind", action="store_true", help="also attribute DWARF frame records by function address")
    parser.add_argument("--match", type=Path, help="verify against a separate shipping binary")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    identity = text_identity(args.executable, section_map(args.executable)[".text"])
    if args.match and identity != text_identity(args.match, section_map(args.match)[".text"]):
        parser.error("symbolized and shipping executables have different .text")
    with subprocess.Popen(["nm", "-n", "-S", "-C", "--defined-only", args.executable],
                          stdout=subprocess.PIPE, text=True) as proc:
        result = classify(proc.stdout, identity, args.executable if args.unwind else None)
        if proc.wait():
            raise SystemExit("nm failed")
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
