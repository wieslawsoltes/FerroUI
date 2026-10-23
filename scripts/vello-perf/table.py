"""Summarizes the logs of bench.sh / bench-bin.sh as a Markdown table.

    python3 scripts/vello-perf/table.py <outdir> <label>,<label>,...

Each cell is the median milliseconds a frame with the device waited for, then without waiting
(what a window on screen pays on the processor); the tour rows are the total time of all frames,
with the 95th percentile of a frame.
"""
import glob
import os
import re
import sys

directory, labels = sys.argv[1], sys.argv[2].split(",")
rows = {}
for path in glob.glob(os.path.join(directory, "*.log")):
    label = os.path.basename(path).split("-frame_benchmark")[0]
    for line in open(path, errors="replace"):
        m = re.match(r"^(.+?): without waiting for the device: ms per frame: median ([\d.]+)", line)
        if m:
            rows.setdefault(m.group(1), {}).setdefault(label, {})["cpu"] = float(m.group(2))
            continue
        m = re.match(r"^(.+?): \d+ frames, (\d+) rendered; ms per frame: median ([\d.]+)", line)
        if m and int(m.group(2)) > 1:
            rows.setdefault(m.group(1), {}).setdefault(label, {})["all"] = float(m.group(3))
            continue
        m = re.search(r"\] (tour \d): .*rendering ([\d.]+) ms in all; ms per frame: median ([\d.]+), mean ([\d.]+), p95 ([\d.]+)", line)
        if m:
            rows.setdefault("catalog " + m.group(1) + " (total ms)", {}).setdefault(label, {})["tour"] = (float(m.group(2)), float(m.group(5)))
present = [label for label in labels if any(label in v for v in rows.values())]
print("| Scenario | " + " | ".join(present) + " |")
print("|---|" + "---|" * len(present))
for key in sorted(rows):
    cells = []
    for label in present:
        v = rows[key].get(label, {})
        if "tour" in v:
            cells.append(f"{v['tour'][0]:.0f} (p95 {v['tour'][1]:.2f})")
        elif "all" in v:
            cells.append(f"{v['all']:.2f} / {v.get('cpu', float('nan')):.2f}")
        else:
            cells.append("")
    print(f"| {key} | " + " | ".join(cells) + " |")
