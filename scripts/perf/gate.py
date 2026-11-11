#!/usr/bin/env python3
"""The local performance gate: the frame cost of the gauntlet and the still wall and the
TreeDataGrid benchmarks, of a base commit against the working tree, measured on this machine.

    python3 scripts/perf/gate.py
    python3 scripts/perf/gate.py --base origin/main --rounds 3 --markdown target/perf-gate.md
    python3 scripts/perf/gate.py --only gauntlet
    python3 scripts/perf/gate.py --only tdg --tdg-filter VirtualizationBenchmarks
    python3 scripts/perf/gate.py --only counters

The base (default: the merge-base of HEAD with origin/main) is checked out, detached, in a git
worktree of its own under the scratch folder (default: `ferroui-gate-scratch` beside the
checkout; reused between runs), and built there with a target folder of its own; the candidate is
this checkout as it is, uncommitted changes included, built with another target folder. Both are
built first, then measured alternately (base, candidate, base, candidate, ...) for `--rounds`
rounds, so that both sides see the same machine. A metric is the median of its rounds. When a
metric is over its tolerance by less than the spread of its own rounds on either side (one
disturbed round can move a median of three that far), the gate measures more rounds, alternately
again, up to `--max-rounds`, before it decides.

What is measured (see docs/porting/desktop-performance.md, section 8.1):

- the frame gate, `samples/GauntletBenchmark/tests/frame_gate.rs`: per scenario (`gauntlet/16`,
  `wall/20`) the milliseconds a frame of the UI thread, of the render thread and of wall time,
  from its `GATE {...}` lines;
- the TreeDataGrid benchmarks, `tests/FerroUI.TreeDataGrid.Benchmarks`: the median time of one
  operation of every benchmark, from the tables of the harness (`ferroui-benchmarks`,
  `harness.rs`).

What is checked without a measurement, on the candidate alone and once (`counters`): the tests
that assert how much work an operation does, read from the framework's performance counters
(`COUNTER_CHECKS`). The counters exist only with the feature `perf-counters`, which no other
automated check turns on, so the assertions of these tests run here. A test that fails fails the
gate.

When the base has no frame gate (commits before it), the candidate's `samples/GauntletBenchmark`
is copied into the base worktree (and made a member of its workspace when it is not one), so that
both sides run the same benchmark code; the output says so.

Cargo runs through the command in the environment variable FERROUI_CARGO when it is set (a
wrapper that serialises builds, for example), else `cargo`. Only the Python standard library is
used. Exit status: 0 when no metric regressed by more than its tolerance, 1 when one did (or a
benchmark failed on the candidate), 2 when a build or a run failed.
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import platform
import re
import shlex
import shutil
import statistics
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

# --- what is run ----------------------------------------------------------------------------------

GATE_PACKAGE = "gauntlet-benchmark"
GATE_SAMPLE = Path("samples/GauntletBenchmark")
GATE_TEST = GATE_SAMPLE / "tests" / "frame_gate.rs"
TDG_PACKAGE = "ferroui-treedatagrid-benchmarks"
# FerroUI's own regression benchmarks, apart from the ports of other frameworks' benchmarks (the
# gauntlet follows the Cranpose benchmark and is synced with it): the scenes of the frame
# benchmarks, timed headless and in a desktop window, and the micro-benchmarks of the framework's
# tree mechanics (category `trees` of `tests/FerroUI.Benchmarks`), of element construction, of
# text blocks measured and arranged at two widths, of text blocks whose text changes and of the
# bidirectional algorithm at two lengths of each shape of text.
FRAMES_PACKAGE = "ferroui-frame-benchmarks"
FRAMES_CRATE = Path("tests/FerroUI.FrameBenchmarks")
FRAMES_BINARY = "frame-benchmarks"
FRAMEWORK_PACKAGE = "ferroui-benchmarks"
FRAMEWORK_CRATE = Path("tests/FerroUI.Benchmarks")
FRAMEWORK_FILTER = (
    "trees::,layout_subsystem::,styling_subsystem::,property_subsystem::,"
    "layout::ElementConstructionBenchmark,"
    "text::TextBlockConstraintBenchmark,text::TextBlockChangingTextBenchmark,"
    "text::BidiScalingBenchmark"
)
# The categories of the framework benchmarks a base has to have to be measured with its own copy
# of them; a base without one of them is measured with the candidate's.
FRAMEWORK_CATEGORIES = ("trees", "layout_subsystem", "styling_subsystem", "property_subsystem")
# The scenes the window suite opens, and how long each runs.
WINDOW_SCENES = ["text", "list", "transforms", "wall", "lines"]

TEST_ARGS = ["--ignored", "--nocapture", "--test-threads=1"]

# The tests of the frame benchmarks (`cargo test ... -- --list --ignored`). Each runs in a process
# of its own: a start is the start of a process that has built nothing yet, and a scene leaves
# the heap and the caches of the process in a state that moves what is measured after it. A
# scene of the start (thousands of elements) moved the build of the `text` scene by a seventh,
# and a scene of the frames the build of the one after it by as much as two thirds when the two
# sides left the allocator in different states (a side that has fewer composition objects frees
# other blocks). What a scene is measured after is then the same on both sides: nothing.
FRAMES_TEST = ["test", "-p", FRAMES_PACKAGE, "--release", "--test", "frames", "--"]

# The tests that assert counts of work (`ferroui_base::diagnostics::perf_counters`), each a
# package, the features that compile the counters in, and the filter of its tests. Their
# assertions are skipped without the feature, so the gate runs them with it: deterministic, on
# the candidate only, once.
COUNTER_CHECKS = [
    ("ferroui-base", "perf-counters", "style_work"),
    ("ferroui-base", "perf-counters", "composition_work"),
    ("ferroui-controls", "perf-counters", "composition_work"),
]

SUITES = {
    # Not measured: `run_counter_checks`.
    "counters": {"build": [], "run": []},
    "gauntlet": {
        "build": ["test", "-p", GATE_PACKAGE, "--release", "--test", "frame_gate", "--no-run"],
        "run": ["test", "-p", GATE_PACKAGE, "--release", "--test", "frame_gate", "--", *TEST_ARGS],
    },
    "tdg": {
        "build": ["test", "-p", TDG_PACKAGE, "--release", "--lib", "--no-run"],
        "run": ["test", "-p", TDG_PACKAGE, "--release", "--lib", "run_benchmarks", "--", *TEST_ARGS],
    },
    "frames": {
        "build": ["test", "-p", FRAMES_PACKAGE, "--release", "--test", "frames", "--no-run"],
        # Not one run: each test in a process of its own (`run_frames`).
        "run": [],
    },
    "framework": {
        "build": ["test", "-p", FRAMEWORK_PACKAGE, "--release", "--lib", "--no-run"],
        "run": ["test", "-p", FRAMEWORK_PACKAGE, "--release", "--lib", "run_benchmarks", "--", *TEST_ARGS],
    },
    # Not run through cargo: the binary is started once per scene (`run_window`).
    "window": {
        "build": ["build", "-p", FRAMES_PACKAGE, "--release", "--bin", FRAMES_BINARY],
        "run": [],
    },
}
ALL_SUITES = ["counters", "gauntlet", "frames", "framework", "tdg", "window"]

# --- tolerances -----------------------------------------------------------------------------------
#
# A metric regresses when the candidate's median exceeds the base's by more than
# `--tolerance` times the metric's factor below, and by more than the metric's floor (a change
# smaller than the floor is below what the measurement resolves, whatever its share).
#
# - UI thread and wall time of a frame: the factor 1. The frame gate averages hundreds of frames
#   of a deterministic workload on the CPU (raster Skia), so a few percent is a real change.
# - Render thread and wall time are those of a frame the surface received (`presented_frames`):
#   the loop of the benchmark does not wait for the display, and a frame of the clock that
#   starts while the render thread still has the batch before it commits nothing and costs
#   close to nothing. A side that presents fewer of its frames would look faster by the frames
#   it did not draw.
# - Render thread: 1.5. The render thread runs beside the UI thread and its time moves with
#   where the scheduler puts the two threads (performance or efficiency cores) and with the state
#   of Skia's glyph caches when the measured window starts.
# - TreeDataGrid benchmarks: 2 (3 for operations under 100 ns). The gate runs the harness with
#   short iterations (`--tdg-iterations`, `--tdg-min-iteration-ms`) in a new process per run;
#   from one process to the next the layout of the heap and of the code alone moves a benchmark
#   by several percent, and the shortest operations are within reach of the timer and the loop.

FRAME_FACTORS = {"ui_ms_per_frame": 1.0, "render_ms_per_frame": 1.5, "wall_ms_per_frame": 1.0}
FRAME_FLOOR_MS = 0.05
# - The start of a scene (the frame benchmarks): its build, its show (first layout, recording
#   and commit) and, for the scenes of the start, the time from the show to the first frame are
#   measured once a run, so a round has one sample of each: 1.5 for the parts, 1 for their sum.
STARTUP_FACTORS = {"build_ms": 1.5, "show_ms": 1.5, "first_frame_ms": 1.5, "startup_ms": 1.0}
STARTUP_FLOOR_MS = 0.5
# - The memory a scene of the start adds to the process (resident megabytes): what the allocator
#   returns to the system moves the parts, their sum less: 2 for the parts, 1 for their sum, and a
#   megabyte is not a change.
MEMORY_FACTORS = {"build_mb": 2.0, "show_mb": 2.0, "first_frame_mb": 2.0, "startup_mb": 1.0}
MEMORY_FLOOR_MB = 1.0
# - The sizes of the objects an element is made of (`SIZES {...}`): exact, so any growth is one.
SIZE_FACTOR = 0.0
SIZE_FLOOR_BYTES = 0.0
# - The window suite: a desktop process per scene on the GPU backend. Frame times move with the
#   display and the compositor of the system (and stop at the display's rate), the first frame
#   with the state of the disk and the shader caches: 2 for both.
WINDOW_FACTOR = 2.0
WINDOW_FRAME_FLOOR_MS = 0.5
WINDOW_FIRST_FRAME_FLOOR_MS = 20.0
TDG_FACTOR = 2.0
TDG_SHORT_FACTOR = 3.0
TDG_SHORT_NS = 100.0
TDG_FLOOR_NS = 2.0


@dataclass
class Metric:
    """One number of a run, lower is better."""

    key: str
    unit: str  # "ms", "ns", "MB" or "bytes"
    factor: float
    floor: float


@dataclass
class Side:
    name: str  # "base" or "candidate"
    root: Path
    target: Path
    label: str
    values: dict[str, list[float]] = field(default_factory=dict)
    metrics: dict[str, Metric] = field(default_factory=dict)
    failed: set[str] = field(default_factory=set)


class GateError(Exception):
    pass


# --- parsing --------------------------------------------------------------------------------------

GATE_LINE = re.compile(r"^GATE (\{.*\})\s*$")
SIZES_LINE = re.compile(r"^SIZES (\{.*\})\s*$")


# The times of a record that are compared for a frame the surface received.
PRESENTED_FRAME_TIMES = ("render_ms_per_frame", "wall_ms_per_frame")


def presented_frames(record: dict) -> float:
    """What a time for a frame of the clock of `record` is multiplied by to be the time for a
    frame the surface received: the frames of the clock over the frames received, when the
    record counts both and fewer were received than the clock had (a record without the count,
    or with as many frames received as measured, is left as it is)."""
    frames = record.get("frames")
    rendered = record.get("rendered")
    if not isinstance(frames, (int, float)) or not isinstance(rendered, (int, float)):
        return 1.0
    if rendered <= 0 or rendered >= frames:
        return 1.0
    return float(frames) / float(rendered)


def parse_gate(output: str) -> list[tuple[Metric, float]]:
    """The metrics of the `GATE {...}` and `SIZES {...}` lines of the frame gate."""
    found = []
    for line in output.splitlines():
        sizes = SIZES_LINE.match(line.strip())
        if sizes:
            for name, size in json.loads(sizes.group(1)).items():
                found.append((Metric(f"sizes/{name}", "bytes", SIZE_FACTOR, SIZE_FLOOR_BYTES), float(size)))
            continue
        match = GATE_LINE.match(line.strip())
        if not match:
            continue
        record = json.loads(match.group(1))
        bench = record["bench"]
        presented = presented_frames(record)
        for name, factor in FRAME_FACTORS.items():
            if name in record:
                metric = Metric(f"{bench} {name}", "ms", factor, FRAME_FLOOR_MS)
                scale = presented if name in PRESENTED_FRAME_TIMES else 1.0
                found.append((metric, float(record[name]) * scale))
        for name, factor in STARTUP_FACTORS.items():
            if name in record:
                metric = Metric(f"{bench} {name}", "ms", factor, STARTUP_FLOOR_MS)
                found.append((metric, float(record[name])))
        for name, factor in MEMORY_FACTORS.items():
            if name in record:
                metric = Metric(f"{bench} {name}", "MB", factor, MEMORY_FLOOR_MB)
                found.append((metric, float(record[name])))
    return found


TIME = re.compile(r"^([0-9]+(?:\.[0-9]+)?) (ns|us|ms|s)$")
TIME_SCALE = {"ns": 1.0, "us": 1e3, "ms": 1e6, "s": 1e9}
CLASS_LINE = re.compile(r"^([A-Za-z0-9_]+)::([A-Za-z0-9_]+)$")
PROGRESS_LINE = re.compile(r"^  (\S.*?): (?:([0-9.]+ (?:ns|us|ms|s)) \(\d+ invocations an iteration\)|FAILED: (.*))$")


def parse_time(text: str) -> float | None:
    """Nanoseconds of a time as the harness formats it (`format_time`: `12.34 ns`, `1.234 us`,
    `1.234 ms`, `1.234 s`)."""
    match = TIME.match(text.strip())
    if not match:
        return None
    return float(match.group(1)) * TIME_SCALE[match.group(2)]


def tdg_metric(full_name: str, nanoseconds: float) -> Metric:
    factor = TDG_SHORT_FACTOR if nanoseconds < TDG_SHORT_NS else TDG_FACTOR
    return Metric(full_name, "ns", factor, TDG_FLOOR_NS)


def parse_tdg(output: str) -> tuple[list[tuple[Metric, float]], set[str]]:
    """The median time of one operation of every benchmark of a run of the harness, and the
    benchmarks that failed.

    The harness prints a line per benchmark as it ends,

        `  treedatagrid::Class.name[Param=value]: 1.234 us (120 invocations an iteration)`
        `  treedatagrid::Class.name[Param=value]: FAILED: <message>`

    (the mean of the iterations), and then a table per class:

        treedatagrid::Class
        | Benchmark | Parameters | Mean | Median | Min | StdDev | ...
        |-----------|------------|------|--------|-----|--------|
        | name      | Param=value | 1.234 us | 1.230 us | ...

    The full name is `category::Class.name`, with `[parameters]` when there are any. The median
    is taken from the table; a benchmark the table does not have (output cut short) falls back
    to the mean of its line.
    """
    medians: dict[str, float] = {}
    means: dict[str, float] = {}
    failed: set[str] = set()

    for line in output.splitlines():
        match = PROGRESS_LINE.match(line.rstrip())
        if match:
            if match.group(3) is not None:
                failed.add(match.group(1))
            else:
                value = parse_time(match.group(2))
                if value is not None:
                    means[match.group(1)] = value

    prefix = None
    columns: list[str] | None = None
    for raw in output.splitlines():
        line = raw.strip()
        class_match = CLASS_LINE.match(line)
        if class_match:
            prefix = f"{class_match.group(1)}::{class_match.group(2)}"
            columns = None
            continue
        if prefix is None or not line.startswith("|"):
            if not line:
                prefix, columns = None, None
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if columns is None:
            columns = cells
            continue
        if all(set(cell) <= {"-"} for cell in cells):
            continue
        if len(cells) != len(columns):
            continue
        row = dict(zip(columns, cells))
        name = row.get("Benchmark", "")
        parameters = row.get("Parameters", "")
        full_name = f"{prefix}.{name}[{parameters}]" if parameters else f"{prefix}.{name}"
        if row.get("Mean", "").startswith("FAILED"):
            failed.add(full_name)
            continue
        value = parse_time(row.get("Median", "")) or parse_time(row.get("Mean", ""))
        if value is not None:
            medians[full_name] = value

    for name, value in means.items():
        medians.setdefault(name, value)
    found = [(tdg_metric(name, value), value) for name, value in medians.items() if name not in failed]
    return found, failed


# --- running --------------------------------------------------------------------------------------


def cargo_command() -> list[str]:
    return shlex.split(os.environ.get("FERROUI_CARGO", "cargo"))


def git(root: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    if check and result.returncode != 0:
        raise GateError(f"git {' '.join(args)}: {result.stderr.strip()}")
    return result.stdout.strip()


def run_cargo(side: Side, args: list[str], env: dict[str, str], log: Path, echo: bool) -> str:
    command = [*cargo_command(), *args]
    full_env = {**os.environ, **env, "CARGO_TARGET_DIR": str(side.target)}
    print(f"[{side.name}] {' '.join(shlex.quote(part) for part in command)}", flush=True)
    log.parent.mkdir(parents=True, exist_ok=True)
    with open(log, "w", encoding="utf-8") as out:
        process = subprocess.Popen(
            command, cwd=side.root, env=full_env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
        )
        lines = []
        assert process.stdout is not None
        for line in process.stdout:
            out.write(line)
            lines.append(line)
            if echo or line.startswith(("GATE ", "error")) or "panicked" in line:
                sys.stdout.write(f"  {line}")
        status = process.wait()
    if status != 0:
        raise GateError(f"[{side.name}] cargo {' '.join(args)} failed with status {status}; the log is {log}")
    return "".join(lines)


def frame_tests(side: Side, env: dict[str, str], log: Path, echo: bool) -> list[str]:
    """The tests the frame benchmarks of `side` measure with (a base may have fewer)."""
    output = run_cargo(side, [*FRAMES_TEST, "--list", "--ignored"], env, log, echo)
    names = (line.removesuffix(": test").strip() for line in output.splitlines() if line.endswith(": test"))
    return sorted(names)


def run_frames(side: Side, tests: list[str], env: dict[str, str], log: Path, echo: bool) -> str:
    """Each test of the frame benchmarks in a process of its own."""
    output = ""
    for test in tests:
        test_log = log.with_name(f"{log.stem}-{test}{log.suffix}")
        output += run_cargo(side, [*FRAMES_TEST, test, "--exact", *TEST_ARGS], env, test_log, echo)
    return output


def prepare_base(root: Path, scratch: Path, base: str, suites: list[str], copy_gate: bool) -> tuple[Path, list[str]]:
    """Checks out `base`, detached, in the worktree of the scratch folder (created on first use),
    and copies into it the candidate's benchmark code the measured suites need when the base has
    none of it or `copy_gate` asks for it: the frame gate of the gauntlet, the frame benchmarks,
    the framework benchmarks (when the base lacks one of their categories). Returns the worktree and the
    folders copied."""
    worktree = scratch / "base"
    common = Path(git(root, "rev-parse", "--path-format=absolute", "--git-common-dir")).resolve()
    if worktree.exists():
        theirs = git(worktree, "rev-parse", "--path-format=absolute", "--git-common-dir", check=False)
        if not theirs or Path(theirs).resolve() != common:
            raise GateError(f"{worktree} exists and is not a worktree of this repository; remove it or pass --scratch")
        # What an earlier run copied in is dropped before the checkout.
        git(worktree, "reset", "--hard", "-q")
        git(worktree, "clean", "-fdq")
        git(worktree, "checkout", "--detach", "-q", base)
    else:
        scratch.mkdir(parents=True, exist_ok=True)
        # --force: the path may be registered from an earlier scratch folder that was deleted.
        git(root, "worktree", "add", "--force", "--detach", str(worktree), base)

    wanted: list[tuple[Path, bool]] = []
    if "gauntlet" in suites:
        wanted.append((GATE_SAMPLE, (worktree / GATE_TEST).is_file()))
    if "frames" in suites or "window" in suites:
        wanted.append((FRAMES_CRATE, (worktree / FRAMES_CRATE / "Cargo.toml").is_file()))
    if "framework" in suites:
        present = all((worktree / FRAMEWORK_CRATE / category).is_dir() for category in FRAMEWORK_CATEGORIES)
        wanted.append((FRAMEWORK_CRATE, present))

    copied = []
    for folder, present in wanted:
        if present and not copy_gate:
            continue
        target = worktree / folder
        if target.exists():
            shutil.rmtree(target)
        shutil.copytree(root / folder, target)
        manifest = worktree / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        member = f'"{folder.as_posix()}"'
        if member not in text:
            text, count = re.subn(r"(?m)^members = \[\n", f"members = [\n    {member},\n", text, count=1)
            if count != 1:
                raise GateError(f"cannot add {folder} to the members of {manifest}")
            manifest.write_text(text, encoding="utf-8")
        copied.append(folder.as_posix())
    return worktree, copied


def run_window(side: Side, scene: str, seconds: float, log: Path, echo: bool) -> list[tuple[Metric, float]]:
    """Opens `scene` in the desktop program of the frame benchmarks for `seconds` and returns its
    first frame (the time from the start of the process to its first animation frame), its second
    frame where the program reports one (the first frame is not drawn yet when it is reported: the
    second is the time by which the scene is on the screen) and its time a frame (from the median
    of its `PERF fps` lines after the first three)."""
    import time

    binary = side.target / "release" / FRAMES_BINARY
    env = {**os.environ, "FRAMES_SCENE": scene, "PERF_REPORT": "1"}
    env.setdefault("PERF_FONTS", str(side.root / "src/FerroUI.Themes.Material/Assets"))
    print(f"[{side.name}] {binary.name} {scene} for {seconds:g} s", flush=True)
    log.parent.mkdir(parents=True, exist_ok=True)
    start = time.monotonic()
    first_frame = None
    second_frame = None
    rates = []
    with open(log, "w", encoding="utf-8") as out:
        process = subprocess.Popen([str(binary)], cwd=side.root, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        assert process.stdout is not None
        try:
            os.set_blocking(process.stdout.fileno(), False)
            buffer = ""
            while time.monotonic() - start < seconds and process.poll() is None:
                try:
                    chunk = process.stdout.read()
                except BlockingIOError:
                    # A text stream over a pipe that does not block raises where there is nothing
                    # to read yet (it does not always return an empty string).
                    chunk = ""
                if chunk:
                    buffer += chunk
                    *lines, buffer = buffer.split("\n")
                    for line in lines:
                        out.write(line + "\n")
                        if echo:
                            sys.stdout.write(f"  {line}\n")
                        if line.startswith("PERF first_frame") and first_frame is None:
                            first_frame = (time.monotonic() - start) * 1000.0
                        elif line.startswith("PERF second_frame") and second_frame is None:
                            second_frame = (time.monotonic() - start) * 1000.0
                        elif line.startswith("PERF fps "):
                            rates.append(float(line.split()[2]))
                else:
                    time.sleep(0.01)
        finally:
            process.kill()
            process.wait()
    if first_frame is None or len(rates) < 5:
        raise GateError(f"[{side.name}] {scene} drew no frames in {seconds:g} s; the log is {log}")
    rate = statistics.median(rates[3:])
    found = [
        (Metric(f"window/{scene} ms_per_frame", "ms", WINDOW_FACTOR, WINDOW_FRAME_FLOOR_MS), 1000.0 / rate),
        (Metric(f"window/{scene} first_frame_ms", "ms", WINDOW_FACTOR, WINDOW_FIRST_FRAME_FLOOR_MS), first_frame),
    ]
    if second_frame is not None:
        found.append(
            (Metric(f"window/{scene} second_frame_ms", "ms", WINDOW_FACTOR, WINDOW_FIRST_FRAME_FLOOR_MS), second_frame)
        )
    return found


def counter_check_args(package: str, features: str, test_filter: str, build: bool) -> list[str]:
    args = ["test", "-p", package, "--features", features, "--lib"]
    return [*args, "--no-run"] if build else [*args, test_filter]


TEST_RESULT = re.compile(r"^test result: \w+\. (\d+) passed; (\d+) failed;")


def run_counter_checks(candidate: Side, logs: Path, echo: bool) -> list[Row]:
    """Runs the counter-asserting tests of `COUNTER_CHECKS` on the candidate and returns a row of
    the table for each: the number of tests that passed, or FAILED (a test failed, or the filter
    selected none). A build that fails is an error of the gate, like the build of a suite."""
    rows = []
    for package, features, test_filter in COUNTER_CHECKS:
        name = f"{package}-{test_filter}"
        key = f"counters/{package} {test_filter}"
        run_cargo(candidate, counter_check_args(package, features, test_filter, True), {}, logs / f"build-counters-{name}.log", echo)
        log = logs / f"counters-{name}.log"
        try:
            run_cargo(candidate, counter_check_args(package, features, test_filter, False), {}, log, echo)
            succeeded = True
        except GateError:
            succeeded = False
        passed = failed = 0
        for line in log.read_text(encoding="utf-8").splitlines():
            match = TEST_RESULT.match(line.strip())
            if match:
                passed += int(match.group(1))
                failed += int(match.group(2))
        if succeeded and failed == 0 and passed > 0:
            rows.append(Row(key, "-", f"{passed} passed", "", "", "ok", False))
        else:
            reason = f"{failed} of {passed + failed} tests failed" if failed else "no test ran" if succeeded else "the run failed"
            print(f"[{candidate.name}] counters: {key}: {reason}; the log is {log}", flush=True)
            rows.append(Row(key, "-", "FAILED", "", "", "FAILED", True))
    return rows


def record(side: Side, found: list[tuple[Metric, float]]) -> None:
    for metric, value in found:
        side.metrics[metric.key] = metric
        side.values.setdefault(metric.key, []).append(value)


# --- the comparison -------------------------------------------------------------------------------


def format_value(value: float, unit: str) -> str:
    if unit == "ms":
        return f"{value:.3f} ms"
    if unit == "MB":
        return f"{value:.1f} MB"
    if unit == "bytes":
        return f"{value:.0f} bytes"
    if value < 1e3:
        return f"{value:.2f} ns"
    if value < 1e6:
        return f"{value / 1e3:.3f} us"
    if value < 1e9:
        return f"{value / 1e6:.3f} ms"
    return f"{value / 1e9:.3f} s"


@dataclass
class Row:
    key: str
    base: str
    candidate: str
    change: str
    tolerance: str
    verdict: str
    regressed: bool


def compare(base: Side, candidate: Side, tolerance: float) -> list[Row]:
    rows = []
    keys = list(dict.fromkeys([*candidate.metrics, *base.metrics, *sorted(candidate.failed | base.failed)]))
    for key in keys:
        metric = candidate.metrics.get(key) or base.metrics.get(key)
        b = statistics.median(base.values[key]) if key in base.values else None
        c = statistics.median(candidate.values[key]) if key in candidate.values else None
        unit = metric.unit if metric else "ns"
        limit = tolerance * metric.factor if metric else tolerance
        cell_b = format_value(b, unit) if b is not None else ("FAILED" if key in base.failed else "-")
        cell_c = format_value(c, unit) if c is not None else ("FAILED" if key in candidate.failed else "-")
        if key in candidate.failed:
            rows.append(Row(key, cell_b, "FAILED", "", "", "FAILED", True))
            continue
        if b is None or c is None or metric is None:
            verdict = "new" if b is None else "missing"
            rows.append(Row(key, cell_b, cell_c, "", f"{limit * 100:g} %", verdict, c is None))
            continue
        change = (c - b) / b if b > 0 else 0.0
        if change > limit and c - b > metric.floor:
            verdict, regressed = "REGRESSED", True
        elif change < -limit and b - c > metric.floor:
            verdict, regressed = "faster", False
        else:
            verdict, regressed = "ok", False
        rows.append(Row(key, cell_b, cell_c, f"{change * 100:+.1f} %", f"{limit * 100:g} %", verdict, regressed))
    return rows


HEADER = ("Metric", "Base", "Candidate", "Change", "Tolerance", "Verdict")


def is_within_spread(base: Side, candidate: Side, key: str) -> bool:
    """Whether the change of a metric's median is smaller than the spread of its rounds on either
    side: then one disturbed round can account for it, and more rounds are needed to tell."""
    b, c = base.values.get(key), candidate.values.get(key)
    if not b or not c or len(b) < 2 or len(c) < 2:
        return False
    change = statistics.median(c) - statistics.median(b)
    return change < max(max(b) - min(b), max(c) - min(c))


def cells(row: Row) -> tuple[str, ...]:
    return (row.key, row.base, row.candidate, row.change, row.tolerance, row.verdict)


def text_table(rows: list[Row]) -> str:
    table = [HEADER, *(cells(row) for row in rows)]
    widths = [max(len(line[column]) for line in table) for column in range(len(HEADER))]
    out = []
    for index, line in enumerate(table):
        out.append(
            "  ".join(
                cell.ljust(widths[column]) if column == 0 else cell.rjust(widths[column])
                for column, cell in enumerate(line)
            )
        )
        if index == 0:
            out.append("  ".join("-" * width for width in widths))
    return "\n".join(out)


def markdown_table(rows: list[Row]) -> str:
    out = ["| " + " | ".join(HEADER) + " |", "|---|---:|---:|---:|---:|---|"]
    for row in rows:
        key, *rest = cells(row)
        out.append("| `" + key + "` | " + " | ".join(rest) + " |")
    return "\n".join(out)


# --- main -----------------------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--base", help="the base commit (default: the merge-base of HEAD with origin/main)")
    parser.add_argument("--rounds", type=int, default=3, help="measured rounds per side (default 3)")
    parser.add_argument(
        "--max-rounds",
        type=int,
        default=9,
        help="the rounds per side up to which a metric over its tolerance by less than the spread of its rounds is measured again (default 9)",
    )
    parser.add_argument(
        "--only",
        default="all",
        help="what to measure, comma separated: counters, gauntlet, frames, framework, tdg, window, or all (default all)",
    )
    parser.add_argument("--framework-filter", default=FRAMEWORK_FILTER, help=f"FERROUI_BENCH_FILTER of the framework run (default {FRAMEWORK_FILTER})")
    parser.add_argument("--window-seconds", type=float, default=12.0, help="how long the window suite runs each scene (default 12)")
    parser.add_argument("--window-scenes", default=",".join(WINDOW_SCENES), help=f"the scenes of the window suite (default {','.join(WINDOW_SCENES)})")
    parser.add_argument("--tolerance", type=float, default=0.05, help="the allowed slowdown, as a fraction (default 0.05)")
    parser.add_argument("--tdg-filter", help="FERROUI_BENCH_FILTER of the TreeDataGrid run: parts of names, comma separated")
    parser.add_argument("--tdg-iterations", type=int, default=7, help="FERROUI_BENCH_ITERATIONS of the TreeDataGrid run (default 7)")
    parser.add_argument("--tdg-warmup", type=int, default=2, help="FERROUI_BENCH_WARMUP of the TreeDataGrid run (default 2)")
    parser.add_argument(
        "--tdg-min-iteration-ms", type=int, default=250, help="FERROUI_BENCH_MIN_ITERATION_MS of the TreeDataGrid run (default 250)"
    )
    parser.add_argument("--frames", type=int, help="GATE_FRAMES of the frame gate (its default: 240)")
    parser.add_argument("--warmup", type=int, help="GATE_WARMUP of the frame gate (its default: 60)")
    parser.add_argument("--scratch", help="the folder of the base worktree, the target folders and the logs (default: ../ferroui-gate-scratch)")
    parser.add_argument("--copy-gate", action="store_true", help="copy the candidate's frame gate into the base even when the base has one")
    parser.add_argument("--markdown", help="also write the table as Markdown to this file, for the description of a pull request")
    parser.add_argument("--verbose", action="store_true", help="echo the whole output of cargo and the benchmarks")
    parser.add_argument("--dry-run", action="store_true", help="print the plan and the commands, run nothing")
    options = parser.parse_args()
    if options.rounds < 1:
        parser.error("--rounds must be at least 1")
    if options.max_rounds < options.rounds:
        parser.error("--max-rounds must be at least --rounds")

    try:
        return gate(options)
    except GateError as error:
        print(f"gate: {error}", file=sys.stderr)
        return 2


def gate(options: argparse.Namespace) -> int:
    root = Path(git(Path(__file__).resolve().parent, "rev-parse", "--show-toplevel"))
    scratch = Path(options.scratch).resolve() if options.scratch else (root.parent / "ferroui-gate-scratch")
    base_ref = options.base
    if base_ref is None:
        if not git(root, "rev-parse", "--verify", "-q", "origin/main", check=False):
            raise GateError("origin/main is unknown here: pass --base")
        base_ref = git(root, "merge-base", "HEAD", "origin/main")
    base_sha = git(root, "rev-parse", "--verify", f"{base_ref}^{{commit}}")
    head_sha = git(root, "rev-parse", "HEAD")
    dirty = bool(git(root, "status", "--porcelain", "--untracked-files=no"))
    candidate_label = head_sha[:10] + (" with uncommitted changes" if dirty else "")
    suites = ALL_SUITES if options.only == "all" else [suite.strip() for suite in options.only.split(",") if suite.strip()]
    unknown = [suite for suite in suites if suite not in SUITES]
    if unknown:
        raise GateError(f"unknown suites: {', '.join(unknown)} (known: {', '.join(ALL_SUITES)})")
    window_scenes = [scene.strip() for scene in options.window_scenes.split(",") if scene.strip()]

    bench_env = {
        "FERROUI_BENCH_ITERATIONS": str(options.tdg_iterations),
        "FERROUI_BENCH_WARMUP": str(options.tdg_warmup),
        "FERROUI_BENCH_MIN_ITERATION_MS": str(options.tdg_min_iteration_ms),
    }
    env = {
        "gauntlet": {},
        "frames": {},
        "framework": {**bench_env, "FERROUI_BENCH_FILTER": options.framework_filter},
        "window": {},
        "tdg": {
            "FERROUI_BENCH_ITERATIONS": str(options.tdg_iterations),
            "FERROUI_BENCH_WARMUP": str(options.tdg_warmup),
            "FERROUI_BENCH_MIN_ITERATION_MS": str(options.tdg_min_iteration_ms),
        },
    }
    if options.tdg_filter:
        env["tdg"]["FERROUI_BENCH_FILTER"] = options.tdg_filter
    for suite in ("gauntlet", "frames"):
        if options.frames is not None:
            env[suite]["GATE_FRAMES"] = str(options.frames)
        if options.warmup is not None:
            env[suite]["GATE_WARMUP"] = str(options.warmup)

    timed = [suite for suite in suites if suite != "counters"]
    what = [f"{options.rounds} rounds of {', '.join(timed)}"] if timed else []
    if "counters" in suites:
        what.insert(0, "the counter checks")
    print(f"base: {base_ref} ({base_sha[:10]}), candidate: {candidate_label}, {' and '.join(what)}")
    print(f"scratch: {scratch}; cargo: {' '.join(cargo_command())}")
    if base_sha == head_sha:
        print("note: the base is HEAD; the comparison shows the uncommitted changes, or the noise of the machine")

    # The counter checks come first and are taken out of the measured suites: they need no base
    # and no rounds.
    counters = "counters" in suites
    suites = [suite for suite in suites if suite != "counters"]
    logs = scratch / "logs" / datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    candidate = Side("candidate", root, scratch / "target-candidate", candidate_label)
    check_rows: list[Row] = []
    if counters and options.dry_run:
        for check in COUNTER_CHECKS:
            print(f"build: {' '.join(cargo_command() + counter_check_args(*check, True))}")
            print(f"run:   {' '.join(cargo_command() + counter_check_args(*check, False))}")
    elif counters:
        check_rows = run_counter_checks(candidate, logs, options.verbose)
        if not suites:
            failures = [row for row in check_rows if row.regressed]
            print()
            print(text_table(check_rows))
            print()
            print(f"{len(check_rows)} counter checks, {len(failures)} failed" if failures else f"{len(check_rows)} counter checks, none failed")
            print(f"logs: {logs}")
            return 1 if failures else 0

    if options.dry_run:
        for suite in suites:
            variables = " ".join(f"{name}={value}" for name, value in env[suite].items())
            print(f"build: {' '.join(cargo_command() + SUITES[suite]['build'])}")
            if suite == "window":
                print(f"run:   {FRAMES_BINARY} with FRAMES_SCENE={','.join(window_scenes)}, {options.window_seconds:g} s each")
            elif suite == "frames":
                print(f"run:   each test in a process of its own: {variables + ' ' if variables else ''}{' '.join(cargo_command() + FRAMES_TEST)} <test> --exact {' '.join(TEST_ARGS)}")
            else:
                print(f"run:   {variables + ' ' if variables else ''}{' '.join(cargo_command() + SUITES[suite]['run'])}")
        return 0

    base_root, copied = prepare_base(root, scratch, base_sha, suites, options.copy_gate)
    if copied:
        reason = "--copy-gate" if options.copy_gate else "the base has no benchmark code of its own for them"
        print(f"note: {reason}: the candidate's {', '.join(copied)} were copied into the base worktree, so both sides run the same benchmark code")

    base = Side("base", base_root, scratch / "target-base", f"{base_ref} ({base_sha[:10]})")

    # Both sides are built before anything is measured: a build does not run beside a measurement.
    for suite in suites:
        for side in (base, candidate):
            run_cargo(side, SUITES[suite]["build"], env[suite], logs / f"build-{suite}-{side.name}.log", options.verbose)

    frames: dict[str, list[str]] = {}
    if "frames" in suites:
        for side in (base, candidate):
            frames[side.name] = frame_tests(side, env["frames"], logs / f"list-frames-{side.name}.log", options.verbose)

    suite_of: dict[str, str] = {}

    def measure(round_index: int, measured: list[str], of: int) -> None:
        for suite in measured:
            for side in (base, candidate):
                print(f"round {round_index + 1} of {of}: {suite}, {side.name}", flush=True)
                log = logs / f"{suite}-{side.name}-{round_index + 1}.log"
                if suite == "window":
                    for scene in window_scenes:
                        scene_log = logs / f"window-{scene.replace('+', '-')}-{side.name}-{round_index + 1}.log"
                        found = run_window(side, scene, options.window_seconds, scene_log, options.verbose)
                        record(side, found)
                        suite_of.update((metric.key, suite) for metric, _ in found)
                    continue
                if suite == "frames":
                    output = run_frames(side, frames[side.name], env[suite], log, options.verbose)
                else:
                    output = run_cargo(side, SUITES[suite]["run"], env[suite], log, options.verbose)
                if suite in ("gauntlet", "frames"):
                    found = parse_gate(output)
                    if not found:
                        raise GateError(f"[{side.name}] the {suite} run printed no GATE line; the log is {log}")
                    record(side, found)
                    suite_of.update((metric.key, suite) for metric, _ in found)
                else:
                    found, failed = parse_tdg(output)
                    if not found and not failed:
                        raise GateError(f"[{side.name}] the {suite} run printed no result; the log is {log}")
                    record(side, found)
                    suite_of.update((metric.key, suite) for metric, _ in found)
                    side.failed |= failed

    for round_index in range(options.rounds):
        measure(round_index, suites, options.rounds)

    rounds = options.rounds
    while rounds < options.max_rounds:
        doubtful = [row.key for row in compare(base, candidate, options.tolerance) if row.regressed and is_within_spread(base, candidate, row.key)]
        if not doubtful:
            break
        measured = [suite for suite in suites if any(suite_of.get(key) == suite for key in doubtful)]
        print(f"over the tolerance by less than the spread of their rounds: {', '.join(doubtful)}; measuring another round", flush=True)
        measure(rounds, measured, rounds + 1)
        rounds += 1

    rows = [*check_rows, *compare(base, candidate, options.tolerance)]
    regressions = [row for row in rows if row.regressed]
    summary = (
        f"{len(rows)} metrics, {len(regressions)} regressed beyond their tolerance"
        if regressions
        else f"{len(rows)} metrics, none regressed beyond its tolerance"
    )
    print()
    print(text_table(rows))
    print()
    print(summary)
    print(f"logs: {logs}")

    if options.markdown:
        machine = f"{platform.system()} {platform.machine()}, Python {platform.python_version()}"
        notes = []
        if copied:
            notes.append(f"The candidate's {', '.join(f'`{folder}`' for folder in copied)} were copied into the base, so both sides run the same benchmark code.")
        lines = [
            "### Local performance gate",
            "",
            f"Base `{base_sha[:10]}` ({base_ref}), candidate `{candidate_label}`; {options.rounds} rounds"
            + (f" (up to {rounds} where a metric was over its tolerance within the spread of its rounds)" if rounds > options.rounds else "")
            + ", ABAB, "
            f"median of the rounds; tolerance {options.tolerance * 100:.0f} % times the factor of the metric "
            "(`scripts/perf/gate.py`). Frame gate: 1280 x 820 at a scaling of 2, raster Skia, render-thread mode. "
            f"TreeDataGrid: median time of an operation, {options.tdg_iterations} iterations of at least "
            f"{options.tdg_min_iteration_ms} ms after {options.tdg_warmup} of warm-up"
            + (f", filter `{options.tdg_filter}`" if options.tdg_filter else "")
            + f". Machine: {machine}.",
            "",
            *([*notes, ""] if notes else []),
            markdown_table(rows),
            "",
            summary + ".",
            "",
        ]
        path = Path(options.markdown)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("\n".join(lines), encoding="utf-8")
        print(f"markdown: {path}")

    return 1 if regressions else 0


if __name__ == "__main__":
    sys.exit(main())
