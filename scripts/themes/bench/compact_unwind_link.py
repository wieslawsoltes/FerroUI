#!/usr/bin/env python3
"""Opt-in rustc linker for a stripped Linux x86-64 PIE. FERROUI_CC selects the cc executable."""
import hashlib
import os
from pathlib import Path
import struct
import subprocess
import sys
from tempfile import TemporaryDirectory

from attribute_codegen import Elf
from compact_unwind import require, rewrite_object

PHDR = struct.Struct("<IIQQQQQQ")
RESERVE = b".cfi_reserve"
FRAMES = b".eh_frame"
INDEX = b".eh_frame_hdr"


def frame_coverage(data, frames, index):
    """Check every binary-search entry against its FDE and compare PC ranges and LSDA targets."""
    fva, off, size = frames[3:6]
    iva, head, length = index[3:6]
    require(data[head:head+4] == b"\x01\x1b\x03\x3b", "unknown unwind index encoding")
    pointer, count = struct.unpack_from("<iI", data, head+4)
    require(iva+4+pointer == fva and length == 12+8*count, "invalid unwind index")
    signature = hashlib.sha256()
    for at in range(head+12, head+length, 8):
        relative_pc, relative_fde = struct.unpack_from("<ii", data, at)
        fde = iva + relative_fde
        pos = off + fde - fva
        require(off <= pos <= off+size-17, "FDE outside unwind section")
        record, cie, pc, span, aug = struct.unpack_from("<IIiIB", data, pos)
        require(0 < cie and record >= 13+aug and pos+4+record <= off+size, "invalid FDE")
        require(fde+8+pc == iva+relative_pc and aug in (0, 4), "unknown FDE layout")
        lsda = struct.unpack_from("<i", data, pos+17)[0] if aug else 0
        signature.update(struct.pack("<QIQ", iva+relative_pc, span, fde+17+lsda if lsda else 0))
    return signature.digest()


def inspect(path):
    elf = Elf(path)
    try:
        data = elf.data
        require(struct.unpack_from("<H", data, 16)[0] == 3, "expected PIE")
        entry, phoff = struct.unpack_from("<QQ", data, 24)
        stride, count = struct.unpack_from("<HH", data, 54)
        require(stride == PHDR.size, "unsupported program headers")
        programs = [PHDR.unpack_from(data, phoff+i*stride) for i in range(count)]
        kinds = {p[0] for p in programs}
        require({3, 0x6474e552, 0x6474e551} <= kinds, "expected interpreter, RELRO and GNU stack")
        require(all(not (p[0] == 0x6474e551 and p[1] & 1) and not (p[0] == 1 and p[1] & 3 == 3)
                    for p in programs), "unsupported memory permissions")
        loads = [p for p in programs if p[0] == 1]
        for _, _, offset, _, _, file_bytes, _, _ in programs:
            require(offset+file_bytes <= len(data), "segment outside file")
        sections = {}
        for s in elf.sections():
            if s[2] & 2:
                name = elf.name(elf.section_names+s[0])
                require(name not in sections, "ambiguous allocated section")
                if s[1] != 8 and s[5]:
                    require(any(s[4]-p[2] == s[3]-p[3] and 0 <= s[4]-p[2] <= p[5]-s[5]
                                for p in loads), "section outside file-backed load mapping")
                sections[name] = s
        coverage = frame_coverage(data, sections[FRAMES], sections[INDEX])
        content = {}
        for name, s in sections.items():
            digest = None
            if s[1] != 8 and name not in (FRAMES, INDEX, b".note.gnu.build-id"):
                digest = hashlib.sha256(memoryview(data)[s[4]:s[4]+s[5]]).digest()
            content[name] = (s[1], s[2], s[3], s[5], digest)
        return entry, programs, content, coverage
    finally:
        elf.data.close()


