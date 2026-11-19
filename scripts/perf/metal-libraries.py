#!/usr/bin/env python3
"""Shader libraries made ahead for Metal (docs/porting/gpu-rendering.md, "Pipelines made ahead of
their first draw").

    python3 scripts/perf/metal-libraries.py make <platform> <folder of .metal texts>...
    python3 scripts/perf/metal-libraries.py build <table.fml> <folder of .metal texts>...
    python3 scripts/perf/metal-libraries.py pack <table.fml> <table.fmz>
    python3 scripts/perf/metal-libraries.py list <table.fml or table.fmz>
    python3 scripts/perf/metal-libraries.py check <table.fml or table.fmz> <folder of .metal texts>...

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

`check` says how many texts of folders a table has a library for, and names the others: what a
recording run on a kind of device would still compile.

The crate carries a table of ordinary drawing for each kind of device of `PLATFORMS` below
(`src/Skia/FerroUI.Skia/gpu/metal/libraries/ordinary-<platform>.fmz`; its build script takes
the file of the target when there is one, so a table for another kind of device is a file and a
line here). `make <platform>` is the one step that makes one: it compiles the texts with the SDK
and the language of the platform and packs the table into the crate. The texts are those of the
pipelines of the record of ordinary drawing and of the scenes of the gauntlet sample, recorded on
a Mac with Apple silicon:

    FERROUI_METAL_LIBRARY_RECORD=<texts> cargo test -p ferroui-skia --lib \
        the_libraries_of_ordinary_drawing -- --test-threads=1
    for scene in gauntlet wall patchwork; do      # a few seconds each, then close the window
        FERROUI_METAL_LIBRARY_RECORD=<texts> PERF_SCENARIO=$scene <target>/release/perf-compare-ferroui
    done
    for platform in macos-apple ios-apple ios-simulator; do
        python3 scripts/perf/metal-libraries.py make $platform <texts>
    done

Skia writes the same texts for the GPUs of the Apple family on macOS and on iOS: the text follows
the capabilities of the device (`MtlCaps`), and an Apple GPU from the A14 and the M1 on has the
same ones, as has the device of the iOS simulator, which is the GPU of the Mac. The libraries of
iOS are compiled for iOS (`ios-apple`) and for its simulator (`ios-simulator`, which loads only
libraries made for it) from the texts recorded on the Mac. A text of another device (an older
iPhone, an Intel Mac) is not in a table and is compiled as before. None of the texts of ordinary
drawing reads its destination, which the simulator's compiler would reject (`gpu-rendering.md`,
"The iOS simulator"); `make` leaves out and names a text a compiler rejects.

A table is made again when the pinned Skia changes (the test above fails: Skia writes other
texts).
"""

import argparse
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

MAGIC = b"FML1"
PACKED = b"FMZ1"

# The kinds of device the crate carries a table for: the SDK, the language Skia asks for (Metal
# 2.3) and the oldest system, and the minimum flag of the compiler for it. Keep the names in step
# with `table_platform` of `src/Skia/FerroUI.Skia/build.rs`.
PLATFORMS = {
    "macos-apple": ("macosx", "macos-metal2.3", "-mmacosx-version-min=", "11.0"),
    "ios-apple": ("iphoneos", "ios-metal2.3", "-mios-version-min=", "14.0"),
    "ios-simulator": ("iphonesimulator", "ios-metal2.3", "-mios-simulator-version-min=", "14.0"),
}

# Where the crate keeps its tables.
CRATE_TABLES = Path(__file__).resolve().parents[2] / "src/Skia/FerroUI.Skia/gpu/metal/libraries"


def key(text: bytes) -> int:
    value = 0xCBF29CE484222325
    for byte in text:
        value ^= byte
        value = (value * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return value


def minimum_flag(sdk: str) -> str:
    # The simulator has a flag of its own: with the one of the devices the library is made for a
    # device, which the simulator does not load.
    return next(flag for platform_sdk, _, flag, _ in PLATFORMS.values() if platform_sdk == sdk)


def compile_text(source: Path, sdk: str, std: str, minimum: str, work: Path) -> bytes | None:
    air = work / (source.stem + ".air")
    library = work / (source.stem + ".metallib")
    flag = minimum_flag(sdk)
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


def texts_of(folders: list[str]) -> list[Path]:
    # A text that is in several folders is the same text: its name is its key.
    by_name = {}
    for folder in folders:
        for source in Path(folder).glob("*.metal"):
            by_name.setdefault(source.name, source)
    return [by_name[name] for name in sorted(by_name)]


def build(arguments: argparse.Namespace) -> int:
    texts = texts_of(arguments.folders)
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


def make(arguments: argparse.Namespace) -> int:
    sdk, std, _, minimum = PLATFORMS[arguments.platform]
    packed = Path(arguments.out) if arguments.out else CRATE_TABLES / f"ordinary-{arguments.platform}.fmz"
    with tempfile.TemporaryDirectory() as work:
        table = Path(work) / "table.fml"
        steps = argparse.Namespace(table=str(table), folders=arguments.folders, sdk=sdk, std=std, minimum=minimum)
        built = build(steps)
        if built == 1:
            return built
        pack(argparse.Namespace(table=str(table), packed=str(packed)))
    # Texts the compiler of the platform rejects (2) are named above and compiled at run time.
    return built


def check(arguments: argparse.Namespace) -> int:
    table = read_table(arguments.table)
    keys = {text_key: text_length for text_key, text_length, _ in entries(table)}
    texts = texts_of(arguments.folders)
    missing = []
    for source in texts:
        text = source.read_bytes()
        if keys.get(key(text)) != len(text):
            missing.append(source.name)
    for name in missing:
        print(f"not in the table: {name}")
    print(f"{len(texts) - len(missing)} of {len(texts)} texts have a library in {arguments.table}")
    return 0 if texts and not missing else 2


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
    make_command = commands.add_parser("make", help="compile and pack the table of a kind of device for the crate")
    make_command.add_argument("platform", choices=sorted(PLATFORMS))
    make_command.add_argument("folders", nargs="+")
    make_command.add_argument("--out", default=None, help="the packed table (default: the file of the crate)")
    make_command.set_defaults(run=make)
    pack_command = commands.add_parser("pack", help="pack a table for a program to carry")
    pack_command.add_argument("table")
    pack_command.add_argument("packed")
    pack_command.set_defaults(run=pack)
    list_command = commands.add_parser("list", help="the libraries of a table")
    list_command.add_argument("table")
    list_command.set_defaults(run=show)
    check_command = commands.add_parser("check", help="the texts of folders a table has a library for")
    check_command.add_argument("table")
    check_command.add_argument("folders", nargs="+")
    check_command.set_defaults(run=check)
    arguments = parser.parse_args()
    if arguments.command == "build":
        _, std, _, minimum = next(value for value in PLATFORMS.values() if value[0] == arguments.sdk)
        arguments.std = arguments.std or std
        arguments.minimum = arguments.minimum or minimum
    return arguments.run(arguments)


if __name__ == "__main__":
    sys.exit(main())
