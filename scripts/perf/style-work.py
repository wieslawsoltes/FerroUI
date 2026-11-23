#!/usr/bin/env python3
"""Interleave two ferroui-base style_work_probe test executables.

Build and retain the two release test binaries separately. Run without
perf-counters for timing; repeat with it to inspect work counts. Each sample
uses a fresh process. Construction, verification, printing and teardown are
outside the reported timings; process launch time is not measured.
"""

import argparse
import hashlib
import json
import os
import platform
import random
from pathlib import Path
import re
import statistics
import subprocess


def sample(binary, elements):
    env = dict(os.environ, FERROUI_STYLE_PROBE_ELEMENTS=str(elements))
    result = subprocess.run(
        [str(binary), "style_work_probe", "--ignored", "--nocapture", "--test-threads=1"],
        env=env, text=True, capture_output=True, timeout=180, check=True,
    )
    samples = {}
    for match in re.finditer(r"([a-z_]+): elements=(\d+), elapsed_us=([\d.]+)([^\n]*)", result.stdout):
        name, size, elapsed, rest = match.groups()
        assert int(size) == elements
        counts = {key: int(value) for key, value in re.findall(r"(\w+)=(\d+)", rest)}
        samples[name] = {"elapsed_us": float(elapsed), "counts": counts}
    assert len(samples) == 6, result.stdout + result.stderr
    return samples


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--elements", type=int, nargs="+", default=[10_000, 100_000])
    parser.add_argument("--rounds", type=int, default=7)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    assert args.rounds >= 3 and all(size > 0 for size in args.elements)
    binaries = {"reference": args.reference.resolve(), "candidate": args.candidate.resolve()}
    records, summaries = [], []
    for size in args.elements:
        # One unrecorded process per arm warms executable/filesystem pages.
        for binary in binaries.values():
            sample(binary, size)
        for round_number in range(args.rounds):
            order = ["reference", "candidate"] if round_number % 2 == 0 else ["candidate", "reference"]
            for arm in order:
                measured = sample(binaries[arm], size)
                records.append({"elements": size, "round": round_number, "arm": arm, "samples": measured})
        for case in records[-1]["samples"]:
            times = {
                arm: [r["samples"][case]["elapsed_us"] for r in records if r["elements"] == size and r["arm"] == arm]
                for arm in binaries
            }
            reference, candidate = (statistics.median(times[arm]) for arm in binaries)
            ratios = [b / a for a, b in zip(times["reference"], times["candidate"])]
            rng = random.Random(0)
            bootstrap = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(5000))
            summary = {
                "elements": size, "case": case,
                "reference_median_us": reference, "candidate_median_us": candidate,
                "median_paired_ratio": statistics.median(ratios),
                "paired_ratio_min": min(ratios), "paired_ratio_max": max(ratios),
                "paired_ratio_bootstrap_95": [bootstrap[125], bootstrap[4874]],
            }
            summaries.append(summary)
            print(f"{size:>7} {case:<25} {reference:>12.3f} -> {candidate:>12.3f} us  paired ratio {summary['median_paired_ratio']:.3f}", flush=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({
        "binaries": {key: str(value) for key, value in binaries.items()},
        "binary_sha256": {key: hashlib.sha256(value.read_bytes()).hexdigest() for key, value in binaries.items()},
        "host": {"platform": platform.platform(), "machine": platform.machine()},
        "rounds": args.rounds, "summary": summaries, "raw": records,
    }, indent=2) + "\n")


if __name__ == "__main__":
    main()
