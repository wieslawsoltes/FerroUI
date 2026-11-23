#!/usr/bin/env python3
"""Shader libraries made ahead for Metal (docs/porting/gpu-rendering.md, "Pipelines made ahead of
their first draw").

    python3 scripts/perf/metal-libraries.py build <table.fml> <folder of .metal texts>...
    python3 scripts/perf/metal-libraries.py pack <table.fml> <table.fmz>
    python3 scripts/perf/metal-libraries.py list <table.fml or table.fmz>

The texts are what `FERROUI_METAL_LIBRARY_RECORD=<folder>` wrote while a program ran: every
shader text Skia gave the Metal device. `build` compiles each with the Metal compiler of Xcode
(`xcrun metal`, `xcrun metallib`), the way the device compiles it (the language version Skia
asks for, the default options), and writes the table:

    FML1, then for each library: the key of its text (u64), the length of the text (u32), the
    length of the library (u32), little endian, and the library.

The key is FNV-1a over the bytes of the text. A text that does not compile is left out and
named; the device compiles it when it meets it. `pack` writes the table a program carries
(`ferroui_skia::register_metal_libraries(include_bytes!(..))`, or `FERROUI_METAL_LIBRARIES`):
FMZ1, the length of the table (u32), and the table as an LZFSE stream, which the system
unpacks; a quarter of the size.

The table of ordinary drawing the crate carries
(`src/Skia/FerroUI.Skia/gpu/metal/libraries/ordinary-macos-apple.fmz`) is made of the texts of
the pipelines of the record of ordinary drawing and of the scenes of the gauntlet sample, on a
Mac with Apple silicon:

    FERROUI_METAL_LIBRARY_RECORD=<texts> cargo test -p ferroui-skia --lib \
        the_libraries_of_ordinary_drawing -- --test-threads=1
    for scene in gauntlet wall patchwork; do      # a few seconds each, then close the window
        FERROUI_METAL_LIBRARY_RECORD=<texts> PERF_SCENARIO=$scene <target>/release/perf-compare-ferroui
    done
    python3 scripts/perf/metal-libraries.py build ordinary.fml <texts>
    python3 scripts/perf/metal-libraries.py pack ordinary.fml \
        src/Skia/FerroUI.Skia/gpu/metal/libraries/ordinary-macos-apple.fmz

It is made again when the pinned Skia changes (the test above fails: Skia writes other texts).
"""

import argparse
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

MAGIC = b"FML1"
PACKED = b"FMZ1"


def key(text: bytes) -> int:
    value = 0xCBF29CE484222325
    for byte in text:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value


def compile_text(source: Path, sdk: str, std: str, minimum: str, work: Path) -> bytes | None:
    air = work / (source.stem + ".air")
    library = work / (source.stem + ".metallib")
    flag = "-mmacosx-version-min=" if sdk == "macosx" else "-mios-version-min="
    steps = (
        ["xcrun", "-sdk", sdk, "metal", "-c", "-std=" + std, flag + minimum, "-Wno-everything",
         "-o", str(air), str(source)],
        ["xcrun", "-sdk", sdk, "metallib", "-o", str(library), str(air)],
    )
    for step in steps:
        done = subprocess.run(step, capture_output=True, text=True)
        if done.returncode != 0:
            print(f"{source.name}: {' '.join(step[3:5])} failed\n{done.stderr.strip()[:2000]}", file=sys.stderr)
            return None
    return library.read_bytes()


def build(arguments: argparse.Namespace) -> int:
    # A text that is in several folders is the same text: its name is its key.
    by_name = {}
    for folder in arguments.folders:
        for source in Path(folder).glob("*.metal"):
            by_name.setdefault(source.name, source)
    texts = [by_name[name] for name in sorted(by_name)]
    if not texts:
        print(f"no .metal texts in {' '.join(arguments.folders)}", file=sys.stderr)
        return 1
    table = bytearray(MAGIC)
    made = text_bytes = library_bytes = 0
    with tempfile.TemporaryDirectory() as work:
        for source in texts:
            text = source.read_bytes()
            library = compile_text(source, arguments.sdk, arguments.std, arguments.minimum, Path(work))
            if library is None:
                continue
            table += struct.pack("<QII", key(text), len(text), len(library)) + library
            made += 1
            text_bytes += len(text)
            library_bytes += len(library)
    Path(arguments.table).write_bytes(table)
    print(f"{made} of {len(texts)} libraries, {text_bytes} bytes of text, {library_bytes} bytes of "
          f"libraries, table {len(table)} bytes: {arguments.table}")
    return 0 if made == len(texts) else 2


def lzfse(data: bytes, direction: str) -> bytes:
    done = subprocess.run(["/usr/bin/compression_tool", direction, "-a", "lzfse"], input=data, capture_output=True)
    if done.returncode != 0:
        raise SystemExit(f"compression_tool {direction}: {done.stderr.decode(errors='replace').strip()}")
    return done.stdout


def read_table(path: str) -> bytes:
    table = Path(path).read_bytes()
    if table[:4] == PACKED:
        (length,) = struct.unpack_from("<I", table, 4)
        table = lzfse(table[8:], "-decode")
        if len(table) != length:
            raise SystemExit("the packed table does not unpack to its length")
    return table


def pack(arguments: argparse.Namespace) -> int:
    table = read_table(arguments.table)
    count = sum(1 for _ in entries(table))
    packed = PACKED + struct.pack("<I", len(table)) + lzfse(table, "-encode")
    Path(arguments.packed).write_bytes(packed)
    print(f"{count} libraries, table {len(table)} bytes, packed {len(packed)} bytes: {arguments.packed}")
    return 0


def entries(table: bytes):
    if table[:4] != MAGIC:
        raise SystemExit("not a table of Metal libraries")
    at = 4
    while at + 16 <= len(table):
        text_key, text_length, length = struct.unpack_from("<QII", table, at)
        at += 16
        if at + length > len(table):
            break
        yield text_key, text_length, table[at:at + length]
        at += length


def show(arguments: argparse.Namespace) -> int:
    count = total = 0
    for text_key, text_length, library in entries(read_table(arguments.table)):
        print(f"{text_key:016x}  text {text_length:7d}  library {len(library):7d}")
        count += 1
        total += len(library)
    print(f"{count} libraries, {total} bytes")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    build_command = commands.add_parser("build", help="compile the texts of folders into a table")
    build_command.add_argument("table")
    build_command.add_argument("folders", nargs="+")
    build_command.add_argument("--sdk", default="macosx", choices=("macosx", "iphoneos", "iphonesimulator"))
    build_command.add_argument("--std", default=None, help="the language (default: Metal 2.3 of the SDK)")
    build_command.add_argument("--minimum", default=None, help="the oldest system (default: 11.0, iOS 14.0)")
    build_command.set_defaults(run=build)
    pack_command = commands.add_parser("pack", help="pack a table for a program to carry")
    pack_command.add_argument("table")
    pack_command.add_argument("packed")
    pack_command.set_defaults(run=pack)
    list_command = commands.add_parser("list", help="the libraries of a table")
    list_command.add_argument("table")
    list_command.set_defaults(run=show)
    arguments = parser.parse_args()
    if arguments.command == "build":
        mac = arguments.sdk == "macosx"
        arguments.std = arguments.std or ("macos-metal2.3" if mac else "ios-metal2.3")
        arguments.minimum = arguments.minimum or ("11.0" if mac else "14.0")
    return arguments.run(arguments)


if __name__ == "__main__":
    sys.exit(main())