def verify(before, after, gap):
    old_entry, old_ph, old, old_frames = before
    entry, ph, sections, frames = after
    require(entry == old_entry and frames == old_frames, "entry point or FDE coverage changed")
    require(len(ph) == len(old_ph), "program headers changed")
    original = old[FRAMES]
    compact = sections[FRAMES]
    require(compact[:3] == original[:3] and compact[3]+gap == original[3], "unexpected frame section")
    require(RESERVE not in old and sections[RESERVE] == (8, 2, compact[2]+compact[3], gap, None),
            "unexpected reserve")
    require({k:v for k,v in old.items() if k != FRAMES} ==
            {k:v for k,v in sections.items() if k not in (FRAMES, RESERVE)}, "allocated sections changed")
    for a, b in zip(old_ph, ph):
        require(a[:2] == b[:2] and a[3:5] == b[3:5] and a[6:] == b[6:], "segment layout changed")
        shortened = a[0:2] == (1, 4) and a[3]+a[5] == original[2]+original[3]
        require(a[5]-b[5] == (gap if shortened else 0), "segment file size changed")
        require(b[0] != 1 or (b[2]-b[3]) % max(1,b[7]) == 0, "unaligned load segment")


def optimize(cc, args):
    # Side-effecting linker options (e.g. a link map) belong to the ordinary link only.
    flags = {"--as-needed","-Bstatic","-Bdynamic","--eh-frame-hdr","-z","noexecstack","--gc-sections",
             "relro","now","--strip-all","--icf=safe","pack-relative-relocs"}
    driver = {"-pie", "-m64", "-nodefaultlibs", "-fuse-ld=lld", "-s"}
    remaining = iter(args)
    for arg in remaining:
        if arg in {"-o", "-L", "-B"}:
            value = next(remaining, "")
            if not value or value.startswith(("-", "@")):
                return
        elif arg in driver or arg.startswith(("-l", "-L", "-B")):
            continue
        elif arg.startswith("-Wl,") and set(arg[4:].split(",")) <= flags:
            continue
        elif arg.startswith(("-", "@")) or not arg.endswith((".o", ".rlib", ".so", ".a")):
            return
    objects = [i for i, a in enumerate(args) if a.endswith(".rcgu.o")]
    if len(objects) != 1 or args.count("-o") != 1 or "-pie" not in args or "-fuse-ld=lld" not in args:
        return
    oi = args.index("-o")+1
    out = Path(args[oi])
    if not out.is_file() or out.is_symlink() or not ({"-s","-Wl,--strip-all"} & set(args)):
        return
    before = inspect(out)
    with TemporaryDirectory(prefix=".unwind-", dir=out.parent) as directory:
        tmp = Path(directory)
        obj, result = tmp/"compact.o", tmp/"image"
        rewrite_object(Path(args[objects[0]]), obj)
        new = args.copy()
        new[objects[0]], new[oi] = str(obj), str(result)
        subprocess.run([cc,*new], check=True)
        gap = before[2][FRAMES][3] - inspect(result)[2][FRAMES][3]
        require(gap > 0, "no unwind saving")
        script = tmp/"reserve.ld"
        script.write_text(f"SECTIONS {{ .cfi_reserve (NOLOAD) : {{ . += {gap}; }} }} INSERT AFTER .eh_frame;\n")
        subprocess.run([cc,*new,"-Xlinker","-T","-Xlinker",str(script)], check=True)
        require(result.stat().st_size < out.stat().st_size, "candidate is not smaller")
        verify(before, inspect(result), gap)
        os.replace(result, out)


def main(args):
    cc = os.environ.get("FERROUI_CC", "/usr/bin/cc")
    status = subprocess.run([cc,*args]).returncode
    if status == 0:
        try:
            optimize(cc, args)
        except (OSError, ValueError, IndexError, KeyError, struct.error, subprocess.CalledProcessError) as error:
            print(f"compact unwind: using ordinary linker output ({error})", file=sys.stderr)
    return status if status >= 0 else 128-status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
