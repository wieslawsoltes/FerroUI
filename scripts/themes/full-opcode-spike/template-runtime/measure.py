"""Compare independently built full-theme binaries in DIR, named THEME-{native,stock,single,fused}."""
import hashlib
import json
import os
from pathlib import Path
import random
import statistics
import subprocess
import sys

directory = Path(sys.argv[1]).resolve()
variants = ["native", "stock", "single", "fused"]
themes = ["winui", "fluent"]
interposer = directory / "allocation_probe.dylib"


def measure(theme, variant, iterations, allocations=False):
    exe = directory / f"{theme}-{variant}"
    env = dict(os.environ, TEMPLATE_ITERATIONS=str(iterations))
    env.pop("DYLD_INSERT_LIBRARIES", None)
    if allocations:
        env["DYLD_INSERT_LIBRARIES"] = str(interposer)
    proc = subprocess.run([exe], capture_output=True, text=True, env=env, check=True)
    line = next(s for s in proc.stderr.splitlines() if s.startswith("template_probe "))
    count, elapsed, alloc, realloc, free, requested = map(int, line.split()[1:])
    assert count == iterations
    if not allocations:
        assert (alloc, realloc, free, requested) == (0, 0, 0, 0)
    return elapsed / count, [alloc / count, realloc / count, free / count, requested / count]


def paired_interval(deltas):
    rng = random.Random(87132)
    medians = sorted(statistics.median(rng.choices(deltas, k=len(deltas))) for _ in range(10000))
    return [statistics.median(deltas), medians[249], medians[9749]]


observations = {}
for theme in themes:
    observations[theme] = {}
    for variant in variants:
        exe = directory / f"{theme}-{variant}"
        observe = observations[theme][variant] = {
            "bytes": exe.stat().st_size,
            "sha256": hashlib.sha256(exe.read_bytes()).hexdigest(),
        }
        measure(theme, variant, 1024)
        allocs = [measure(theme, variant, 2048, True)[1] for _ in range(3)]
        assert allocs[0] == allocs[1] == allocs[2] and allocs[0][0] > 0
        observe["allocations_reallocations_frees_requested_bytes_per_build"] = allocs[0]
        observe["ns_per_template_by_block"] = []

# This four-period balanced Latin square puts each mode in each slot and balances predecessor
# modes. Reverse theme order on alternating blocks. Each observation is a new process.
orders = [[0, 1, 3, 2], [1, 2, 0, 3], [2, 3, 1, 0], [3, 0, 2, 1]]
for block in range(24):
    for theme in (themes if block % 2 == 0 else reversed(themes)):
        for index in orders[block % len(orders)]:
            variant = variants[index]
            ns, _ = measure(theme, variant, 65536)
            observations[theme][variant]["ns_per_template_by_block"].append(ns)
    print(f"block {block + 1}/24", flush=True)

for theme in themes:
    stock = observations[theme]["stock"]["ns_per_template_by_block"]
    for variant in variants:
        entry = observations[theme][variant]
        data = entry["ns_per_template_by_block"]
        entry["median_ns"] = statistics.median(data)
        entry["candidate_minus_stock_median_and_bootstrap_95pct_ns"] = paired_interval([a - b for a, b in zip(data, stock)])
        print(theme, variant, round(entry["median_ns"], 1), entry["candidate_minus_stock_median_and_bootstrap_95pct_ns"], flush=True)
(directory / "timings.json").write_text(json.dumps(observations, indent=2) + "\n")
