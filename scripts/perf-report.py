#!/usr/bin/env python3
"""Binary size and startup time of the three reference applications.

Builds `hello_window`, `themed_window` and `control-catalog-desktop` with the
release profile, then prints for each the size of the stripped executable and
the time from process start to the first window (median of several warm
launches, with the load average of the machine before and after them). With
`--check <file>` the numbers are compared with a stored baseline and the
script fails when one regressed by more than the tolerance; `--save <file>`
stores the numbers as a baseline; `--markdown <file>` appends the table (and
the comparison) as Markdown, for example to `$GITHUB_STEP_SUMMARY`.

    python3 scripts/perf-report.py
    python3 scripts/perf-report.py --save target/perf-baseline.json
    python3 scripts/perf-report.py --check target/perf-baseline.json
    python3 scripts/perf-report.py --size-only
    python3 scripts/perf-report.py --source ../baseline --save baseline.json

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
    command = ["cargo", "build", "--release", "--locked"] + cargo_arguments
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


def measure(runs, exit_ms, do_build, size_only):
    results = {}
    with tempfile.TemporaryDirectory(prefix="ferroui-perf-") as directory:
        for name, cargo_arguments, relative_path, arguments, marker in APPLICATIONS:
            if do_build:
                build(cargo_arguments)
            executable = os.path.join(target_directory(), "release", relative_path)
            if not os.path.exists(executable):
                sys.exit(f"{executable} does not exist; run without --no-build")
            stripped = stripped_copy(executable, directory)
            row = {"size_bytes": os.path.getsize(stripped)}
            if not size_only:
                load_before = load_average()
                # The first launch of a new executable pays for the validation of its code signature.
                cold = time_to_marker([stripped] + arguments, marker, exit_ms)
                times = []
                for _ in range(runs):
                    elapsed = time_to_marker([stripped] + arguments, marker, exit_ms)
                    if elapsed is not None:
                        times.append(elapsed)
                    time.sleep(0.2)
                if len(times) < runs:
                    sys.exit(f"{name}: the line /{marker}/ was printed in {len(times)} of {runs} launches")
                row.update(
                    {
                        "startup_ms": round(statistics.median(times), 1),
                        "startup_min_ms": round(min(times), 1),
                        "startup_max_ms": round(max(times), 1),
                        "runs": runs,
                        "first_launch_ms": None if cold is None else round(cold, 1),
                        "load_average": [load_before, load_average()],
                    }
                )
            results[name] = row
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
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--runs", type=int, default=7, help="warm launches per application (default 7)")
    parser.add_argument("--exit-ms", type=int, default=1500, help="how long each launch stays open (default 1500)")
    parser.add_argument("--no-build", action="store_true", help="measure the executables that are already built")
    parser.add_argument("--source", metavar="DIR", help="build and measure this checkout (default: the one of the script)")
    parser.add_argument("--size-only", action="store_true", help="measure the sizes only; launch nothing")
    parser.add_argument("--save", metavar="FILE", help="store the numbers as a baseline")
    parser.add_argument("--check", metavar="FILE", help="compare with a stored baseline")
    parser.add_argument("--markdown", metavar="FILE", help="append the table (and the comparison) as Markdown")
    parser.add_argument("--title", default="Size and startup", help="heading of the Markdown table")
    parser.add_argument("--size-tolerance", type=float, default=1.0, help="allowed growth of a size in percent (default 1)")
    parser.add_argument("--time-tolerance", type=float, default=15.0, help="allowed growth of a time in percent (default 15)")
    options = parser.parse_args()
    if options.runs < 1:
        parser.error("--runs must be at least 1")
    if options.source:
        global ROOT
        ROOT = os.path.abspath(options.source)

    results = measure(options.runs, options.exit_ms, not options.no_build, options.size_only)
    print_table(results)
    machine = describe_machine()
    print(machine)

    markdown = markdown_table(results, options.title) + [machine, ""]
    failed = False
    if options.check:
        with open(options.check) as file:
            baseline = json.load(file)
        rows = comparison(results, baseline, options.size_tolerance, options.time_tolerance)
        print_comparison(rows)
        markdown += markdown_comparison(rows, f"{options.title}: change against {os.path.basename(options.check)}")
        failed = any(row[5] for row in rows)

    if options.save:
        with open(options.save, "w") as file:
            json.dump(results, file, indent=2)
            file.write("\n")
    if options.markdown:
        with open(options.markdown, "a") as file:
            file.write("\n".join(markdown) + "\n")
    if failed:
        sys.exit(1)


if __name__ == "__main__":
    main()
