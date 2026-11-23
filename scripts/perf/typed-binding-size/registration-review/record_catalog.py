#!/usr/bin/env python3
"""Record comparable section lengths, artifact hashes, registered invokers and rendered content."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

from census import HERE, macho

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
args = parser.parse_args()
out = args.output.resolve()

def sha(data):
    return hashlib.sha256(data).hexdigest()

def pixels(path):
    with tempfile.TemporaryDirectory() as temporary:
        bmp = Path(temporary) / "frame.bmp"
        subprocess.run(["sips", "-s", "format", "bmp", str(path), "--out", str(bmp)], check=True,
                       stdout=subprocess.DEVNULL)
        data = bmp.read_bytes()
    offset = struct.unpack_from("<I", data, 10)[0]
    width, height, planes, bits = struct.unpack_from("<iiHH", data, 18)
    assert planes == 1 and bits == 32 and len(data) - offset == width * abs(height) * 4
    return [width, abs(height)], data[offset:]

arms = {}
for arm in ["stock", "candidate"]:
    binary = (out / arm).read_bytes()
    segments, sections = macho(binary)
    arms[arm] = {
        "file_bytes":len(binary), "sha256":sha(binary), "linkmap_sha256":sha((out / f"{arm}.map").read_bytes()),
        "text_bytes":sections["__TEXT", "__text"][1], "read_only_const_bytes":sections["__TEXT","__const"][1],
        "relocatable_const_bytes":sections["__DATA_CONST", "__const"][1], "linkedit_segment_bytes":segments["__LINKEDIT"],
        "default_window_log":(out / f"{arm}-default.log").read_text().splitlines(),
        "verification":json.loads((out / f"{arm}-verified.json").read_text()),
    }
methods = json.loads(subprocess.check_output([sys.executable, str(HERE / "compare_methods.py"), str(out / "stock"), str(out / "candidate")]))
assert methods["specialized"]["candidate"]["method_rows"] > 0
captures = {}
for path in sorted((out / "stock-pages").glob("*.png")):
    if path.stem.endswith(("-composed", "-window")):
        continue
    shape, stock = pixels(path)
    candidate_shape, candidate = pixels(out / "candidate-pages" / path.name)
    assert shape == candidate_shape
    differences = sum(a != b for a,b in zip(memoryview(stock).cast("I"), memoryview(candidate).cast("I")))
    captures[path.name] = {"dimensions":shape, "pixels_different":differences,
                           "stock_pixels_sha256":sha(stock),"candidate_pixels_sha256":sha(candidate)}
assert len(captures) == 3
print(json.dumps({
    "revision":subprocess.check_output(["git","rev-parse","HEAD"],cwd=out/"workspace",text=True).strip(),
    "rustc":subprocess.check_output(["rustc","--version"],text=True).strip(),
    "target":"aarch64-apple-darwin", "profile":"dist", "patch_sha256":sha((HERE/"routed-handler.patch").read_bytes()),
    "base_features":["full-themes","markup-functions","type-tables"], "arms":arms, "methods":methods, "captures":captures
}, indent=2))
