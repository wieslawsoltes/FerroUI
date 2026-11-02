#!/usr/bin/env python3
"""Create an FXU operand update from the baseline and edited Cargo XAML build outputs."""

import argparse
from dataclasses import dataclass
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import struct
from tempfile import NamedTemporaryFile

U32 = struct.Struct("<I")
SLOT = struct.Struct("<QHBI")
MAX_BLOCK = 16 << 20
MAX_UPDATE = 64 << 20


def field(data):
    return U32.pack(len(data)) + data


def distinct(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate manifest key {key!r}")
        result[key] = value
    return result


def fingerprint(data):
    value = 0xCBF29CE484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{value:016x}"


@dataclass
class Emission:
    signature: tuple
    slots: list
    blocks: list
    schema: bytes


def load(manifest):
    if not manifest.name.endswith(".fxb.json"):
        raise ValueError("expected a generated .fxb.json manifest")
    prefix = manifest.name.removesuffix(".fxb.json")
    meta = json.loads(manifest.read_bytes(), object_pairs_hook=distinct)
    if not isinstance(meta, dict) or type(meta.get("version")) is not int or meta["version"] != 1:
        raise ValueError("unsupported FXB manifest")
    module, table = meta.get("module"), meta.get("blocks")
    if not isinstance(module, str) or not module or not isinstance(table, dict) or len(table) > 65536:
        raise ValueError("invalid FXB module or block table")
    folder = manifest.parent.resolve()

    def read(name):
        path = folder / name
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"missing or indirect build artifact {path}")
        return path.read_bytes()

    source = read(prefix + ".rs")
    if fingerprint(source) != meta.get("caller"):
        raise ValueError("native caller differs from its published manifest")
    signature = (module.encode(), prefix.encode(), source, read(prefix + ".fxp-options.rs"))
    digest = sha256(b"FerroUI/FXU/1")
    for value in signature:
        digest.update(field(value))
    pattern = re.compile(re.escape(prefix) + r"-([0-9a-f]{16})(?:-([1-9][0-9]*))?\.fxb")
    slots, blocks = [], []
    for name, layout in sorted(table.items()):
        match = pattern.fullmatch(name)
        if not match or not isinstance(layout, list) or len(layout) not in (2, 3):
            raise ValueError(f"invalid FXB slot {name!r}")
        kind = layout[0]
        if type(kind) is not int or not 1 <= kind <= 9 or (kind == 5) != (len(layout) == 3):
            raise ValueError(f"invalid FXB factory layout for {name}")
        imports = layout[2] if kind == 5 else 0
        collision = int(match[2] or 0)
        if type(imports) is not int or not 0 <= imports <= 0xFFFFFFFF or collision > 0xFFFF:
            raise ValueError(f"FXB slot out of range for {name}")
        data = read(name)
        if (not 6 <= len(data) <= MAX_BLOCK or data[:4] != b"FXB\0"
                or data[4] not in (1, 2, 3) or data[5] != kind or fingerprint(data) != layout[1]):
            raise ValueError(f"{name} differs from its published FXB generation")
        slot = SLOT.pack(int(match[1], 16), collision, kind, imports)
        digest.update(slot)
        digest.update(field(data))
        slots.append(slot)
        blocks.append(data)
    schema = b"FXS\1" + digest.digest() + field(signature[0]) + field(signature[1])
    schema += U32.pack(len(slots)) + b"".join(slots)
    if schema != read(prefix + ".fxs"):
        raise ValueError("installed schema differs from generated caller, options, or operands")
    return Emission(signature, slots, blocks, schema)


def publish(baseline, revision, output):
    if output.suffix != ".fxu" or output.is_symlink():
        raise ValueError("output must name a regular .fxu file")
    old, new = load(baseline), load(revision)
    if old.signature != new.signature or old.slots != new.slots:
        raise ValueError("native caller, compiler options, or factory imports changed; rebuild or use FXP")
    edits = [(i, b) for i, (a, b) in enumerate(zip(old.blocks, new.blocks)) if a != b]
    size = 72 + sum(8 + len(data) for _, data in edits)
    if size > MAX_UPDATE:
        raise ValueError("FXU exceeds 64 MiB")
    body = bytearray(b"FXU\1" + sha256(old.schema).digest() + U32.pack(len(edits)))
    for index, data in edits:
        body.extend(U32.pack(index))
        body.extend(field(data))
    body.extend(sha256(body).digest())
    temp = None
    try:
        with NamedTemporaryFile(dir=output.parent, prefix=".fxu-", delete=False) as stream:
            temp = Path(stream.name)
            stream.write(body)
        os.replace(temp, output)
    finally:
        if temp is not None:
            temp.unlink(missing_ok=True)
    return len(edits), len(body)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path, help="installed build's .fxb.json manifest")
    parser.add_argument("revision", type=Path, help="edited build's .fxb.json manifest")
    parser.add_argument("output", type=Path, help="destination .fxu file")
    args = parser.parse_args()
    try:
        count, size = publish(args.baseline, args.revision, args.output)
    except (ValueError, OSError) as error:
        parser.exit(1, f"{error}\n")
    print(f"{count} changed blocks, {size} bytes")


if __name__ == "__main__":
    main()
