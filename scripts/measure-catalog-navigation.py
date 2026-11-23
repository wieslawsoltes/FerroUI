#!/usr/bin/env python3
"""Compare compiled catalog executables using identical page visits, without screenshots.

Each page is followed by Home to control for outgoing-page teardown. Processes
alternate before/after order per round. Loaded measures UI readiness, not GPU
presentation. Raw logs and every observation are retained alongside the summary.
Invalid navigation can retry an entire pair with a fixed attempt limit; both
processes from a rejected pair are excluded from the observations.
"""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import time


PAGES = ["Buttons", "TextBox", "ComboBox", "TextBlock", "ColorPicker", "Calendar", "TreeDataGrid"]
TIMING = re.compile(r"Navigation timing: (.*?) \| (.*?) \| ([0-9.]+) ms$")


def fingerprint(path):
    digest = hashlib.sha256()
    with path.open("rb") as binary:
        for chunk in iter(lambda: binary.read(1024 * 1024), b""):
            digest.update(chunk)
    checkout = Path(__file__).resolve().parents[1]
    try:
        recorded_path = "<checkout>/" + path.relative_to(checkout).as_posix()
    except ValueError:
        recorded_path = "<executable>/" + path.name
    return dict(path=recorded_path, sha256=digest.hexdigest())


