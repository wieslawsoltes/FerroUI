"""Measure alternate native/typed Button builds in four previously linked full/trimmed probes."""
import json
from pathlib import Path
import random
import statistics
import subprocess
import sys

out = Path(sys.argv[1]).resolve()
cases = ["winui", "winui-button", "fluent", "fluent-button"]
modes = ["native", "bytecode"]
observations = {case: {mode: [] for mode in modes} for case in cases}
for block in range(32):
    order = cases[block % 4:] + cases[:block % 4]
    for case in order:
        for mode in (modes if block % 2 == 0 else reversed(modes)):
            proc = subprocess.run([out / f"{case}-{mode}"], text=True, capture_output=True, check=True)
            row = list(map(int, proc.stdout.split()))
            assert len(row) == 4
            observations[case][mode].append(row[3])
for case, data in observations.items():
    native, typed = data["native"], data["bytecode"]
    data["medians_ns"] = [statistics.median(native), statistics.median(typed)]
    deltas = [b - a for a, b in zip(native, typed)]
    rng = random.Random(8407)
    boot = sorted(statistics.median(rng.choices(deltas, k=len(deltas))) for _ in range(10000))
    data["paired_difference_ns_median_and_bootstrap_95pct"] = [statistics.median(deltas), boot[249], boot[9749]]
    print(case, data["medians_ns"], data["paired_difference_ns_median_and_bootstrap_95pct"], flush=True)
(out / "paired-templates.json").write_text(json.dumps(observations, indent=2) + "\n")
