"""Compact the two DWARF CFI encodings emitted by the measured Rust x86-64 release.

Operate on the final relocatable object, before LLD resolves addresses. Unknown
formats leave the original input available to the linker. No function is dropped.
"""
from collections import Counter
from pathlib import Path
import shutil
import struct

from attribute_codegen import Elf, RELA, SHDR, SYM, U32, U64


class UnsupportedUnwind(ValueError):
    """The input is outside the set of object and unwind formats we understand."""


# Version 1, code alignment 1, data alignment -8, return register 16.
# Both describe a PC-relative signed 32-bit PC. zPLR also has a PC-relative
# personality reference and an LSDA pointer. Its personality is relocated by LLD.
CIES = {
    bytes.fromhex("017a5200017810011b0c070890010000"): False,
    bytes.fromhex("017a504c5200017810079b000000001b1b0c070890010000"): True,
}


def require(condition, message):
    if not condition:
        raise UnsupportedUnwind(message)


def uleb(data, pos):
    value = 0
    for shift in range(0, 64, 7):
        if pos >= len(data):
            break
        byte = data[pos]
        pos += 1
        value |= (byte & 127) << shift
        if byte < 128:
            return value, pos
    raise UnsupportedUnwind("unterminated or oversized LEB operand")


def signed_leb(value):
    result = bytearray()
    while True:
        low = value & 127
        value >>= 7
        done = (value == 0 and low < 64) or (value == -1 and low >= 64)
        result.append(low if done else low | 128)
        if done:
            return result


def advance(delta):
    if delta == 0:
        return b""
    if delta < 64:
        return bytes([0x40 | delta])
    for opcode, width in ((2, 1), (3, 2), (4, 4)):
        if delta < 1 << (8 * width):
            return bytes([opcode]) + delta.to_bytes(width, "little")
    raise UnsupportedUnwind("unrepresentable PC advance")