def launch(binary, label, round_number, args):
    sequence = [page for _ in range(args.visits) for target in args.pages for page in (target, "Home")]
    environment = dict(os.environ)
    # Do not inherit another smoke run's screenshot or page-selection settings.
    for key in list(environment):
        if key.startswith("FERROUI_SMOKE_"):
            del environment[key]
    # Allow dispatcher/timer drift without truncating the final return to Home.
    environment.update(
        FERROUI_CATALOG_NAV_TIMING="1",
        FERROUI_SMOKE_PAGE_SEQUENCE=",".join(sequence),
        FERROUI_SMOKE_PAGES=str(args.interval_ms),
        FERROUI_SMOKE_EXIT_MS=str((len(sequence) + 2) * args.interval_ms + 10_000),
        FERROUI_CATALOG_THEME="winui",
        FERROUI_CATALOG_THEME_VARIANT="light",
        FERROUI_SMOKE_RENDERING="metal",
    )
    path = args.output / f"{label}-{round_number}.log"
    process_metadata = dict(started_utc=datetime.now(timezone.utc).isoformat(),
                            load_average_start=os.getloadavg())
    print(f"Running {label}, round {round_number}: {binary}", flush=True)
    started = time.monotonic()
    with path.open("w") as log:
        subprocess.run([str(binary)], env=environment, stdout=log, stderr=subprocess.STDOUT,
                       timeout=(len(sequence) + 2) * args.interval_ms / 1000 + 70, check=True)
    process_metadata.update(elapsed_seconds=time.monotonic() - started,
                            load_average_end=os.getloadavg())
    path.with_suffix(".process.json").write_text(json.dumps(process_metadata, indent=2) + "\n")
    rows = []
    previous = None
    for line in path.read_text().splitlines():
        match = TIMING.fullmatch(line)
        if not match:
            continue
        page, phase, milliseconds = match.groups()
        if phase == "assets-ready":
            rows.append(dict(label=label, round=round_number, page=page, previous=previous))
            previous = page
        row = next(row for row in reversed(rows) if row["page"] == page)
        row[phase] = float(milliseconds)
    observed = [row["page"] for row in rows]
    if observed != ["Home", *sequence] or any("loaded" not in row for row in rows):
        raise RuntimeError(f"Unexpected/incomplete navigation in {path}: "
                           f"expected {len(sequence) + 1} complete visits, observed {len(rows)}; "
                           "inspect the log for timer drift or extra input")
    if "Timer fired: closing the main window" not in path.read_text():
        raise RuntimeError(f"Missing timed shutdown in {path}")
    measured = rows[1:]
    counts = {}
    for row in measured:
        key = (row["previous"], row["page"])
        counts[key] = counts.get(key, 0) + 1
        row["visit"] = counts[key]
    return measured


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before", required=True, type=Path)
    parser.add_argument("--after", required=True, type=Path)
    parser.add_argument("--output", type=Path, default=Path("target/navigation-comparison"))
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--visits", type=int, default=3)
    parser.add_argument("--interval-ms", type=int, default=800)
    parser.add_argument("--pages", default=",".join(PAGES), help="comma-separated subset of the standard pages")
    parser.add_argument("--warmups", type=int, default=0, help="unmeasured one-visit launches per executable")
    parser.add_argument("--max-pair-attempts", type=int, default=1,
                        help="retry a whole process pair after navigation validation fails")
    args = parser.parse_args()
    if min(args.rounds, args.visits, args.interval_ms, args.max_pair_attempts) < 1:
        parser.error("rounds, visits, interval and max-pair-attempts must be positive")
    if args.warmups < 0:
        parser.error("warmups must be nonnegative")
    canonical = {page.lower(): page for page in PAGES}
    try:
        args.pages = [canonical[page.strip().lower()] for page in args.pages.split(",")]
    except KeyError:
        parser.error("pages must be a comma-separated subset of: " + ", ".join(PAGES))
    if len(set(args.pages)) != len(args.pages):
        parser.error("pages must not contain duplicates")
    args.output.mkdir(parents=True, exist_ok=True)
    binaries = {"before": args.before.resolve(strict=True), "after": args.after.resolve(strict=True)}
    metadata = dict(started_utc=datetime.now(timezone.utc).isoformat(),
                    platform=platform.platform(), rounds=args.rounds, visits=args.visits, interval_ms=args.interval_ms,
                    theme="winui", variant="light", renderer="metal", pages=args.pages, warmups=args.warmups,
                    max_pair_attempts=args.max_pair_attempts,
                    binaries={label: fingerprint(path) for label, path in binaries.items()})
    (args.output / "run.json").write_text(json.dumps(metadata, indent=2) + "\n")
    for warmup in range(1, args.warmups + 1):
        warmup_args = argparse.Namespace(**vars(args))
        warmup_args.visits = 1
        for label, binary in binaries.items():
            launch(binary, label + "-warmup", warmup, warmup_args)
    observations = []
    discarded = []
    for round_number in range(1, args.rounds + 1):
        order = ["before", "after"] if round_number % 2 else ["after", "before"]
        for attempt in range(1, args.max_pair_attempts + 1):
            pair_args = argparse.Namespace(**vars(args))
            pair_args.output = args.output / f"round-{round_number}-attempt-{attempt}"
            pair_args.output.mkdir(parents=True, exist_ok=True)
            pair = []
            try:
                for label in order:
                    pair.extend(launch(binaries[label], label, round_number, pair_args))
            except RuntimeError as error:
                discarded.append(dict(round=round_number, attempt=attempt, reason=str(error)))
                (args.output / "discarded.json").write_text(json.dumps(discarded, indent=2) + "\n")
                print(f"Discarding round {round_number}, attempt {attempt}: {error}", flush=True)
                if attempt == args.max_pair_attempts:
                    raise
                continue
            for visit in pair:
                visit["attempt"] = attempt
            # Publish observations only after both processes pass validation.
            observations.extend(pair)
            (args.output / "observations.json").write_text(json.dumps(observations, indent=2) + "\n")
            break
    summary = []
    for previous, page in [("Home", page) for page in args.pages] + [(page, "Home") for page in args.pages]:
        row = dict(previous=previous, page=page)
        for label in binaries:
            visits = [visit for visit in observations
                      if visit["label"] == label and visit["page"] == page and visit["previous"] == previous]
            row[label] = {phase: statistics.median(visit[phase] for visit in visits)
                          for phase in ["constructed", "host-returned", "loaded"]}
            row[label]["samples"] = len(visits)
        row["loaded_change_percent"] = 100 * (row["after"]["loaded"] / row["before"]["loaded"] - 1)
        row["rounds"] = []
        for round_number in range(1, args.rounds + 1):
            pair = {label: statistics.median(visit["loaded"] for visit in observations
                    if visit["label"] == label and visit["round"] == round_number
                    and visit["previous"] == previous and visit["page"] == page) for label in binaries}
            row["rounds"].append(dict(round=round_number, **pair,
                loaded_change_percent=100 * (pair["after"] / pair["before"] - 1)))
        summary.append(row)
        print(f"{previous} -> {page}: {row['before']['loaded']:.2f} -> "
              f"{row['after']['loaded']:.2f} ms ({row['loaded_change_percent']:+.1f}%)")
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")


if __name__ == "__main__":
    main()
