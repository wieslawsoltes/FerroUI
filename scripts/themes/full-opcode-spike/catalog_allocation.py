"""Account for a stripped Mach-O Catalog executable using its matching Apple link map."""
import argparse
from collections import Counter
import hashlib
import json
import mmap
from pathlib import Path
import re
import struct
import subprocess


def macho(binary):
    fields = lambda offset, fmt: struct.unpack_from("<" + fmt, binary, offset)
    assert fields(0, "I")[0] == 0xfeedfacf
    segments, sections = {}, {}
    offset = 32
    for _ in range(fields(16, "I")[0]):
        command, size = fields(offset, "II")
        if command == 0x19:
            name, address, _, file_offset, file_bytes, _, _, count = fields(offset + 8, "16sQQQQiiI")
            name = name.rstrip(b"\0").decode()
            segments[name] = file_bytes
            for i in range(count):
                start = offset + 72 + i * 80
                label, _, addr, length, position = fields(start, "16s16sQQI")
                sections[name, label.rstrip(b"\0").decode()] = (addr, length, position)
        offset += size
    return segments, sections


def embedded_assets(binary, const):
    files = subprocess.check_output(["git", "ls-files", "-z", "--", "src", "samples", "components"]).decode().split("\0")
    extensions = set(".png .jpeg .jpg .webp .bmp .gif .ico .icns .ttf .otf .ttc .woff .woff2 .mp4 .m4a .mov .wav .svg .json .lottie .onnx .fxc".split())
    _, size, offset = const
    hits = set()
    for file in map(Path, files):
        if file.suffix.lower() in extensions and file.stat().st_size >= 128:
            data = file.read_bytes()
            start = binary.find(data, offset, offset + size)
            if start >= 0:
                hits.add((start, start + len(data)))
    covered = end = 0
    for start, stop in sorted(hits):
        covered += max(0, stop - max(start, end))
        end = max(end, stop)
    return covered


def instructions(linkmap, text_section):
    address, size, _ = text_section
    contents = linkmap.read_text(errors="replace").split("# Dead Stripped Symbols:")[0]
    assert re.search(rf"^0x{address:X}\s+0x0*{size:X}\s+__TEXT\s+__text$", contents, re.M | re.I)
    objects = {int(i): path for i, path in re.findall(r"^\[\s*(\d+)\]\s+(.+)$", contents, re.M)}
    rows = [(int(a, 16), int(n, 16), int(i), name) for a, n, i, name in re.findall(
        r"^(0x\w+)\s+(0x\w+)\s+\[\s*(\d+)\]\s+(.+)$", contents, re.M)
        if address <= int(a, 16) < address + size and int(n, 16)]
    names = "\n".join(name.removeprefix("_") for _, _, _, name in rows) + "\n"
    decoded = subprocess.run(["xcrun", "llvm-cxxfilt", "-n"], input=names, capture_output=True, text=True, check=True).stdout.splitlines()
    assert len(decoded) == len(rows)
    groups, generated = Counter(), Counter()
    generated_origins, construction_imports = Counter(), Counter()
    generated_path = re.compile(r"\b\w+::compiled_(?!binding)\w+::")
    end = address
    for (start, length, index, raw), name in zip(rows, decoded):
        assert start >= end and start + length <= address + size
        end = start + length
        owner = re.match(r"<?([a-zA-Z_][a-zA-Z0-9_]*)(?:::|\[)", name)
        crate = owner[1] if raw.startswith("__R") and owner else ""
        generated_module = re.match(r"^([a-zA-Z0-9_]+)::(compiled_(?!binding)[A-Za-z0-9_]+)::", name)
        # Trait call shims start with '<' and were deliberately outside the
        # original direct-module census. Keep this second attribution orthogonal.
        if generated_path.search(name):
            if generated_module:
                role = "directly named generated XAML"
            elif name.startswith("<") and re.match(r"^<\w+::compiled_(?!binding)\w+::", name):
                role = "trait implementations for generated types"
            else:
                role = "other symbols naming a generated module"
            generated_origins[role] += length
            if "::CONSTRUCTION_IMPORTS::" in name:
                construction_imports[role] += length
        if generated_module:
            generated[crate] += length
            category = "Generated XAML instructions"
        elif crate == "ferroui_base":
            if re.match(r"<?ferroui_base::(property_store|styled_property|direct_property)::", name):
                category = "FerroUI.Base property store and property types"
            elif re.match(r"<?ferroui_base::data::core::typed_binding_expression::", name):
                category = "FerroUI.Base typed binding expressions"
            else:
                category = "Other FerroUI.Base instructions"
        elif crate == "control_catalog":
            category = "Other ControlCatalog instructions"
        elif crate == "ferroui_controls":
            category = "Core controls"
        elif crate == "ferroui_markup_xaml":
            category = "XAML runtime library"
        elif crate in ("core", "alloc", "std"):
            category = "Rust core, alloc and std"
        elif any(label in objects[index] for label in ("libskia_bindings-", "libharfbuzz_sys-")):
            category = "Skia and HarfBuzz native archives"
        else:
            category = "Other instructions and code alignment"
        groups[category] += length
    groups["Other instructions and code alignment"] += size - sum(groups.values())
    assert generated_origins["directly named generated XAML"] == sum(generated.values())
    return groups, generated, generated_origins, construction_imports


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("map", type=Path)
    args = parser.parse_args()
    with args.binary.open("rb") as file, mmap.mmap(file.fileno(), 0, access=mmap.ACCESS_READ) as binary:
        segments, sections = macho(binary)
        groups, generated, generated_origins, construction_imports = instructions(args.map, sections["__TEXT", "__text"])
        constants = sections["__TEXT", "__const"]
        assets = embedded_assets(binary, constants)
        groups["Embedded assets matched to source files"] = assets
        groups["Other read-only constants"] = constants[1] - assets
        groups["Relocatable constant data"] = sections["__DATA_CONST", "__const"][1]
        groups["Mach-O link and signing data"] = segments["__LINKEDIT"]
        groups["Other sections, headers and padding"] = len(binary) - sum(groups.values())
        print(json.dumps(dict(bytes=len(binary), sha256=hashlib.sha256(binary).hexdigest(),
                              allocation=groups, generated_instructions_by_crate=generated,
                              generated_xaml_instructions_by_symbol=generated_origins,
                              construction_import_instructions_by_symbol=construction_imports), indent=2))


if __name__ == "__main__":
    main()
