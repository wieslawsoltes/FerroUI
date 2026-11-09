#!/usr/bin/env python3
"""Interleave main and candidate release executables for generated selector indexes.

Build the same selectors.rs harness on both revisions. Time uninstrumented
binaries; use count-allocations/perf-counters builds separately for work counts.
Results are written only to the explicitly supplied local output path.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import random
import statistics
import subprocess


def sample(binary, rules, percent, mode, elements):
    result = subprocess.run([str(binary), str(rules), str(percent), mode, str(elements)],
                            capture_output=True, text=True, timeout=180, check=True)
    return json.loads(result.stdout.strip().splitlines()[-1])


def interval(ratios):
    rng = random.Random(0)
    samples = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(5000))
    return [samples[125], samples[4874]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--rounds", type=int, default=21)
    parser.add_argument("--rules", type=int, nargs="+", default=[0, 1, 10, 100, 1000])
    parser.add_argument("--percent", type=int, nargs="+", default=[0, 10, 100])
    parser.add_argument("--modes", nargs="+", default=["typed", "dynamic", "fallback"])
    parser.add_argument("--elements", type=int, default=1000)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    assert args.rounds >= 3 and args.elements > 0
    binaries = {"reference":args.reference.resolve(), "candidate":args.candidate.resolve()}
    records, summary = [], []
    for rules in args.rules:
        for percent in args.percent if rules else [0]:
            for mode in args.modes:
                if mode == "fallback" and percent != args.percent[0]:
                    continue  # No type bounds: the matching percentage does not apply.
                for binary in binaries.values():
                    sample(binary, rules, percent, mode, args.elements)
                runs = {arm:[] for arm in binaries}
                for round_number in range(args.rounds):
                    order = ["reference", "candidate"] if round_number % 2 == 0 else ["candidate", "reference"]
                    for arm in order:
                        record = sample(binaries[arm], rules, percent, mode, args.elements)
                        records.append({"arm":arm,"round":round_number,"result":record})
                        runs[arm].append(record)
                for phase in [p["phase"] for p in runs["reference"][0]["samples"]]:
                    phases = {arm:[next(p for p in r["samples"] if p["phase"] == phase) for r in results]
                              for arm, results in runs.items()}
                    before = [p["elapsed_us"] for p in phases["reference"]]
                    after = [p["elapsed_us"] for p in phases["candidate"]]
                    ratios = [b/a for a,b in zip(before,after)]
                    row = {"rules":rules,"matching_percent":percent,"mode":mode,"phase":phase,
                           "reference_median_us":statistics.median(before),"candidate_median_us":statistics.median(after),
                           "paired_ratio":statistics.median(ratios),"paired_ratio_bootstrap_95":interval(ratios)}
                    for arm in phases:
                        for metric in ["allocations", "allocated_bytes", "styles_evaluated"]:
                            values = [p[metric] for p in phases[arm]]
                            row[f"{arm}_{metric}"] = statistics.median(values) if values[0] is not None else None
                    summary.append(row)
                    print(f"{rules:5} {percent:3}% {mode:8} {phase:34} {row['reference_median_us']:10.2f} -> {row['candidate_median_us']:10.2f} us  ratio={row['paired_ratio']:.3f}", flush=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps({"rounds":args.rounds,"host":platform.platform(),
        "binaries":{arm:str(binary) for arm,binary in binaries.items()},
        "binary_sha256":{arm:hashlib.sha256(binary.read_bytes()).hexdigest() for arm,binary in binaries.items()},
        "summary":summary,"raw":records}, indent=2)+'\n')


if __name__ == "__main__":
    main()
