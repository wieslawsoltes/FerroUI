#!/usr/bin/env python3
"""Binary size and startup time of the three reference applications.

Builds `hello_window`, `themed_window` and `control-catalog-desktop` with the
release profile (or, with `--profile dist`, with the size-optimised profile
the published numbers are measured with), then prints for each the size of the stripped executable and
the time from process start to the first window (median of several warm
launches, with the load average of the machine before and after them; all
applications are built first, the launches wait for the load of the machine
to settle and go round the applications).

With `--baseline-target <dir>` the executables of an earlier build in that
cargo target directory are launched too, alternately with the measured ones,
and the two are compared: the fair comparison of start-up times, since both
see the same machine. With `--check <file>` the numbers are compared with
numbers stored by `--save <file>` instead. Either comparison fails the script
when a number regressed by more than its tolerance, unless `--no-fail` is
given. `--markdown <file>` appends the tables and the comparison as Markdown,
for example to `$GITHUB_STEP_SUMMARY`.

    python3 scripts/perf-report.py
    python3 scripts/perf-report.py --save target/perf-baseline.json
    python3 scripts/perf-report.py --check target/perf-baseline.json
    python3 scripts/perf-report.py --size-only
    python3 scripts/perf-report.py --profile dist
    python3 scripts/perf-report.py --source ../baseline --size-only   # builds ../baseline
    python3 scripts/perf-report.py --baseline-target ../baseline/target

Only the Python standard library, `cargo` and `strip` are used. The startup
figures depend on the load of the machine and are only comparable between
runs made on the same idle machine; the sizes are exact. Measuring startup
needs a desktop session in which the applications can open a window (only
macOS has a windowing backend); `--size-only` builds and measures the sizes
alone and works on every operating system the applications build on.
"""

import argparse
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import threading
import time

# The checkout that is built; `--source` selects another one.
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The cargo profile that is built and measured; `--profile` selects another one.
PROFILE = "release"

# name, cargo arguments, path under the profile directory, arguments, line that marks the first window
APPLICATIONS = [
    ("hello_window", ["-p", "ferroui-desktop", "--example", "hello_window"], "examples/hello_window", [], r"Window opened"),
    (
        "themed_window (Simple)",
        ["-p", "ferroui-themes-simple", "--example", "themed_window"],
        "examples/themed_window",
        [],
        r"Window opened",
    ),
    (
        "themed_window (Fluent)",
        ["-p", "ferroui-themes-simple", "--example", "themed_window"],
        "examples/themed_window",
        ["--fluent"],
        r"Window opened",
    ),
    ("control-catalog-desktop", ["-p", "control-catalog-desktop"], "control-catalog-desktop", [], r"App activated"),
]


def target_directory():
    return os.environ.get("CARGO_TARGET_DIR") or os.path.join(ROOT, "target")


def build(cargo_arguments):
    command = ["cargo", "build", "--profile", PROFILE, "--locked"] + cargo_arguments
    print("$ " + " ".join(command), flush=True)
    subprocess.run(command, cwd=ROOT, check=True)


def stripped_copy(path, directory):
    copy = os.path.join(directory, os.path.basename(path))
    if not os.path.exists(copy):
        shutil.copy2(path, copy)
        subprocess.run(["strip", copy], check=True, stderr=subprocess.DEVNULL)
        if sys.platform == "darwin":
            # Stripping invalidates the ad-hoc signature the linker made.
            subprocess.run(["codesign", "-s", "-", "-f", copy], check=False, stderr=subprocess.DEVNULL)
    return copy


def load_average():
    return round(os.getloadavg()[0], 2) if hasattr(os, "getloadavg") else None


