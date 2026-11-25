#!/usr/bin/env python3
"""Recompute the published phase summaries from the unchanged benchmark CSVs.

Uses only Python's standard library. Values are medians of three process medians,
not a pooled sample distribution. The matched final dataset also supports the
median of three within-pair baseline/optimized ratios. Each process contributes
its maximum observed VmHWM; these are whole-process snapshots, not allocations.
"""

import argparse
import csv
import json
import math
from pathlib import Path
from statistics import median


WORKLOADS = ("geometry", "definitions", "text", "bitmap", "raster")
PHASES = ("record", "fresh_document", "cold", "warm", "multi_view", "no_dedup", "gzip")
RUNS = (1, 2, 3)
STRUCTURE_FIELDS = (
    "samples",
    "exports_per_operation",
    "output_bytes",
    "scene_layers",
    "scene_draw_calls",
    "exported_layers",
    "images",
    "definitions_deduplicated",
    "vector_cache_hits",
    "raster_cache_hits",
    "recording_bytes",
)


def read_runs(directory, workload):
    runs = []
    for run in RUNS:
        path = directory / f"{workload}-run{run}.csv"
        with path.open(newline="", encoding="utf-8") as stream:
            rows = list(csv.DictReader(stream))
        if len(rows) != len(PHASES) or {r["phase"] for r in rows} != set(PHASES):
            raise ValueError(f"Unexpected phases in {path}")
        if any(r["workload"] != workload for r in rows):
            raise ValueError(f"Unexpected workload in {path}")
        parsed = {}
        for row in rows:
            converted = {}
            for name, value in row.items():
                if name in ("workload", "phase"):
                    continue
                number = float(value) if name.endswith("_ms") else int(value)
                if not math.isfinite(number) or number < 0:
                    raise ValueError(f"Invalid {name} in {path}")
                converted[name] = number
            # This cache did not exist in the baseline. Do not rewrite its CSVs.
            converted.setdefault("prepared_vector_cache_hits", 0)
            parsed[row["phase"]] = converted
        runs.append(parsed)
    return runs


def latency_summary(rows):
    medians = [r["median_ms"] for r in rows]
    p95s = [r["p95_ms"] for r in rows]
    return {
        "median_ms": median(medians),
        "process_medians_ms": medians,
        "process_median_range_ms": [min(medians), max(medians)],
        "process_p95_ms": p95s,
        "process_p95_range_ms": [min(p95s), max(p95s)],
        "prepared_vector_cache_hits": rows[0]["prepared_vector_cache_hits"],
    }


def summarize(baseline, optimized):
    result = {
        "aggregation": "median of three process medians; per-process p95 ranges are not pooled p95",
        "paired_aggregation": "median of the three within-pair baseline/optimized median-latency ratios",
        "memory": "median of three per-process maximum observed VmHWM snapshots in KiB",
        "workloads": {},
    }
    for workload in WORKLOADS:
        before = read_runs(baseline, workload)
        after = read_runs(optimized, workload)
        phases = {}
        for phase in PHASES:
            left = [run[phase] for run in before]
            right = [run[phase] for run in after]
            for field in STRUCTURE_FIELDS:
                values = {row[field] for row in left + right}
                if len(values) != 1:
                    raise ValueError(f"Changed {field} for {workload}/{phase}: {values}")
            for rows in (left, right):
                if len({r["prepared_vector_cache_hits"] for r in rows}) != 1:
                    raise ValueError(f"Inconsistent prepared hits for {workload}/{phase}")
            old = latency_summary(left)
            new = latency_summary(right)
            paired_speedups = [
                before_row["median_ms"] / after_row["median_ms"]
                for before_row, after_row in zip(left, right)
            ]
            phases[phase] = {
                "baseline": old,
                "optimized": new,
                "speedup": old["median_ms"] / new["median_ms"],
                "latency_change_percent": (new["median_ms"] / old["median_ms"] - 1) * 100,
                "paired_speedups": paired_speedups,
                "median_paired_speedup": median(paired_speedups),
                "paired_speedup_range": [min(paired_speedups), max(paired_speedups)],
                "structure": {field: left[0][field] for field in STRUCTURE_FIELDS},
            }
        old_hwm = [max(row["peak_rss_kib"] for row in run.values()) for run in before]
        new_hwm = [max(row["peak_rss_kib"] for row in run.values()) for run in after]
        result["workloads"][workload] = {
            "phases": phases,
            "baseline_process_max_hwm_kib": old_hwm,
            "optimized_process_max_hwm_kib": new_hwm,
            "baseline_median_max_hwm_kib": median(old_hwm),
            "optimized_median_max_hwm_kib": median(new_hwm),
        }
    return result


def main():
    here = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, default=here / "baseline-paired")
    parser.add_argument("--optimized", type=Path, default=here / "optimized")
    parser.add_argument("--json", action="store_true", help="Print full summaries as JSON")
    args = parser.parse_args()
    result = summarize(args.baseline, args.optimized)
    if args.json:
        print(json.dumps(result, indent=2))
        return
    print("| Workload | Phase | Baseline ms | Optimized ms | Ratio of medians | Median paired speedup |")
    print("| --- | --- | ---: | ---: | ---: | ---: |")
    for workload, data in result["workloads"].items():
        for phase, values in data["phases"].items():
            print(
                f"| {workload} | {phase} | {values['baseline']['median_ms']:.4f} "
                f"| {values['optimized']['median_ms']:.4f} | {values['speedup']:.3f}× "
                f"| {values['median_paired_speedup']:.3f}× |"
            )


if __name__ == "__main__":
    main()
