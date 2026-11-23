"""Run from repository root with prebuilt dist rlibs for ferroui-markup-xaml."""
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys

here = Path(__file__).resolve().parent
out = Path("target/opcode-spike").resolve()
catalog = sys.argv[1:] == ["catalog"]
if catalog:
    out = out / "catalog"
out.mkdir(parents=True, exist_ok=True)
subprocess.run([sys.executable, here/("catalog_compile.py" if catalog else "compile.py"), out], check=True)
from build_driver import build
for mode in ("native","vm"):
    build(here/("catalog.rs" if catalog else "main.rs"), mode, out)
samples = {"native": [], "vm": []}
for i in range(9):
    for mode in (("native","vm") if i%2==0 else ("vm","native")):
        line = subprocess.check_output([out/mode],text=True)
        samples[mode].append(tuple(map(int,line.split())))
for mode, rows in samples.items():
    print(mode, "file bytes", (out/mode).stat().st_size, "first/repeat ns",
          *map(statistics.median,zip(*rows)), "samples", rows)
print("binary template bytes", (out/"template.bin").stat().st_size)
