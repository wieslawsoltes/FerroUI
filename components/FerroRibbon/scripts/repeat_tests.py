#!/usr/bin/env python3
"""Run the ribbon test binary several times and report every result.

Each full run and each run of the order-dependent pair is reported on its own. Nothing is retried
and a failing run does not stop the next one: the point is how often a test fails. The script
fails only when it cannot run the binary at all.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

COMPONENT = Path(__file__).resolve().parents[1]
PACKAGE = COMPONENT / "src" / "FerroRibbon"
# The earlier test leaves the state in which the later one showed a retained element.
PAIR = (
    "backstage_native_animation_uses_source_durations_and_cubic_easing",
    "removed_header_cleanup_can_reenter_tab_collection_after_cache_is_published",
)
WATCHED = (
    "removed_header_cleanup_can_reenter_tab_collection_after_cache_is_published",
    "native_shared_root_handling_precedence_cache_release_and_group_mutations",
)
RESULT = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed")
FAILED = re.compile(r"^test (\S+) \.\.\. FAILED$")


def build() -> str:
    command = ["cargo", "test", "--locked", "-p", "ferroribbon", "--lib", "--no-run", "--message-format=json"]
    output = subprocess.run(command, cwd=COMPONENT, check=True, stdout=subprocess.PIPE, text=True).stdout
    executables = [message["executable"] for message in map(json.loads, filter(None, output.splitlines()))
                   if message.get("reason") == "compiler-artifact" and message.get("executable")
                   and message["target"]["name"] == "ferroribbon" and message["profile"]["test"]]
    if len(executables) != 1:
        raise SystemExit(f"expected one test binary, found {executables}")
    return executables[0]


def environment() -> dict[str, str]:
    return {**os.environ, "CARGO_MANIFEST_DIR": str(PACKAGE), "CARGO_PKG_NAME": "ferroribbon"}


def run(executable: str, arguments: list[str], limit: int) -> dict:
    try:
        result = subprocess.run([executable, *arguments], cwd=PACKAGE, env=environment(), timeout=limit,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    except subprocess.TimeoutExpired:
        return {"status": "timeout", "passed": 0, "failed": 0, "failures": []}
    lines = result.stdout.splitlines()
    summary = [match for match in map(RESULT.match, lines) if match]
    failures = sorted({match.group(1) for match in map(FAILED.match, lines) if match})
    return {
        "status": f"exit {result.returncode}",
        "passed": int(summary[-1].group(2)) if summary else 0,
        "failed": int(summary[-1].group(3)) if summary else 0,
        "failures": failures,
    }


def full_name(names: list[str], suffix: str) -> str:
    found = [name for name in names if name.rsplit("::", 1)[-1] == suffix]
    if len(found) != 1:
        raise SystemExit(f"expected one test named {suffix}, found {found}")
    return found[0]


def short(name: str) -> str:
    return name.rsplit("::", 1)[-1]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", help="the test binary; built with cargo when it is not given")
    parser.add_argument("--runs", type=int, default=6)
    parser.add_argument("--pair-runs", type=int, default=6)
    parser.add_argument("--timeout", type=int, default=900, help="seconds for one run")
    parser.add_argument("--json", type=Path, help="also write the results to this file")
    args = parser.parse_args()

    executable = args.executable or build()
    listing = subprocess.run([executable, "--list"], cwd=PACKAGE, env=environment(), check=True,
                             stdout=subprocess.PIPE, text=True).stdout
    names = [line[:-6] for line in listing.splitlines() if line.endswith(": test")]
    pair = [full_name(names, suffix) for suffix in PAIR]
    watched = [full_name(names, suffix) for suffix in WATCHED]

    report = ["## Repeated ribbon tests", "", "| Run | Result | Passed | Failed | Failing tests |", "|---|---|---:|---:|---|"]
    results: dict[str, list[dict]] = {"full": [], "pair": []}
    for label, count, arguments in (("full", args.runs, []),
                                    ("pair", args.pair_runs, ["--exact", *pair, "--test-threads=1"])):
        for index in range(1, count + 1):
            result = run(executable, arguments, args.timeout)
            results[label].append(result)
            row = (f"| {label} {index} | {result['status']} | {result['passed']} | {result['failed']} | "
                   f"{', '.join(short(name) for name in result['failures']) or 'none'} |")
            report.append(row)
            print(row, flush=True)

    report.extend(["", "| Test | Failed in full runs | Failed in pair runs |", "|---|---:|---:|"])
    for name in watched:
        full = sum(name in result["failures"] for result in results["full"])
        in_pair = sum(name in result["failures"] for result in results["pair"])
        pair_text = f"{in_pair} of {args.pair_runs}" if name in pair else "not in the pair"
        report.append(f"| `{short(name)}` | {full} of {args.runs} | {pair_text} |")
    text = "\n".join(report) + "\n"
    print(text)
    if args.json:
        args.json.write_text(json.dumps(results, indent=2) + "\n")
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as handle:
            handle.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
