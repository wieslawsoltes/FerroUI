"""Build matching arm64 dist probes, verify full inventories, and collect interleaved fresh-process samples."""
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys

here = Path(__file__).resolve().parent
out = Path.cwd() / "target/full-opcode-spike"
target = Path(os.environ.get("SPIKE_TARGET", out / "cargo"))
modes = ["native", "ast", "runtime"]

def build(mode):
    case = out / mode
    with (out / (mode + "-build.log")).open("w") as log:
        subprocess.run(["cargo", "build", "--offline", "--profile", "dist", "--manifest-path", case / "Cargo.toml"],
                       env=dict(os.environ, CARGO_TARGET_DIR=str(target)), stdout=log, stderr=log, check=True)
    exe = case / "probe"
    shutil.copy2(target / "dist/full-opcode-probe", exe)
    subprocess.run(["strip", exe], check=True)
    subprocess.run(["codesign", "-f", "-s", "-", exe], check=True)
    subprocess.run(["codesign", "--verify", exe], check=True)
    return exe

def verify(mode):
    exe = out / mode / "probe"
    with (out / (mode + "-probe.log")).open("w") as log:
        subprocess.run([exe], env=dict(os.environ, FULL_SPIKE_KEYS=str(exe.parent / "keys.txt"),
                         FERROUI_XAML_TIMINGS="1"), stdout=log, stderr=log, check=True)

def measure(count=11):
    keys = (out / "native/keys.txt").read_bytes()
    samples = {mode: [] for mode in modes}
    for mode in modes:
        assert (out / mode / "keys.txt").read_bytes() == keys, mode
    for i in range(count):
        for mode in modes[i % len(modes):] + modes[:i % len(modes)]:
            run = subprocess.run([out / mode / "probe"], stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                 check=True, text=True)
            samples[mode].append([int(v) for v in run.stdout.strip().split()])
    data = {mode: {"stripped_bytes": (out / mode / "probe").stat().st_size, "samples_ns": v,
                   "median_ns": list(map(statistics.median, zip(*v)))} for mode, v in samples.items()}
    data["stages"] = ["WinUITheme::new", "Button theme + template lookup", "first template build", "mean of 128 repeated builds"]
    (out / "timings.json").write_text(json.dumps(data, indent=2) + "\n")
    print(json.dumps({mode: data[mode]["median_ns"] for mode in modes}))

if __name__ == "__main__":
    for mode in sys.argv[1:] or modes:
        if mode == "timings":
            measure()
        else:
            build(mode)
            verify(mode)
