"""Measure process spawn through the setup marker or first successful compositor draw."""
from pathlib import Path
import argparse
import json
import os
import statistics
import subprocess
import threading
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--directory", type=Path, default=Path.cwd() / "target/full-opcode-spike/catalog")
parser.add_argument("--baseline", default="native")
parser.add_argument("--candidate", default="ast")
parser.add_argument("--pairs", type=int, default=7)
parser.add_argument("--marker", default="ControlCatalog: the theme variant Light")
parser.add_argument("--output", default="startup.json")
parser.add_argument("--first-frame", action="store_true", help="time the compositor's first successful draw")
args = parser.parse_args()
if args.pairs < 1 or args.baseline == args.candidate:
    parser.error("choose distinct executables and at least one pair")
out = args.directory.resolve()
env = dict(os.environ, FERROUI_CATALOG_THEME_VARIANT="light", FERROUI_SMOKE_EXIT_MS="150")
if args.first_frame:
    env.update(FERROUI_SLOW_FRAME_MS="0", FERROUI_SMOKE_EXIT_MS="500")
rows = {args.baseline: [], args.candidate: []}
for index in range(args.pairs):
    for mode in (list(rows) if index % 2 == 0 else reversed(rows)):
        start = time.perf_counter_ns()
        with subprocess.Popen([out / mode], env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT) as job:
            deadline = threading.Timer(20, job.kill)
            deadline.start()
            try:
                for line in job.stdout:
                    if args.first_frame:
                        reached = line.startswith("slow frame of composition target ") and ": frame 0 took " in line
                    else:
                        reached = args.marker in line
                    if reached:
                        rows[mode].append(time.perf_counter_ns() - start)
                        break
                job.communicate()
                if job.returncode or len(rows[mode]) != index + 1:
                    raise RuntimeError(f"{mode} failed to finish the measured launch in pair {index + 1}")
            finally:
                deadline.cancel()
for mode in rows:
    assert len(rows[mode]) == args.pairs, mode
data = {mode: {"samples_ns": ns, "median_ns": statistics.median(ns)} for mode, ns in rows.items()}
(out / args.output).write_text(json.dumps(data, indent=2) + "\n")
print(json.dumps(data))