def compact_program(data):
    """Return an equivalent instruction stream, with the first args-size rule intact."""
    out = bytearray()
    pos = pending_pc = redundant = scaled = 0
    previous_args = None
    while pos < len(data):
        start = pos
        op = data[pos]
        pos += 1
        if op == 0:  # DW_CFA_nop
            continue
        if op >> 6 == 1:  # DW_CFA_advance_loc
            pending_pc += op & 63
            continue
        if op in (2, 3, 4):  # DW_CFA_advance_loc1/2/4
            end = pos + (1 << (op - 2))
            require(end <= len(data), "truncated PC advance")
            pending_pc += int.from_bytes(data[pos:end], "little")
            pos = end
            continue
        replacement = None
        if op >> 6 == 2:  # DW_CFA_offset
            _, pos = uleb(data, pos)
        elif op >> 6 == 3:  # DW_CFA_restore
            pass
        elif op in (5, 9, 12, 17, 18, 20, 21):
            _, pos = uleb(data, pos)
            _, pos = uleb(data, pos)
        elif op in (6, 7, 8, 13, 14, 19, 46):
            value, pos = uleb(data, pos)
            if op == 46:  # DW_CFA_GNU_args_size
                if previous_args == value:
                    redundant += 1
                    continue
                previous_args = value
            elif op == 14 and value < 1 << 63 and value % 8 == 0:  # DW_CFA_def_cfa_offset
                candidate = bytes([19]) + signed_leb(value // -8)
                if len(candidate) < pos - start:
                    replacement = candidate
                    scaled += 1
        else:
            # Includes remember/restore-state, expressions and relocated set_loc.
            # None of the measured FDEs requires transforming those programs.
            return data, 0, 0
        out.extend(advance(pending_pc))
        pending_pc = 0
        out.extend(data[start:pos] if replacement is None else replacement)
    # A trailing advance has no subsequent rule change in this FDE.
    return (out if redundant or scaled else data), redundant, scaled


def compact_records(data):
    out = bytearray()
    fields = {}
    cies = {}
    stats = Counter()
    pos = 0
    while pos + 8 <= len(data):
        size, cie = struct.unpack_from("<II", data, pos)
        if size == 0:
            break
        end = pos + size + 4
        require(size >= 4 and end <= len(data) and end % 4 == 0, "invalid frame extent")
        new = len(out)
        if cie == 0:
            encoding = data[pos + 8:end]
            require(encoding in CIES, "unsupported CIE")
            cies[pos] = new, CIES[encoding]
            if CIES[encoding]:
                fields[pos + 19] = new + 19
            out.extend(data[pos:end])
        else:
            origin = pos + 4 - cie
            require(origin in cies, "missing CIE")
            new_origin, has_lsda = cies[origin]
            aug = 4 if has_lsda else 0
            first = 17 + aug
            require(pos + first <= end and data[pos + 16] == aug, "unsupported FDE")
            fields[pos + 8] = new + 8
            if aug:
                fields[pos + 17] = new + 17
            body, redundant, scaled = compact_program(data[pos + first:end])
            record = bytearray(data[pos:pos + first])
            record.extend(body)
            record.extend(bytes(-len(record) % 4))
            require(len(record) <= size + 4, "expanded frame")
            struct.pack_into("<II", record, 0, len(record) - 4, new + 4 - new_origin)
            out.extend(record)
            stats["fdes"] += 1
            stats["changed_fdes"] += bool(redundant or scaled)
            stats["redundant_args"] += redundant
            stats["scaled_offsets"] += scaled
        pos = end
    require(not any(data[pos:]), "invalid frame terminator")
    out.extend(data[pos:])
    return out, fields, stats


def rewrite_object(source: Path, dest: Path) -> dict:
    """Write a separate ET_REL object. All address resolution remains with LLD."""
    require(source.resolve() != dest.resolve() and not dest.exists(), "expected a new output")
    try:
        elf = Elf(source)
    except (ValueError, struct.error) as error:
        raise UnsupportedUnwind(str(error)) from error
    try:
        require(struct.unpack_from("<H", elf.data, 16)[0] == 1, "expected relocatable ELF")
        sections = list(elf.sections())
        frames = [i for i, s in enumerate(sections) if elf.name(elf.section_names+s[0]) == b".eh_frame"]
        require(len(frames) == 1, "expected one unwind section")
        index = frames[0]
        frame = sections[index]
        relocations = [s for s in sections if s[1] == 4 and s[7] == index]
        require(frame[1] in (1, 0x70000001) and len(relocations) == 1, "unsupported unwind relocation layout")
        rel = relocations[0]
        require(rel[9] == RELA.size and rel[5] % RELA.size == 0, "unsupported RELA entries")
        require(not any(s[1] == 9 and s[7] == index for s in sections), "unsupported REL unwind section")
        sym = sections[rel[6]]
        require(sym[1] == 2 and sym[9] == SYM.size and sum(s[1] in (2, 11) for s in sections) == 1,
                "expected one ordinary symbol table")
        extended = next((s for s in sections if s[1] == 18 and s[6] == rel[6]), None)
        # A symbol or an external relocation into the moved records needs more
        # than a relocation-offset adjustment. Leave such objects unchanged.
        frame_symbols = set()
        for i, at in enumerate(range(sym[4], sym[4]+sym[5], SYM.size)):
            _, info, _, section, value, extent = SYM.unpack_from(elf.data, at)
            if section == 0xffff:
                require(extended is not None, "missing extended symbol index")
                section = U32.unpack_from(elf.data, extended[4] + 4*i)[0]
            if section == index:
                require(info == 3 and value == 0 and extent == 0,
                        "symbol exposing unwind data")
                frame_symbols.add(i)
        if frame_symbols:
            for s in sections:
                if s[1] in (4, 9):
                    stride = RELA.size if s[1] == 4 else 16
                    require(s[9] == stride and s[5] % stride == 0, "unsupported relocation entry")
                    for at in range(s[4], s[4]+s[5], stride):
                        require(U64.unpack_from(elf.data, at + 8)[0] >> 32 not in frame_symbols,
                                "reference to unwind section")
        result, fields, stats = compact_records(elf.data[frame[4]:frame[4]+frame[5]])
        adjusted = bytearray(elf.data[rel[4]:rel[4]+rel[5]])
        for at in range(0, len(adjusted), RELA.size):
            offset, info, _ = RELA.unpack_from(adjusted, at)
            require(offset in fields and info & 0xffffffff == 2, "unexpected FDE relocation")
            U64.pack_into(adjusted, at, fields.pop(offset))
        require(not fields, "missing PC, LSDA or personality relocation")
        stats.update(old_bytes=frame[5], new_bytes=len(result))
        shutil.copyfile(source, dest)
        with dest.open("r+b") as output:
            for at, payload in ((frame[4], result), (rel[4], adjusted),
                                (elf.shoff + index*SHDR.size + 32, U64.pack(len(result)))):
                output.seek(at)
                output.write(payload)
        return dict(stats)
    except (IndexError, struct.error) as error:
        raise UnsupportedUnwind("incomplete object") from error
    finally:
        elf.data.close()
