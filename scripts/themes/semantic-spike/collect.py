"""Collect the complete theme, reader and cache measurements after running each driver."""
import json
from pathlib import Path
import subprocess

here = Path(__file__).resolve().parent
root = here.parents[2]
out = root / "target/semantic-spike"

def read(path):
    return json.loads(path.read_text())

result = {
    "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
    "codec_revision": "8678e15b8ae738931237c0e70ef66f61a6ecbfd2",
    "host": "macOS arm64",
    "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
    "method": [
        "prepare.py --readers verifies the source and decoded syntax graph of every included document by re-encoding both with the pinned FXD encoder.",
        "Reader timings alternate XML and FXP on the same preloaded markup for 21 trials after warming each reader. Both materialize and drop every untransformed XamlX document.",
        "fluent.py uses seven interleaved fresh processes per mode. Both execute the full Fluent root, resolve and build Button, check two padding updates, resolve Light/Dark ButtonBackground and check compact ComboBox height.",
        "The four execution fields are theme construction, first Button theme and template lookup, first template build and mean of 128 subsequent builds. Times are nanoseconds, excluding process launch.",
        "cached.py executes two full WinUI themes per process, nine interleaved processes per mode. The second constructor reuses the loader's transformed group in the same address space. Both constructors use the real theme and retain normal deferred realization.",
        "All executable comparisons use release with fat LTO, one codegen unit, panic=abort and size-optimized theme crates. Some local builds overlapped the measurements."
    ],
    "reproduce_from_root": [
        "python3 -B scripts/themes/semantic-spike/prepare.py --readers",
        "python3 -B scripts/themes/full-opcode-spike/prepare.py",
        "python3 -B scripts/themes/full-opcode-spike/ast_project.py",
        "python3 -B scripts/themes/semantic-spike/fluent.py",
        "python3 -B scripts/themes/semantic-spike/cached.py",
        "python3 -B scripts/themes/semantic-spike/collect.py",
    ],
    "archives": read(out / "readers.json"),
    "fluent_full_execution": read(out / "fluent/results.json"),
    "winui_transformed_group_cache": read(out / "cache/results.json"),
}
(here / "results.json").write_text(json.dumps(result, indent=2) + "\n")