def time_to_marker(command, marker, exit_ms):
    """Milliseconds from the start of the process to the first output line that matches `marker`.

    The process is killed when it is still running well after it should have
    closed its window, and the launch then counts as failed.
    """
    environment = dict(os.environ, FERROUI_SMOKE_EXIT_MS=str(exit_ms))
    pattern = re.compile(marker)
    start = time.monotonic()
    process = subprocess.Popen(command, env=environment, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
    killed = threading.Event()

    def kill():
        killed.set()
        process.kill()

    watchdog = threading.Timer(exit_ms / 1000.0 + 60.0, kill)
    watchdog.start()
    elapsed = None
    line = b""
    while True:
        byte = process.stdout.read(1)
        if not byte:
            break
        line += byte
        if byte == b"\n":
            if elapsed is None and pattern.search(line.decode(errors="replace")):
                elapsed = (time.monotonic() - start) * 1000.0
            line = b""
    process.wait()
    watchdog.cancel()
    return None if killed.is_set() else elapsed


def settle(max_seconds, threshold):
    """Waits (at most `max_seconds`) until the one-minute load average is below `threshold`."""
    if not hasattr(os, "getloadavg"):
        return
    deadline = time.monotonic() + max_seconds
    if os.getloadavg()[0] > threshold:
        print(f"waiting for the load average ({os.getloadavg()[0]:.2f}) to drop below {threshold}", flush=True)
    while os.getloadavg()[0] > threshold and time.monotonic() < deadline:
        time.sleep(5)


def locate(target, directory, label):
    """Stripped copies of the executables under the target directory `target`, in `directory`/`label`."""
    copies = os.path.join(directory, label)
    os.makedirs(copies, exist_ok=True)
    found = {}
    for name, _, relative_path, arguments, marker in APPLICATIONS:
        executable = os.path.join(target, PROFILE, relative_path)
        if not os.path.exists(executable):
            sys.exit(f"{executable} does not exist; build it first (or run without --no-build)")
        stripped = stripped_copy(executable, copies)
        found[name] = (stripped, [stripped] + arguments, marker)
    return found


def measure(runs, exit_ms, do_build, size_only, settle_seconds, settle_load, baseline_target=None):
    """Sizes and start-up times of the applications, as {set: {application: numbers}}.

    The set "measured" is the build of the checkout; with `baseline_target`,
    the set "baseline" is the build already in that target directory, and
    the launches of the two alternate.
    """
    with tempfile.TemporaryDirectory(prefix="ferroui-perf-") as directory:
        # Everything is built before anything is launched, so that no launch
        # runs on a machine that is still busy with a build.
        if do_build:
            built = []
            for _, cargo_arguments, _, _, _ in APPLICATIONS:
                if cargo_arguments not in built:
                    build(cargo_arguments)
                    built.append(cargo_arguments)
        sets = {}
        if baseline_target:
            sets["baseline"] = locate(baseline_target, directory, "baseline")
        sets["measured"] = locate(target_directory(), directory, "measured")
        results = {
            label: {name: {"size_bytes": os.path.getsize(stripped)} for name, (stripped, _, _) in found.items()}
            for label, found in sets.items()
        }
        if size_only:
            return results

        names = [name for name, _, _, _, _ in APPLICATIONS]
        labels = list(sets)
        settle(settle_seconds, settle_load)
        load_before = load_average()
        # The first launch of a new executable pays for the validation of its
        # code signature; it is measured once per file and kept apart.
        first = {label: {} for label in labels}
        launched = set()
        for name in names:
            for label in labels:
                stripped, command, marker = sets[label][name]
                elapsed = time_to_marker(command, marker, exit_ms)
                first[label][name] = None if stripped in launched or elapsed is None else round(elapsed, 1)
                launched.add(stripped)
                time.sleep(0.2)
        # The warm launches go round the applications, so that a change of
        # the load of the machine affects all of them alike; the baseline and
        # the measured build of an application are launched one after the
        # other, in alternating order.
        times = {label: {name: [] for name in names} for label in labels}
        for run in range(runs):
            for name in names:
                for label in labels if run % 2 == 0 else reversed(labels):
                    _, command, marker = sets[label][name]
                    elapsed = time_to_marker(command, marker, exit_ms)
                    if elapsed is not None:
                        times[label][name].append(elapsed)
                    time.sleep(0.2)
        load_after = load_average()
        for label in labels:
            for name in names:
                marker = sets[label][name][2]
                measured = times[label][name]
                if len(measured) < runs:
                    sys.exit(f"{name} ({label}): the line /{marker}/ was printed in {len(measured)} of {runs} launches")
                results[label][name].update(
                    {
                        "startup_ms": round(statistics.median(measured), 1),
                        "startup_min_ms": round(min(measured), 1),
                        "startup_max_ms": round(max(measured), 1),
                        "runs": runs,
                        "first_launch_ms": first[label][name],
                        "load_average": [load_before, load_after],
                    }
                )
    return results


def size_text(row):
    return f"{row['size_bytes'] / 1e6:.2f} MB"


def startup_cells(row):
    if "startup_ms" not in row:
        return ["-", "-", "-", "-"]
    first = "-" if row["first_launch_ms"] is None else f"{row['first_launch_ms']:.0f} ms"
    load = " / ".join("-" if value is None else f"{value:.2f}" for value in row["load_average"])
    return [
        f"{row['startup_ms']:.0f} ms",
        f"{row['startup_min_ms']:.0f} to {row['startup_max_ms']:.0f} ms",
        first,
        load,
    ]


HEADER = ["application", "stripped size", "startup (median)", "range", "first launch", "load average (before / after)"]


def print_table(results):
    rows = [[name, size_text(row)] + startup_cells(row) for name, row in results.items()]
    widths = [max(len(cell) for cell in column) for column in zip(HEADER, *rows)]
    print()
    for row in [HEADER] + rows:
        print("  ".join(cell.ljust(width) if i == 0 else cell.rjust(width) for i, (cell, width) in enumerate(zip(row, widths))))


def markdown_table(results, title):
    lines = [f"### {title}", "", "| " + " | ".join(HEADER) + " |", "|" + "---|" + "---:|" * (len(HEADER) - 1)]
    for name, row in results.items():
        lines.append("| " + " | ".join([name, size_text(row)] + startup_cells(row)) + " |")
    lines.append("")
    return lines


def comparison(results, baseline, size_tolerance, time_tolerance):
    """Rows of (name, quantity, before, after, change in percent, regressed)."""
    rows = []
    for name, row in results.items():
        reference = baseline.get(name)
        if reference is None:
            continue
        for key, tolerance in (("size_bytes", size_tolerance), ("startup_ms", time_tolerance)):
            if key not in reference or key not in row:
                continue
            before, after = reference[key], row[key]
            change = (after - before) / before * 100.0
            rows.append((name, key, before, after, change, change > tolerance))
    return rows


def quantity_text(key, value):
    return f"{value / 1e6:.2f} MB" if key == "size_bytes" else f"{value:.0f} ms"


def print_comparison(rows):
    print()
    for name, key, before, after, change, regressed in rows:
        status = "REGRESSION" if regressed else "ok"
        print(
            f"{name:28} {key:12} {quantity_text(key, before):>11} -> {quantity_text(key, after):>11} {change:+6.1f}%  {status}"
        )


def markdown_comparison(rows, title):
    lines = [f"### {title}", "", "| application | quantity | before | after | change |", "|---|---|---:|---:|---:|"]
    for name, key, before, after, change, regressed in rows:
        quantity = "stripped size" if key == "size_bytes" else "startup (median)"
        mark = " (regression)" if regressed else ""
        lines.append(
            f"| {name} | {quantity} | {quantity_text(key, before)} | {quantity_text(key, after)} | {change:+.1f} %{mark} |"
        )
    lines.append("")
    return lines


def describe_machine():
    parts = [sys.platform]
    try:
        if sys.platform == "darwin":
            parts.append(subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip())
            parts.append("macOS " + subprocess.run(["sw_vers", "-productVersion"], capture_output=True, text=True).stdout.strip())
        parts.append(subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip())
    except OSError:
        pass
    parts.append(f"{os.cpu_count()} logical CPUs")
    return ", ".join(part for part in parts if part)


def main():
    global PROFILE, ROOT
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--runs", type=int, default=7, help="warm launches per application (default 7)")
    parser.add_argument("--exit-ms", type=int, default=1500, help="how long each launch stays open (default 1500)")
    parser.add_argument("--no-build", action="store_true", help="measure the executables that are already built")
    parser.add_argument("--source", metavar="DIR", help="build and measure this checkout (default: the one of the script)")
    parser.add_argument(
        "--profile",
        default=PROFILE,
        help="cargo profile to build and measure: release (default, set up for build time) or dist (whole-program "
        "optimisation, the smallest executables); --baseline-target is read with the same profile",
    )
    parser.add_argument("--size-only", action="store_true", help="measure the sizes only; launch nothing")
    parser.add_argument("--save", metavar="FILE", help="store the numbers as a baseline")
    parser.add_argument("--check", metavar="FILE", help="compare with a stored baseline")
    parser.add_argument(
        "--baseline-target",
        metavar="DIR",
        help="cargo target directory of a baseline build; its executables are launched alternately with the "
        "measured ones and compared with them",
    )
    parser.add_argument("--save-baseline", metavar="FILE", help="store the numbers of --baseline-target")
    parser.add_argument("--markdown", metavar="FILE", help="append the table (and the comparison) as Markdown")
    parser.add_argument("--title", default="Size and startup", help="heading of the Markdown table")
    parser.add_argument("--settle-seconds", type=int, default=300, help="longest wait for the load to settle (default 300)")
    parser.add_argument("--settle-load", type=float, default=1.5, help="load average to wait for before launching (default 1.5)")
    parser.add_argument("--no-fail", action="store_true", help="report regressions without failing")
    parser.add_argument("--size-tolerance", type=float, default=1.0, help="allowed growth of a size in percent (default 1)")
    parser.add_argument("--time-tolerance", type=float, default=15.0, help="allowed growth of a time in percent (default 15)")
    options = parser.parse_args()
    if options.runs < 1:
        parser.error("--runs must be at least 1")
    PROFILE = options.profile
    if options.source:
        ROOT = os.path.abspath(options.source)

    measured = measure(
        options.runs,
        options.exit_ms,
        not options.no_build,
        options.size_only,
        options.settle_seconds,
        options.settle_load,
        options.baseline_target,
    )
    results = measured["measured"]
    machine = describe_machine()
    markdown = []
    baseline = None
    baseline_name = None
    if "baseline" in measured:
        baseline = measured["baseline"]
        baseline_name = "the baseline build"
        print("\nbaseline (" + options.baseline_target + ")", end="")
        print_table(baseline)
        markdown += markdown_table(baseline, f"{options.title}: baseline")
        if options.save_baseline:
            with open(options.save_baseline, "w") as file:
                json.dump(baseline, file, indent=2)
                file.write("\n")
        print("\nmeasured", end="")
    elif options.check:
        with open(options.check) as file:
            baseline = json.load(file)
        baseline_name = os.path.basename(options.check)
    print_table(results)
    print(machine)
    markdown += markdown_table(results, options.title) + [machine, ""]

    failed = False
    if baseline is not None:
        rows = comparison(results, baseline, options.size_tolerance, options.time_tolerance)
        print_comparison(rows)
        markdown += markdown_comparison(rows, f"{options.title}: change against {baseline_name}")
        failed = any(row[5] for row in rows)

    if options.save:
        with open(options.save, "w") as file:
            json.dump(results, file, indent=2)
            file.write("\n")
    if options.markdown:
        with open(options.markdown, "a") as file:
            file.write("\n".join(markdown) + "\n")
    if failed and not options.no_fail:
        sys.exit(1)


if __name__ == "__main__":
    main()
