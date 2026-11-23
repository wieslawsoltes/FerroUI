#!/usr/bin/env python3
"""Records of GPU pipelines (`src/Skia/FerroUI.Skia/gpu/graphite/pipeline_record.rs`): what a
record holds, and several of them merged into one.

    python3 scripts/perf/pipeline-records.py list <record>...
    python3 scripts/perf/pipeline-records.py merge <out> <record>...

A record is written by a run of an application with `FERROUI_PIPELINE_RECORD=<file>` in its
environment: the keys of the pipelines its screens made (the file) and a line a pipeline with
Skia's label of it (`<file>.txt`). An application ships its record and registers it
(`ferroui_skia::register_pipeline_record`); the record the Skia crate carries for ordinary
drawing (`gpu/graphite/records/ordinary-macos-apple.fpr`) is the merge of the records of the
screens it was recorded from, made with `merge`.

The format: the magic `FPR1`, then for each key its length as a little-endian 32-bit number and
its bytes.
"""

import struct
import sys
from pathlib import Path

MAGIC = b"FPR1"


def keys(path: Path) -> list[bytes]:
    data = path.read_bytes()
    if data[:4] != MAGIC:
        sys.exit(f"{path}: not a record of pipelines")
    found, at = [], 4
    while at < len(data):
        if at + 4 > len(data):
            sys.exit(f"{path}: cut short")
        (length,) = struct.unpack_from("<I", data, at)
        at += 4
        if at + length > len(data):
            sys.exit(f"{path}: cut short")
        found.append(data[at : at + length])
        at += length
    return found


def main() -> None:
    if len(sys.argv) < 3 or sys.argv[1] not in ("list", "merge"):
        sys.exit(__doc__)
    if sys.argv[1] == "list":
        for name in sys.argv[2:]:
            found = keys(Path(name))
            print(f"{name}: {len(found)} keys, {Path(name).stat().st_size} bytes")
        return
    out, sources = Path(sys.argv[2]), [Path(name) for name in sys.argv[3:]]
    merged: dict[bytes, None] = {}
    for source in sources:
        for key in keys(source):
            merged.setdefault(key)
    with out.open("wb") as file:
        file.write(MAGIC)
        for key in merged:
            file.write(struct.pack("<I", len(key)))
            file.write(key)
    print(f"{out}: {len(merged)} keys from {len(sources)} records, {out.stat().st_size} bytes")


if __name__ == "__main__":
    main()
