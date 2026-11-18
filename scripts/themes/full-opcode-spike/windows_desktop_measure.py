"""Measure a linked Windows desktop sample and its compiler-generated XAML dependency graph."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

from application_image import measure


def desktop_measure(exe, backend, revision):
    data = exe.read_bytes()
    header = struct.unpack_from("<I", data, 0x3C)[0]
    if data[header:header + 4] != b"PE\0\0":
        raise ValueError(f"{exe} is missing a PE header")
    machine, count = struct.unpack_from("<HH", data, header + 4)
    if machine != 0x8664:
        raise ValueError(f"{exe} is not an AMD64 executable")
    optional_size = struct.unpack_from("<H", data, header + 20)[0]
    table = header + 24 + optional_size
    sections = {}
    for index in range(count):
        name, virtual_size, _, raw_size = struct.unpack_from("<8sIII", data, table + index * 40)
        sections[name.rstrip(b"\0").decode("ascii")] = {"raw_bytes": raw_size, "virtual_bytes": virtual_size}
    image = measure(exe.with_suffix(".d"))
    has_images = image["total"].get("generated_image_bytes", 0) > 0
    if has_images != (backend == "bytecode"):
        raise ValueError(f"{backend} does not match the compiler images for {exe}")
    return {
        "revision": revision, "backend": backend, "package": exe.stem,
        "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(),
        "pe_sections": sections, "compiler_output": image,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path, help="Cargo-built executable with its sibling .d file")
    parser.add_argument("--backend", choices=("native", "bytecode"), required=True)
    parser.add_argument("--revision", required=True, help="source revision used to build the executable")
    args = parser.parse_args()
    print(json.dumps(desktop_measure(args.executable, args.backend, args.revision), indent=2))
