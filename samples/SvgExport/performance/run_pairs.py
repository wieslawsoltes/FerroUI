#!/usr/bin/env python3
"""Run the matched Linux comparison using two preserved Release executables.

Each workload has three pairs of fresh processes. Execution order reverses in
the second repetition. CSVs are preserved exactly as emitted by export_bench.
Run without simultaneous builds, renderers or validation jobs.
"""

import argparse
import csv
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import subprocess
import time


WORKLOADS = ("geometry", "definitions", "text", "bitmap", "raster")
PHASES = {"record", "fresh_document", "cold", "warm", "multi_view", "no_dedup", "gzip"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-executable", required=True, type=Path)
    parser.add_argument("--optimized-executable", required=True, type=Path)
    parser.add_argument("--baseline-revision", required=True)
    parser.add_argument("--optimized-revision", required=True)
    parser.add_argument("--results", required=True, type=Path, help="New output directory")
    parser.add_argument("--cpu", default=0, type=int)
    args = parser.parse_args()
    binaries = {
        "baseline": args.baseline_executable.resolve(strict=True),
        "optimized": args.optimized_executable.resolve(strict=True),
    }
    revisions = {
        "baseline": args.baseline_revision,
        "optimized": args.optimized_revision,
    }
    if any(not re.fullmatch(r"[0-9a-f]{40}", value) for value in revisions.values()):
        parser.error("Source revisions must be full 40-character commit hashes")
    if args.cpu < 0:
        parser.error("CPU index must be nonnegative")
    args.results.mkdir(parents=True, exist_ok=False)
    metadata = {
        "method": "Three paired processes per workload; baseline first in runs 1/3, optimized first in run 2",
        "cpu_affinity": args.cpu,
        "warm_samples": 40,
        "cold_samples": 8,
        "warmups": 5,
        "binaries": {},
        "processes": [],
    }
    for variant, binary in binaries.items():
        (args.results / variant).mkdir()
        with binary.open("rb") as stream:
            fingerprint = hashlib.file_digest(stream, "sha256").hexdigest()
        metadata["binaries"][variant] = {
            "revision": revisions[variant],
            "sha256": fingerprint,
            "bytes": binary.stat().st_size,
        }
    for run in (1, 2, 3):
        order = ("baseline", "optimized") if run % 2 else ("optimized", "baseline")
        for workload in WORKLOADS:
            for position, variant in enumerate(order):
                directory = args.results / variant
                output = directory / f"{workload}-run{run}.csv"
                log = output.with_suffix(".log")
                command = [
                    "taskset", "-c", str(args.cpu), str(binaries[variant]),
                    "--only", workload, "--iterations", "40",
                    "--cold-iterations", "8", "--warmup", "5",
                    "--output", str(output),
                ]
                started = datetime.now(timezone.utc).isoformat()
                begin = time.monotonic()
                with log.open("w", encoding="utf-8") as stream:
                    subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, check=True)
                elapsed = time.monotonic() - begin
                with output.open(newline="", encoding="utf-8") as stream:
                    rows = list(csv.DictReader(stream))
                if len(rows) != len(PHASES) or {r["phase"] for r in rows} != PHASES:
                    raise ValueError(f"Unexpected phase output in {output}")
                if {r["workload"] for r in rows} != {workload}:
                    raise ValueError(f"Unexpected workload output in {output}")
                metadata["processes"].append({
                    "run": run,
                    "workload": workload,
                    "variant": variant,
                    "position": position,
                    "started_utc": started,
                    "wall_seconds": elapsed,
                    "csv": str(output.relative_to(args.results)),
                    "command": command,
                })
                (args.results / "metadata.json").write_text(
                    json.dumps(metadata, indent=2) + "\n", encoding="utf-8"
                )
                print(f"run {run}: {variant} {workload} complete ({elapsed:.2f}s)", flush=True)


if __name__ == "__main__":
    main()
