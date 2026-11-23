#!/usr/bin/env python3
"""Paired fresh-process resource/WinUI startup measurements.

Use uninstrumented release binaries for timings and count-allocations binaries
for allocation evidence. Readiness is observed before child teardown begins.
"""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import random
import statistics
import subprocess
import time


def sample(binary, scenario, count):
    started = time.perf_counter_ns()
    # Unbuffered binary reads keep readline from consuming result bytes that
    # communicate() must collect after the readiness marker.
    process = subprocess.Popen([str(binary), scenario, str(count)], bufsize=0,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    lines = []
    ready = None
    while True:
        line = process.stdout.readline()
        if not line:
            break
        if line.strip() == b"RESOURCE_BENCH_READY":
            ready = (time.perf_counter_ns() - started) / 1000
            break
        lines.append(line)
    rest, errors = process.communicate(timeout=180)
    if process.returncode or ready is None:
        raise RuntimeError(f"{scenario}/{count}: {process.returncode}\n{b''.join(lines)+rest!r}\n{errors!r}")
    record = json.loads(rest.strip().splitlines()[-1])
    record["launch_to_verified_us"] = ready
    return record


def interval(ratios):
    rng = random.Random(0)
    samples = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(5000))
    return [samples[125], samples[4874]]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--rounds", type=int, default=21)
    parser.add_argument("--rows", type=int, nargs="+", default=[1, 100, 1000, 5000])
    parser.add_argument("--scenarios", nargs="+", default=["literal", "alias", "theme_button", "theme_full"])
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    assert args.rounds >= 3 and all(n > 0 for n in args.rows)
    binaries = {"reference": args.reference.resolve(), "candidate": args.candidate.resolve()}
    records, summaries = [], []
    for scenario in args.scenarios:
        for count in args.rows if scenario in ("literal", "alias") else [1]:
            for binary in binaries.values():
                sample(binary, scenario, count)
            runs = {arm: [] for arm in binaries}
            for round_number in range(args.rounds):
                order = ["reference", "candidate"] if round_number % 2 == 0 else ["candidate", "reference"]
                for arm in order:
                    result = sample(binaries[arm], scenario, count)
                    runs[arm].append(result)
                    records.append({"scenario":scenario, "rows":count, "round":round_number, "arm":arm, "result":result})
            metrics = ["process_main_to_verified_us", "launch_to_verified_us"]
            metrics += [entry["phase"] for entry in runs["reference"][0]["samples"]]
            for metric in metrics:
                def values(arm):
                    return [result[metric] if metric.endswith("_us") else
                            next(p["elapsed_us"] for p in result["samples"] if p["phase"] == metric)
                            for result in runs[arm]]
                before, after = values("reference"), values("candidate")
                ratios = [b/a for a,b in zip(before,after)]
                row = {"scenario":scenario,"rows":count,"metric":metric,
                       "reference_median_us":statistics.median(before),"candidate_median_us":statistics.median(after),
                       "paired_ratio":statistics.median(ratios),"paired_ratio_bootstrap_95":interval(ratios)}
                if not metric.endswith("_us"):
                    for arm in binaries:
                        phases = [next(p for p in result["samples"] if p["phase"] == metric) for result in runs[arm]]
                        for field in ("allocations", "allocated_bytes"):
                            measured = [p[field] for p in phases]
                            row[f"{arm}_{field}"] = statistics.median(measured) if measured[0] is not None else None
                summaries.append(row)
                print(f"{scenario:12} {count:5} {metric:30} {row['reference_median_us']:10.2f} -> {row['candidate_median_us']:10.2f} us  ratio={row['paired_ratio']:.3f}",flush=True)
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps({
        "rounds":args.rounds,"host":platform.platform(),
        "binaries":{arm:str(binary) for arm,binary in binaries.items()},
        "binary_sha256":{arm:hashlib.sha256(binary.read_bytes()).hexdigest() for arm,binary in binaries.items()},
        "summary":summaries,"raw":records,
    },indent=2)+'\n')


if __name__ == "__main__":
    main()
