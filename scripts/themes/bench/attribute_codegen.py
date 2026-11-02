#!/usr/bin/env python3
"""Measure post-LTO code shapes, exception spans and relative pointer slots in an x86-64 Rust ELF.

Pass the stripped release ELF, a symbolized link of the same inputs, and the final FatLTO
relocatable object saved at link time. For LLD, append -Clink-arg=-Wl,--reproduce=link.tar
to the final cargo rustc command. Extract the tar, remove --strip-all from its response file
and relink those same inputs to retain symbol names. GNU nm and readelf are required. This reader accepts
little-endian ELF64, x86-64 RELA/RELR and the zR/zPLR CFI encodings emitted by rustc.
"""
import argparse
from bisect import bisect_right
from collections import Counter, defaultdict
import hashlib
import json
import mmap
from pathlib import Path
import struct
import subprocess

from attribute_runtime import category, classify, family, section_map, text_identity

SHDR = struct.Struct("<IIQQQQIIQQ")
SYM = struct.Struct("<IBBHQQ")
RELA = struct.Struct("<QQq")
U32 = struct.Struct("<I")
S32 = struct.Struct("<i")
U64 = struct.Struct("<Q")
# ELF x86-64 absolute, PC-relative, GOT, PLT and local-exec TLS word sizes.
WIDTH = {1: 8, 2: 4, 4: 4, 9: 4, 10: 4, 11: 4, 23: 4, 24: 8, 41: 4, 42: 4, 43: 4}


class Elf:
    def __init__(self, path):
        with path.open("rb") as stream:
            self.data = mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ)
        if self.data[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", self.data, 18)[0] != 62:
            raise ValueError("expected a little-endian x86-64 ELF64 file")
        self.shoff = U64.unpack_from(self.data, 40)[0]
        stride, count, strings = struct.unpack_from("<HHH", self.data, 58)
        if stride != SHDR.size:
            raise ValueError("unsupported section table")
        self.count = count or self.section(0)[5]
        self.section_names = self.section(self.section(0)[6] if strings == 0xffff else strings)[4]

    def section(self, i):
        return SHDR.unpack_from(self.data, self.shoff + i * SHDR.size)

    def sections(self):
        return (self.section(i) for i in range(self.count))

    def name(self, start):
        end = self.data.find(b"\0", start)
        if end < 0:
            raise ValueError("unterminated ELF symbol")
        return self.data[start:end]

    def functions(self):
        sym = next(s for s in self.sections() if s[1] == 2)
        ext = next((s for s in self.sections() if s[1] == 18), None)
        strings = self.section(sym[6])[4]
        for i, off in enumerate(range(sym[4], sym[4] + sym[5], SYM.size)):
            name, info, _, index, va, size = SYM.unpack_from(self.data, off)
            if info & 15 != 2 or not size or not index:
                continue
            if index == 0xffff:
                index = U32.unpack_from(self.data, ext[4] + 4 * i)[0]
            yield self.name(strings + name), index, va, size


def symbol_groups(path):
    """Map function addresses to a common demangled family, retaining ambiguous aliases."""
    groups, sizes = {}, {}
    with subprocess.Popen(["nm", "-n", "-S", "-C", "--defined-only", path],
                          stdout=subprocess.PIPE, text=True) as process:
        for line in process.stdout:
            fields = line.split(None, 3)
            if len(fields) != 4 or fields[2] not in {"T", "t", "W", "w", "I", "i"} or not int(fields[1], 16):
                continue
            addr = int(fields[0], 16)
            name = family(fields[3].strip())
            kind = category(name)
            if addr in groups:
                old_kind, old_name = groups[addr]
                if old_kind != kind:
                    kind = "Aliases from multiple categories"
                if old_name != name:
                    name = "Multiple function definitions"
            groups[addr] = (kind, name)
            sizes[addr] = max(sizes.get(addr, 0), int(fields[1], 16))
        if process.wait():
            raise RuntimeError("nm failed")
    return groups, sizes


def read_frames(data, sections, groups):
    va, off, size = sections[".eh_frame"]
    base, table_offset, table_size = sections[".gcc_except_table"]
    hdr = sections[".eh_frame_hdr"]
    header = data[hdr[1]:hdr[1] + 12]
    assert header[:4] == bytes.fromhex("011b033b")
    cies, spans, frame_sizes = {}, defaultdict(set), Counter()
    counts, owners = Counter(), {}
    i = 0
    while i + 4 <= size:
        start = off + i
        length, delta = struct.unpack_from("<II", data, start) if i + 8 <= size else (0, 0)
        if not length:
            break
        assert length < 0xffffffff and i + 4 + length <= size
        if delta == 0:
            end = data.index(0, start + 9)
            aug = data[start + 9:end]
            assert data[start + 8] == 1 and data[end + 1:end + 4] == bytes.fromhex("017810")
            encoding = data[end + 4:end + 4 + data[end + 4] + 1]
            assert (aug == b"zR" and encoding == b"\x01\x1b") or (
                aug == b"zPLR" and encoding[:2] == b"\x07\x9b" and encoding[-2:] == b"\x1b\x1b")
            cies[i] = aug
        else:
            pc = va + i + 8 + S32.unpack_from(data, start + 8)[0]
            owner = groups.get(pc, ("No sized symbol", ""))[0]
            frame_sizes[owner] += length + 4
            counts[owner] += 1
            aug = cies[i + 4 - delta]
            assert data[start + 16] == (4 if aug == b"zPLR" else 0)
            if aug == b"zPLR":
                ptr = S32.unpack_from(data, start + 17)[0]
                if ptr:
                    address = va + i + 17 + ptr
                    assert base <= address < base + table_size
                    spans[address].add(owner)
            owners[pc] = owners.get(pc, 0) + length + 4 + 8
        i += length + 4
    assert sum(counts.values()) == U32.unpack_from(header, 8)[0]
    assert hdr[2] == 12 + 8 * sum(counts.values())
    ordered = sorted(spans)
    storage = Counter({"Leading unreferenced bytes": ordered[0] - base})
    raw_spans = Counter()
    for start, end in zip(ordered, ordered[1:] + [base + table_size]):
        names = spans[start]
        kind = next(iter(names)) if len(names) == 1 else "Shared exception table"
        storage[kind] += end - start
        raw_spans[data[table_offset + start - base:table_offset + end - base]] += 1
    assert sum(storage.values()) == table_size
    return ({"frames": sum(counts.values()), "unique_exception_pointers": len(ordered),
             "raw_duplicate_exception_span_bytes": sum(len(blob) * (n - 1) for blob, n in raw_spans.items()),
             "common_record_and_index_header_bytes": size - sum(frame_sizes.values()) + 12,
             "frame_and_index_bytes": {k: n + 8 * counts[k] for k, n in frame_sizes.most_common()},
             "exception_spans_including_alignment": dict(storage.most_common())}, owners)


def relative_slots(data, sections, groups, sizes):
    ranges = sorted((va, va + size, name, off) for name, (va, off, size) in sections.items()
                    if va and size and name != ".tbss")
    starts = [row[0] for row in ranges]

    def containing(addr):
        i = bisect_right(starts, addr) - 1
        if i < 0 or addr >= ranges[i][1]:
            raise ValueError(f"relative pointer outside loaded sections at {addr:x}")
        return ranges[i]

    edges, text_slots, targets = Counter(), Counter(), defaultdict(set)
    seen = set()

    def fixup(slot, explicit=None):
        assert slot not in seen
        seen.add(slot)
        va, end, src, off = containing(slot)
        assert slot + 8 <= end
        val = U64.unpack_from(data, off + slot - va)[0] if explicit is None else explicit
        dst = containing(val)[2]
        edges[(src, dst)] += 1
        if dst == ".text":
            targets[src].add(val)
            if src == ".data.rel.ro":
                text_slots[groups.get(val, ("No symbol starting at target", ""))[0]] += 1

    _, off, size = sections[".relr.dyn"]
    cursor = None
    for word, in struct.iter_unpack("<Q", data[off:off + size]):
        if word & 1:
            assert cursor is not None
            bits = word >> 1
            while bits:
                bit = bits & -bits
                fixup(cursor + 8 * (bit.bit_length() - 1))
                bits ^= bit
            cursor += 63 * 8
        else:
            fixup(word)
            cursor = word + 8
    _, off, size = sections[".rela.dyn"]
    for slot, typ, val in RELA.iter_unpack(data[off:off + size]):
        if typ == 8:
            fixup(slot, val)
    addressed = Counter()
    for pc in targets[".data.rel.ro"]:
        if pc in groups:
            addressed[groups[pc][0]] += sizes[pc]
    return {"relro_distinct_exact_function_start_text": {
                "bytes": sum(addressed.values()), "by_category": dict(addressed.most_common()),
                "target_addresses_without_exact_symbol": len(targets[".data.rel.ro"] - groups.keys()),
                "scope": "Each exact function start referenced by a .data.rel.ro relative slot is counted once. Some functions also have other references. This measures the addressed machine code, not what would be removed by deleting the slots."},
            "unique_relative_slots": len(seen), "relative_slots_by_source_and_target":
            [{"source": src, "target": dst, "slots": n} for (src, dst), n in edges.most_common()],
            "distinct_text_targets": {k: len(v) for k, v in targets.items()},
            "relro_code_pointer_slots_by_destination_family": dict(text_slots.most_common())}



def object_exception_tables(obj):
    """Compare LSDA input sections before linking, retaining full unsimplified relocation records."""
    relocs = {s[7]: (s[4], s[5]) for s in obj.sections() if s[1] == 4}
    buckets = Counter()
    for index, s in enumerate(obj.sections()):
        if s[1] == 1 and obj.name(obj.section_names + s[0]).startswith(b".gcc_except_table"):
            offset, size = relocs.get(index, (0, 0))
            buckets[(obj.data[s[4]:s[4] + s[5]], obj.data[offset:offset + size])] += 1
    return {"sections": sum(buckets.values()), "raw_bytes": sum(len(b) * n for (b, _), n in buckets.items()),
            "unique_byte_and_relocation_vectors": len(buckets),
            "tables_with_relocations": sum(n for (_, r), n in buckets.items() if r),
            "raw_duplicate_bytes": sum(len(b) * (n - 1) for (b, _), n in buckets.items())}

def code_shapes(obj, linked, sections, groups, frame_cost):
    """Compare final machine bytes, masking only x86-64 relocation operand fields from the input object."""
    text_va, text_off, size = sections[".text"]
    names = {}
    for name, _, va, n in linked.functions():
        if text_va <= va < text_va + size:
            span = (va, n)
            names[name] = span if names.get(name, span) == span else None
    rel = {s[7]: (s[4], s[5]) for s in obj.sections() if s[1] == 4}
    signatures, excluded = {}, Counter()
    for name, sec, offset, length in obj.functions():
        span = names.get(name)
        if span is None:
            continue
        pc, extent = span
        if extent != length:
            excluded["symbol_size_mismatch"] += 1
            continue
        raw = bytearray(linked.data[text_off + pc - text_va:text_off + pc - text_va + length])
        layout, targets = bytearray(), []
        start, n = rel.get(sec, (0, 0))
        for pos in range(start, start + n, RELA.size):
            addr, typ, _ = RELA.unpack_from(obj.data, pos)
            if not offset <= addr < offset + length:
                continue
            typ &= 0xffffffff
            width = WIDTH.get(typ, 0)
            at = addr - offset
            if not width or at + width > len(raw):
                excluded["unsupported_relocation"] += 1
                break
            relative = typ in (2, 4, 9, 24, 41, 42, 43)
            operand = int.from_bytes(raw[at:at + width], "little", signed=relative)
            # The PC-relative target convention can include a constant assembler addend.
            # It is compared only at equal offsets in identical instruction layouts.
            targets.append(pc + at + width + operand if relative else operand)
            raw[at:at + width] = bytes(width)
            layout.extend(struct.pack("<II", at, typ))
        else:
            key = (pc, length)
            # Folded aliases can originate in several object sections. Reject ambiguous operand schemas.
            sig = (hashlib.sha256(struct.pack("<I", len(layout)) + layout + raw).digest(), tuple(targets))
            if key not in signatures:
                signatures[key] = sig
            elif signatures[key] != sig:
                signatures[key] = None
    buckets = defaultdict(list)
    for (pc, length), signature in signatures.items():
        if signature is not None:
            buckets[(length, signature[0])].append((pc, signature[1]))
    owners, outer, counts = Counter(), Counter(), Counter()
    leaders = []
    potential = matched = repeated_groups = frame_min = frame_max = same_operands = pointer_rows = 0
    for (length, _), members in buckets.items():
        pcs = [p for p, _ in members]
        matched += length * len(pcs)
        if len(pcs) < 2:
            continue
        repeated_groups += 1
        upper = (len(pcs) - 1) * length
        potential += upper
        operands = [v for _, v in members]
        distinct = len(set(operands))
        same_operands += (len(pcs) - distinct) * length
        varied = sum(len(set(v)) > 1 for v in zip(*operands))
        pointer_rows += 8 * varied * distinct
        kinds = {groups.get(pc, ("Unlabeled", ""))[0] for pc in pcs}
        fams = {groups.get(pc, ("", "Unlabeled"))[1] for pc in pcs}
        kind = next(iter(kinds)) if len(kinds) == 1 else "Multiple categories"
        fam = next(iter(fams)) if len(fams) == 1 else "Multiple definitions"
        owners[kind] += upper
        outer[fam] += upper
        counts[fam] += len(pcs)
        fc = [frame_cost.get(p, 0) for p in pcs]
        frame_min += sum(fc) - max(fc)
        frame_max += sum(fc) - min(fc)
        leaders.append({"family": fam, "category": kind, "body_bytes": length, "bodies": len(pcs),
                        "operand_sites": len(operands[0]),
                        "sites_varying_across_instances": varied,
                        "distinct_operand_vectors": distinct,
                        "excess_text_bytes": upper, "example_addresses": [hex(p) for p in sorted(pcs)[:2]]})
    return {"method": "Each unique physical function in the final ELF is matched by mangled name and size to its FatLTO input. Hash final linked instructions with only recorded relocation operands zeroed and retain the relocation type/offset list. Groups differ at most in those operands. Choosing one body per group ignores the costs and behavior of carrying the operands, indirect calls, unwind actions and surviving wrappers. This is an optimistic ceiling for this exact shape-sharing strategy, not a saving or a ceiling for all source transformations.",
            "matched_function_text_bytes": matched, "repeated_groups": repeated_groups,
            "redundant_text_before_adaptation": potential,
            "excess_text_even_with_equal_resolved_operands": same_operands,
            "associated_frame_and_index_bytes_excluding_largest_retained_frame_per_group": frame_min,
            "associated_frame_and_index_bytes_excluding_smallest_retained_frame_per_group": frame_max,
            "simple_operand_row_estimate": {"bytes": pointer_rows,
                "method": "For each shape, store eight bytes per varying operand site per distinct operand vector. Constants and duplicate rows within a shape cost zero in this estimate. An implementation may reuse existing type descriptors, index values more compactly or need additional dispatch code, relocation slots and unwind records. This is a representation scenario, not a measured application delta."},
            "per_category": dict(owners.most_common()), "per_family":
            [{"family": k, "redundant_text": v, "bodies_in_repeated_groups": counts[k]} for k, v in outer.most_common(30)],
            "largest_groups": sorted(leaders, key=lambda r: -r["excess_text_bytes"])[:20],
            "excluded_input_symbol_records": dict(excluded)}



def training_coverage(linked, text_section, groups, profile, tool):
    """Join original training records by exact name. Inlining, rewriting and CFG drift limit interpretation."""
    va, _, size = text_section
    names = defaultdict(list)
    for name, _, pc, n in linked.functions():
        if va <= pc < va + size:
            names[name].append((pc, n))
    observations = {}
    for covered in [False, True]:
        flags = ["--covered"] if covered else []
        with subprocess.Popen([tool, "show", "--all-functions", *flags, profile], stdout=subprocess.PIPE) as proc:
            for line in proc.stdout:
                name = line.strip().rsplit(b";", 1)[-1].removesuffix(b":")
                if name in names:
                    observations[name] = covered
            if proc.wait():
                raise RuntimeError("llvm-profdata show failed")
    statuses = defaultdict(set)
    for name, spans in names.items():
        for span in spans:
            statuses[span].add(observations.get(name))
    totals = Counter()
    by_category = defaultdict(Counter)
    for (pc, n), states in statuses.items():
        if True in states:
            label = "At least one exact alias executed"
        elif states == {False}:
            label = "Every exact alias has a zero-count record"
        elif False in states:
            label = "Zero-count and unmatched aliases"
        else:
            label = "No matching training symbol"
        totals[label] += n
        by_category[groups.get(pc, ("Unlabeled", ""))[0]][label] += n
    return {"profile_sha256": hashlib.sha256(profile.read_bytes()).hexdigest(),
            "scope": "Sum the bytes of each sized final ELF function once, joining all of its symbol aliases by exact name against LLVM training records. Remove only LLVM's optional codegen-unit prefix before the semicolon. Counts describe named training functions, not individual instructions in the final image. A zero count does not imply that the feature may be removed. Unmatched records and inlined callees cannot establish cold execution.",
            "function_text_bytes": dict(totals), "by_category": dict(sorted(by_category.items()))}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--shipping", required=True, type=Path)
    parser.add_argument("--symbolized", required=True, type=Path)
    parser.add_argument("--object", required=True, type=Path, help="final native FatLTO object passed to the linker")
    parser.add_argument("--release-report", type=Path, help="source release measurements containing the shipping SHA-256")
    parser.add_argument("--embedded-archive", type=Path, help="verify exact copies of a shipped FXP in .rodata")
    parser.add_argument("--profile", type=Path, help="original indexed training profile to match by mangled name")
    parser.add_argument("--llvm-profdata", default="llvm-profdata", help="matching LLVM profile tool")
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    sections = section_map(args.shipping)
    symsecs = section_map(args.symbolized)
    identities = {}
    for key in [".text", ".eh_frame", ".eh_frame_hdr", ".gcc_except_table", ".rodata", ".data.rel.ro", ".relr.dyn"]:
        a = text_identity(args.shipping, sections[key])
        if a != text_identity(args.symbolized, symsecs[key]):
            parser.error(f"{key} must match the shipping file in bytes, size and virtual address")
        identities[key] = a
    linked, obj = Elf(args.symbolized), Elf(args.object)
    groups, sizes = symbol_groups(args.symbolized)
    with subprocess.Popen(["nm", "-n", "-S", "-C", "--defined-only", args.symbolized],
                          stdout=subprocess.PIPE, text=True) as nm:
        detail = classify(nm.stdout, identities[".text"])
        if nm.wait():
            raise RuntimeError("nm failed")
    if detail["partial_overlap_bytes"]:
        parser.error("an overlapping physical function layout requires an interval-aware code-shape analysis")
    data = args.shipping.read_bytes()
    frames, per_pc = read_frames(data, sections, groups)
    frames["fatlto_input_exception_tables"] = object_exception_tables(obj)
    owners = Counter(detail["categories"])
    owners.update(frames["frame_and_index_bytes"])
    owners.update(frames["exception_spans_including_alignment"])
    owners["Common unwind header and CIEs"] += frames["common_record_and_index_header_bytes"]
    combined_size = sum(sections[k][2] for k in [".text", ".eh_frame", ".eh_frame_hdr", ".gcc_except_table"])
    assert sum(owners.values()) == combined_size
    out = {"shipping_bytes": len(data), "shipping_sha256": hashlib.sha256(data).hexdigest(),
           "attribution_method": "Count the union of sized machine-code intervals after folding. Group by the outer demangled symbol, using shared buckets for ambiguous aliases. FDE records and their search index follow the start address. Exception-table entries span from one referenced LSDA address to the next, including alignment and distinguishing shared spans. The disjoint subtotal is .text plus these three unwind sections. Relocated pointer slots, the code they address, training coverage and repeated shapes are separate views and must not be added to it.",
           "text_and_unwind_disjoint_total_by_category": dict(owners.most_common()),
           "matched_sections": identities, "physical_code": detail, "unwind": frames,
           "relative_pointers": relative_slots(data, sections, groups, sizes),
           "relocation_masked_code": code_shapes(obj, linked, symsecs, groups, per_pc)}
    if args.embedded_archive:
        archive = args.embedded_archive.read_bytes()
        _, off, size = sections[".rodata"]
        rodata = data[off:off + size]
        if not archive or archive not in rodata:
            parser.error("archive bytes are missing in .rodata")
        out["embedded_archive"] = {"bytes": len(archive), "sha256": hashlib.sha256(archive).hexdigest(),
                                   "copies_in_rodata": rodata.count(archive), "first_file_offset": off + rodata.index(archive)}
    if args.profile:
        out["training_coverage"] = training_coverage(linked, symsecs[".text"], groups, args.profile, args.llvm_profdata)
    if args.release_report:
        release = json.loads(args.release_report.read_text())
        levels = [level for level, r in release["profile_use"].items()
                  if r["application"]["sha256"] == out["shipping_sha256"]]
        if args.profile and out["training_coverage"]["profile_sha256"] != release["training"]["merged_profile"]["sha256"]:
            parser.error("profile SHA-256 does not match the release")
        if len(levels) != 1:
            parser.error("shipping image does not identify one profile-use release")
        out = {"source": release["source"], "compiler": release["rustc"], "target": release["target"],
               "profile_use": levels[0], "release_report": args.release_report.name, **out}
    args.output.write_text(json.dumps(out, indent=2) + "\n")


if __name__ == "__main__":
    main()
