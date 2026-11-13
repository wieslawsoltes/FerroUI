#!/usr/bin/env python3
"""The local performance gate: the frame cost of the gauntlet and the still wall and the
TreeDataGrid benchmarks, of a base commit against the working tree, measured on this machine.

    python3 scripts/perf/gate.py
    python3 scripts/perf/gate.py --base origin/main --rounds 3 --markdown target/perf-gate.md
    python3 scripts/perf/gate.py --only gauntlet
    python3 scripts/perf/gate.py --only tdg --tdg-filter VirtualizationBenchmarks
    python3 scripts/perf/gate.py --only counters
    python3 scripts/perf/gate.py --only browser --browser-site target/browser/control-catalog-browser
    python3 scripts/perf/gate.py --only window --window-device android
    python3 scripts/perf/gate.py --only window --window-device ios --device-id <simulator>
    python3 scripts/perf/gate.py --base-dir ../ferroui-base

The base (default: the merge-base of HEAD with origin/main) is checked out, detached, in a git
worktree of its own under the scratch folder (default: `ferroui-gate-scratch` beside the
checkout; reused between runs), and built there with a target folder of its own; the candidate is
this checkout as it is, uncommitted changes included, built with another target folder. Both are
built first, then measured alternately for `--rounds` rounds, so that both sides see the same
machine; the side that goes first changes from round to round (base, candidate; candidate, base;
...), so that neither is always the one measured after the other. A metric is the median of its
rounds. When a
metric is over its tolerance by less than the spread of its own rounds on either side (one
disturbed round can move a median of three that far), the gate measures more rounds, alternately
again, up to `--max-rounds`, before it decides.

What is measured (see docs/porting/desktop-performance.md, section 8.1):

- the frame gate, `samples/GauntletBenchmark/tests/frame_gate.rs`: per scenario (`gauntlet/16`,
  `wall/20`) the milliseconds a frame of the UI thread, of the render thread and of wall time,
  from its `GATE {...}` lines;
- the TreeDataGrid benchmarks, `tests/FerroUI.TreeDataGrid.Benchmarks`, and the framework
  benchmarks, `tests/FerroUI.Benchmarks`: the median time of one operation of every benchmark,
  from the tables of the harness (`ferroui-benchmarks`, `harness.rs`), and, where the harness
  counts them, the instructions the processor retires for it. Each class of benchmarks runs in a
  process of its own. A time over its tolerance whose instructions are not is reported as
  `REGRESSED (time alone)`: the work is the same, and the time is that of where this build put
  the code and the data, or of the machine.

What is checked without a measurement, on the candidate alone and once (`counters`): the tests
that assert how much work an operation does, read from the framework's performance counters
(`COUNTER_CHECKS`). The counters exist only with the feature `perf-counters`, which no other
automated check turns on, so the assertions of these tests run here. A test that fails fails the
gate.

What is checked and measured in a browser, only when asked for (`--only browser`, or `browser`
in a list; never part of `all`): what a page switch of the ControlCatalog site costs in headless
Chromium, by `scripts/browser/navigation-cost.mjs` (desktop-performance.md, section 9.56). The
gate does not build browser sites: `--browser-site` names the candidate's, a folder made by
`scripts/build-browser.sh control-catalog-browser` or the address of a deployed site (with the
query that keeps its frames on the thread of the page, `?Threads=false`). The masks every switch
uploads are checked against the budget of the script, on the candidate alone and once: they do
not depend on the machine, and a switch over its budget fails the gate. With `--browser-base-site`
(a folder or an address as well) the two sites are also measured alternately like every other
suite: the masks, their pixels and the time to the first painted frame of each switch.

The window suite on a device (`--window-device android` or `ios`; an Android emulator or device
that runs, an iOS simulator that is booted; `--device-id` when there is more than one): the gate
builds the host of the frame benchmarks of each side for the device
(`tests/FerroUI.FrameBenchmarks.Android`, `tests/FerroUI.FrameBenchmarks.iOS`), packages it with
the script of the platform, installs both under application ids of their own and starts each
scene with the command of the platform (`am start`, `simctl launch`); the `PERF` lines come from
the log of the device or the console of the process. The first frames of Android are timed by
the clock of the device, from the start of the process. Android needs ANDROID_HOME (the SDK), as
`scripts/android/env.sh` does.

Where this script is not in a git checkout (a machine that gets a copy of the sources, made for
example by `scripts/windows/vm-sync.sh`), or with `--base-dir` in any case, the base is a source
tree as it is: nothing is checked out, and each side is named by the commit its copy says it was
made from. The gate runs on macOS, Linux and Windows; the window suite needs a display (on Linux
`xvfb-run -a` gives one). See docs/porting/desktop-performance.md, section 8.2, and
docs/release.md, section 8.4: the gate on every platform, before a release.

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
import queue
import re
import shlex
import shutil
import statistics
import subprocess
import sys
import threading
import time
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

# The variables of the benchmark harness that list the selected benchmarks and that run one class
# of them (`tests/FerroUI.Benchmarks/harness.rs`).
BENCH_LIST = "FERROUI_BENCH_LIST"
BENCH_CLASS = "FERROUI_BENCH_CLASS"
# The suites of the harness. Each class of their benchmarks runs in a process of its own: a
# benchmark starts from the heap, the caches and the threads the benchmarks before it in the
# process left, and the same benchmark of the same build differed by a third between two processes
# that had run the hundred and more before it (`trees::Propagation.font_family`: 1.82 and 1.40 ms)
# where it is within a few percent after the benchmarks of its class alone.
HARNESS_SUITES = ("framework", "tdg")

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
    # Not run through cargo and not built here: `run_browser` on a site that exists.
    "browser": {"build": [], "run": []},
}
ALL_SUITES = ["counters", "gauntlet", "frames", "framework", "tdg", "window"]
# The hosts of the frame benchmarks on a device, which the window suite runs with `--window-device`:
# the package and its folder, and what the gate needs to start the application and to read it.
ANDROID_PACKAGE = "ferroui-frame-benchmarks-android"
ANDROID_CRATE = Path("tests/FerroUI.FrameBenchmarks.Android")
ANDROID_TARGETS = {"arm64-v8a": "aarch64-linux-android", "x86_64": "x86_64-linux-android"}
ANDROID_ACTIVITY = "org.ferroui.android.FerroMainActivity"
# The tag of the log the Android host writes the `PERF` lines with.
ANDROID_TAG = "ferroui-frames"
# The tag the activity manager of Android says with that it started a process (`Start proc`).
ANDROID_SYSTEM_TAG = "ActivityManager"
IOS_PACKAGE = "ferroui-frame-benchmarks-ios"
IOS_CRATE = Path("tests/FerroUI.FrameBenchmarks.iOS")
IOS_BINARY = "frame-benchmarks-ios"
IOS_TARGET = "aarch64-apple-ios-sim" if platform.machine() == "arm64" else "x86_64-apple-ios"
DEVICE_CRATES = {"android": ANDROID_CRATE, "ios": IOS_CRATE}
# The application id of a side on a device is this with the name of the side: both stay installed.
DEVICE_APPLICATION = "org.ferroui.framebenchmarks"
# The file a copy of the sources made by `scripts/windows/vm-sync.sh` names its commit in.
TREE_COMMIT_MARK = ".vm-sync-commit"
# Suites that are run only when `--only` names them: they need something the gate does not make.
OPT_IN_SUITES = ["browser"]
BROWSER_SCRIPT = Path("scripts/browser/navigation-cost.mjs")

# --- tolerances -----------------------------------------------------------------------------------
#
# A metric regresses when the candidate's median exceeds the base's by more than
# `--tolerance` times the metric's factor below, and by more than the metric's floor (a change
# smaller than the floor is below what the measurement resolves, whatever its share).
#
# - UI thread and wall time of a frame: the factor 1. The frame gate averages hundreds of frames
#   of a deterministic workload on the CPU (raster Skia), so a few percent is a real change.
# - The three times are those of a frame the surface received (`presented_frames`): the loop of
#   the benchmark does not wait for the display, and a frame of the clock that commits nothing
#   costs the render thread close to nothing. A side that presents fewer of its frames would
#   look faster by the frames it did not draw, and a side that needs two frames of the clock,
#   with two layouts, for one picture would not show it in its UI thread time
#   (desktop-performance.md, section 9.46).
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
# - The browser suite: the masks a page switch uploads and their pixels are counts of a
#   deterministic route, the same from run to run but for a frame more or less of an animation:
#   5 (a quarter with the default tolerance), and more than a few masks. The time to the first
#   painted frame is that of a software rasteriser in a headless browser: 10, and 50 ms.
BROWSER_COUNT_FACTOR = 5.0
BROWSER_MASKS_FLOOR = 8.0
BROWSER_MASK_PIXELS_FLOOR = 250_000.0
BROWSER_TIME_FACTOR = 10.0
BROWSER_TIME_FLOOR_MS = 50.0
TDG_FACTOR = 2.0
TDG_SHORT_FACTOR = 3.0
TDG_SHORT_NS = 100.0
TDG_FLOOR_NS = 2.0
# - The instructions of a benchmark operation (the harness reads them where the system counts
#   them): 2 %, whatever `--tolerance` is, and more than one instruction. They do not move with
#   the machine or with where a build puts the code: of 336 pairs of runs of the same build the
#   median difference was 0.01 % and the largest 0.74 %. A benchmark over this tolerance does more
#   work, whatever its time says; a benchmark whose time is over its tolerance and whose
#   instructions are not does the same work in more time. A few benchmarks count differently
#   from one process to the next (the order of a hash table follows a seed the process draws:
#   `Propagation.styles_changed` counted 13.70 to 14.51 million in four runs of one build), so
#   the instructions are over their tolerance only when every round of the candidate also
#   counted more than every round of the base.
INSTRUCTIONS_TOLERANCE = 0.02
INSTRUCTIONS_FLOOR = 1.0


@dataclass
class Metric:
    """One number of a run, lower is better."""

    key: str
    unit: str  # "ms", "ns", "MB", "bytes" or "count"
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
    # The instructions of an operation of a benchmark, a value a round, by the key of its time.
    instructions: dict[str, list[float]] = field(default_factory=dict)


def adb_tool() -> str:
    """`adb` of the search path, or the one of the SDK that `ANDROID_HOME` names."""
    found = shutil.which("adb")
    if found:
        return found
    sdk = os.environ.get("ANDROID_HOME") or os.environ.get("ANDROID_SDK_ROOT")
    if sdk:
        tool = Path(sdk) / "platform-tools" / ("adb.exe" if os.name == "nt" else "adb")
        if tool.exists():
            return str(tool)
    return "adb"


@dataclass
class Device:
    """Where the window suite runs when it is not this machine: an Android device or emulator
    (`adb`), or an iOS simulator (`xcrun simctl`)."""

    kind: str  # "android" or "ios"
    id: str | None
    # Android: the ABI of the device, which names the target of the build.
    abi: str = ""

    def adb(self, *args: str) -> list[str]:
        return [adb_tool(), *(["-s", self.id] if self.id else []), *args]

    @property
    def simulator(self) -> str:
        return self.id or "booted"

    @property
    def target(self) -> str:
        return ANDROID_TARGETS[self.abi] if self.kind == "android" else IOS_TARGET

    def __str__(self) -> str:
        if self.kind == "android":
            return f"an Android device ({self.id or 'the one adb sees'}, {self.abi})"
        return f"an iOS simulator ({self.simulator}, {IOS_TARGET})"


class GateError(Exception):
    pass


# --- parsing --------------------------------------------------------------------------------------

GATE_LINE = re.compile(r"^GATE (\{.*\})\s*$")
SIZES_LINE = re.compile(r"^SIZES (\{.*\})\s*$")


# The times of a record that are compared for a frame the surface received: all of them. The
# work of the UI thread in a frame of the clock that committed nothing is work for the picture
# that followed it.
PRESENTED_FRAME_TIMES = ("ui_ms_per_frame", "render_ms_per_frame", "wall_ms_per_frame")


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


def parse_tdg(output: str) -> tuple[list[tuple[Metric, float]], set[str], dict[str, float]]:
    """The median time of one operation of every benchmark of a run of the harness (or of
    several, one after the other), the benchmarks that failed, and the instructions of an
    operation of the benchmarks whose table has them.

    The harness prints a line per benchmark as it ends,

        `  treedatagrid::Class.name[Param=value]: 1.234 us (120 invocations an iteration)`
        `  treedatagrid::Class.name[Param=value]: FAILED: <message>`

    (the mean of the iterations), and then a table per class:

        treedatagrid::Class
        | Benchmark | Parameters | Mean | Median | Min | StdDev | ... | Instructions | Cycles |
        |-----------|------------|------|--------|-----|--------|
        | name      | Param=value | 1.234 us | 1.230 us | ...          |       152210 |  48211 |

    The full name is `category::Class.name`, with `[parameters]` when there are any. The median
    is taken from the table; a benchmark the table does not have (output cut short) falls back
    to the mean of its line. The columns of the processor are there where the system counts
    (`harness::processor`).
    """
    medians: dict[str, float] = {}
    instructions: dict[str, float] = {}
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
        try:
            instructions[full_name] = float(row.get("Instructions", ""))
        except ValueError:
            pass

    for name, value in means.items():
        medians.setdefault(name, value)
    found = [(tdg_metric(name, value), value) for name, value in medians.items() if name not in failed]
    return found, failed, {name: value for name, value in instructions.items() if name not in failed}


# --- running --------------------------------------------------------------------------------------


def cargo_command() -> list[str]:
    return shlex.split(os.environ.get("FERROUI_CARGO", "cargo"))


def git(root: Path, *args: str, check: bool = True) -> str:
    try:
        result = subprocess.run(["git", "-C", str(root), *args], capture_output=True, text=True)
    except FileNotFoundError:
        # A machine without git measures two source trees (`--base-dir`).
        if check:
            raise GateError(f"git {' '.join(args)}: git is not installed") from None
        return ""
    if check and result.returncode != 0:
        raise GateError(f"git {' '.join(args)}: {result.stderr.strip()}")
    return result.stdout.strip()


def checkout_root(folder: Path) -> Path | None:
    """`folder` when it is the top of a git checkout, else nothing (a copy of the sources, also one
    that lies inside the checkout of something else)."""
    top = git(folder, "rev-parse", "--show-toplevel", check=False)
    return folder if top and Path(top).resolve() == folder.resolve() else None


def tree_commit(folder: Path) -> str | None:
    """The commit a source tree was made from: what its mark says (a copy), else its HEAD (a
    checkout), else nothing."""
    mark = folder / TREE_COMMIT_MARK
    if mark.is_file():
        return mark.read_text(encoding="utf-8").strip() or None
    if checkout_root(folder) is not None:
        return git(folder, "rev-parse", "HEAD", check=False) or None
    return None


def run_tool(command: list[str], env: dict[str, str] | None = None, cwd: Path | None = None, log: Path | None = None) -> str:
    """Runs a tool of a device (adb, simctl, a packaging script) and returns its output."""
    try:
        result = subprocess.run(
            command, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace"
        )
    except FileNotFoundError:
        raise GateError(f"{command[0]} is not installed, or not in PATH") from None
    if log is not None:
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text(result.stdout, encoding="utf-8")
    if result.returncode != 0:
        tail = "\n".join(result.stdout.strip().splitlines()[-8:])
        raise GateError(f"{' '.join(shlex.quote(part) for part in command)} failed with status {result.returncode}:\n{tail}")
    return result.stdout


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


LISTED_NAME = re.compile(r"(?:^|\.\.\. )([A-Za-z0-9_]+::[A-Za-z0-9_]+)\.\S")


def runs_one_class(side: Side) -> bool:
    """Whether the benchmark harness of `side` can run one class of the selected benchmarks (a
    base from before it could cannot)."""
    harness = side.root / FRAMEWORK_CRATE / "harness.rs"
    return harness.is_file() and BENCH_CLASS in harness.read_text(encoding="utf-8")


def benchmark_classes(side: Side, suite: str, env: dict[str, str], log: Path, echo: bool) -> list[str]:
    """The classes (`category::Class`) of the benchmarks the run of `suite` selects on `side`, in
    the order of the harness."""
    output = run_cargo(side, SUITES[suite]["run"], {**env, BENCH_LIST: "1"}, log, echo)
    # The first name follows the line of the test (`test ... run_benchmarks ... `) when the
    # output is not captured.
    classes = [match.group(1) for match in map(LISTED_NAME.search, output.splitlines()) if match]
    return list(dict.fromkeys(classes))


def run_benchmarks(side: Side, suite: str, classes: list[str] | None, env: dict[str, str], log: Path, echo: bool) -> str:
    """The benchmarks of `suite`: each of `classes` in a process of its own, or all of them in
    one when there is no list of classes."""
    if classes is None:
        return run_cargo(side, SUITES[suite]["run"], env, log, echo)
    output = ""
    for name in classes:
        class_log = log.with_name(f"{log.stem}-{name.replace('::', '-')}{log.suffix}")
        output += run_cargo(side, SUITES[suite]["run"], {**env, BENCH_CLASS: name}, class_log, echo)
    return output


def checkout_base(root: Path, scratch: Path, base: str) -> Path:
    """Checks out `base`, detached, in the worktree of the scratch folder (created on first use),
    and returns the worktree."""
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
    return worktree


def copy_benchmarks(root: Path, worktree: Path, suites: list[str], copy_gate: bool, device: Device | None) -> list[str]:
    """Copies into the source tree of the base the candidate's benchmark code the measured suites
    need when the base has none of it or `copy_gate` asks for it: the frame gate of the gauntlet,
    the frame benchmarks, their host on a device (the window suite with `--window-device`; with a
    host that is copied the frame benchmarks are copied as well, since the host is written for
    them), the framework benchmarks (when the base lacks one of their categories). Returns the
    folders copied."""
    wanted: list[tuple[Path, bool]] = []
    if "gauntlet" in suites:
        wanted.append((GATE_SAMPLE, (worktree / GATE_TEST).is_file()))
    if "frames" in suites or "window" in suites:
        host = DEVICE_CRATES[device.kind] if device is not None and "window" in suites else None
        host_present = host is None or (worktree / host / "Cargo.toml").is_file()
        wanted.append((FRAMES_CRATE, host_present and (worktree / FRAMES_CRATE / "Cargo.toml").is_file()))
        if host is not None:
            wanted.append((host, host_present))
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
    return copied


@dataclass
class PerfReport:
    """What the `PERF` lines of a run of the frame benchmarks said, with the times in
    milliseconds from its start."""

    first_frame: float | None = None
    second_frame: float | None = None
    rates: list[float] = field(default_factory=list)
    # The scene the program says it shows (`PERF scene`; a program before that line says none).
    scene: str | None = None


class AndroidClock:
    """The times of the lines of `logcat -v threadtime`, by the clock of the device and from the
    start of the process of `application` (the `Start proc` line of the activity manager):
    neither the time the command that starts it takes nor the time a line takes to reach this
    machine is in them."""

    LINE = re.compile(r"^\d\d-\d\d (\d\d):(\d\d):(\d\d)\.(\d\d\d)\s+(\d+)\s+\d+\s+\w\s+(.*?)\s*: (.*)$")

    def __init__(self, application: str):
        self.started = re.compile(rf"^Start proc (\d+):{re.escape(application)}/")
        self.process: str | None = None
        self.start = 0.0

    def __call__(self, line: str) -> tuple[float, str] | None:
        """The time of `line` in milliseconds from the start of the process and what it says, or
        nothing for a line that is not of the process (one of a run before it among them)."""
        found = self.LINE.match(line)
        if found is None:
            return None
        hours, minutes, seconds, thousandths, process, tag, text = found.groups()
        at = ((int(hours) * 60 + int(minutes)) * 60 + int(seconds)) * 1000.0 + int(thousandths)
        if tag == ANDROID_SYSTEM_TAG:
            started = self.started.match(text)
            if started is not None:
                self.process, self.start = started.group(1), at
            return None
        if self.process is None or process != self.process or tag != ANDROID_TAG:
            return None
        # The line has the time of the day: a run over midnight goes on in the next one.
        return (at - self.start) % 86_400_000.0, text


def read_perf(
    command: list[str], env: dict[str, str] | None, cwd: Path | None, seconds: float, log: Path, echo: bool, launch=None, clock=None
) -> PerfReport:
    """Runs `command` for `seconds` and returns what its `PERF` lines said. `launch`, when given,
    is called once the command runs and starts what the command reports (on a device the command
    is the stream of its log); the run then lasts `seconds` from that call. The times count from
    the start of the command, with the time each line was read at, by a thread of its own
    (neither a pipe that blocks nor one that does not is the same on every system); or, with
    `clock`, they are what it says of each line (`AndroidClock`), and the log has the time each
    line was read at in front of it."""
    report = PerfReport()
    log.parent.mkdir(parents=True, exist_ok=True)
    lines: queue.Queue[tuple[float, str]] = queue.Queue()
    with open(log, "w", encoding="utf-8") as out:
        try:
            process = subprocess.Popen(
                command, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace"
            )
        except FileNotFoundError:
            raise GateError(f"{command[0]} cannot be started: it is not built, not installed or not in PATH") from None
        assert process.stdout is not None

        def pump(stream=process.stdout) -> None:
            for line in stream:
                lines.put((time.monotonic(), line.rstrip("\r\n")))

        threading.Thread(target=pump, daemon=True).start()
        try:
            start = time.monotonic()
            if launch is not None:
                launch()
            while True:
                left = start + seconds - time.monotonic()
                if left <= 0:
                    break
                try:
                    stamp, line = lines.get(timeout=min(left, 0.1))
                except queue.Empty:
                    if process.poll() is not None:
                        break
                    continue
                at = (stamp - start) * 1000.0
                out.write(line + "\n" if clock is None else f"{at:9.1f} {line}\n")
                if echo:
                    sys.stdout.write(f"  {line}\n")
                if clock is not None:
                    told = clock(line)
                    if told is None:
                        continue
                    at, line = told
                elif stamp < start:
                    continue
                if line.startswith("PERF scene "):
                    report.scene = line.split()[2]
                elif line.startswith("PERF first_frame") and report.first_frame is None:
                    report.first_frame = at
                elif line.startswith("PERF second_frame") and report.second_frame is None:
                    report.second_frame = at
                elif line.startswith("PERF fps "):
                    report.rates.append(float(line.split()[2]))
        finally:
            process.kill()
            process.wait()
    return report


def window_metrics(side: Side, scene: str, report: PerfReport, seconds: float, log: Path) -> list[tuple[Metric, float]]:
    """The metrics of a scene in a window: its first frame (the time from the start to its first
    animation frame), its second frame where the program reports one (the first frame is not
    drawn yet when it is reported: the second is the time by which the scene is on the screen)
    and its time a frame (from the median of its `PERF fps` lines after the first three)."""
    if report.scene is not None and report.scene != scene:
        raise GateError(f"[{side.name}] the program showed {report.scene} where {scene} was asked for; the log is {log}")
    if report.first_frame is None or len(report.rates) < 5:
        raise GateError(f"[{side.name}] {scene} drew no frames in {seconds:g} s; the log is {log}")
    rate = statistics.median(report.rates[3:])
    found = [
        (Metric(f"window/{scene} ms_per_frame", "ms", WINDOW_FACTOR, WINDOW_FRAME_FLOOR_MS), 1000.0 / rate),
        (Metric(f"window/{scene} first_frame_ms", "ms", WINDOW_FACTOR, WINDOW_FIRST_FRAME_FLOOR_MS), report.first_frame),
    ]
    if report.second_frame is not None:
        found.append(
            (Metric(f"window/{scene} second_frame_ms", "ms", WINDOW_FACTOR, WINDOW_FIRST_FRAME_FLOOR_MS), report.second_frame)
        )
    return found


def run_window(side: Side, scene: str, seconds: float, log: Path, echo: bool) -> list[tuple[Metric, float]]:
    """Opens `scene` in the desktop program of the frame benchmarks for `seconds` and returns its
    metrics (`window_metrics`)."""
    binary = side.target / "release" / (FRAMES_BINARY + (".exe" if os.name == "nt" else ""))
    env = {**os.environ, "FRAMES_SCENE": scene, "PERF_REPORT": "1"}
    # A program before the fonts were part of it reads them from this folder.
    env.setdefault("PERF_FONTS", str(side.root / "src/FerroUI.Themes.Material/Assets"))
    print(f"[{side.name}] {binary.name} {scene} for {seconds:g} s", flush=True)
    report = read_perf([str(binary)], env, side.root, seconds, log, echo)
    return window_metrics(side, scene, report, seconds, log)


def find_device(kind: str, identifier: str | None) -> Device:
    """The device of the window suite: an Android device or emulator that is running (its ABI is
    asked of it), or an iOS simulator that is booted."""
    device = Device(kind, identifier)
    if kind == "android":
        device.abi = run_tool(device.adb("shell", "getprop", "ro.product.cpu.abi")).strip()
        if device.abi not in ANDROID_TARGETS:
            raise GateError(f"the ABI of the Android device is {device.abi or 'unknown'}: one of {', '.join(ANDROID_TARGETS)} is needed")
        # A screen that is off draws no frames.
        run_tool(device.adb("shell", "input", "keyevent", "KEYCODE_WAKEUP"))
    else:
        run_tool(["xcrun", "simctl", "bootstatus", device.simulator])
    return device


def android_environment(root: Path) -> dict[str, str]:
    """The environment `scripts/android/env.sh` of a source tree sets for a build: the linker and
    the compilers of the NDK. The SDK is the one of ANDROID_HOME."""
    script = root / "scripts/android/env.sh"
    dump = "import json, os; print(json.dumps(dict(os.environ)))"
    output = run_tool(["bash", "-c", 'source "$1" >/dev/null && exec "$2" -c "$3"', "_", str(script), sys.executable, dump])
    return json.loads(output.strip().splitlines()[-1])


def device_application(side: Side) -> str:
    return f"{DEVICE_APPLICATION}.{side.name}"


def device_build_args(device: Device) -> list[str]:
    if device.kind == "android":
        return ["build", "-p", ANDROID_PACKAGE, "--lib", "--target", device.target, "--release"]
    return ["build", "-p", IOS_PACKAGE, "--target", device.target, "--release", "--bin", IOS_BINARY]


def build_device(side: Side, device: Device, scratch: Path, logs: Path, echo: bool) -> None:
    """Builds the host of the frame benchmarks of `side` for the device, packages it with the
    script of the platform (which is told that the build is done: the build goes through
    `run_cargo` like every other) and installs it, with an application id of the side."""
    out = scratch / f"device-{side.name}"
    application = device_application(side)
    name = f"Frames {side.name}"
    if device.kind == "android":
        env = android_environment(side.root)
        run_cargo(side, device_build_args(device), env, logs / f"build-window-{side.name}.log", echo)
        command = [
            "bash", str(side.root / "scripts/android/apk.sh"), "package", "--package", ANDROID_PACKAGE, "--release",
            "--abi", device.abi, "--application-id", application, "--label", name, "--out", str(out), "--no-cargo",
        ]  # fmt: skip
        output = run_tool(command, {**env, "CARGO_TARGET_DIR": str(side.target)}, side.root, logs / f"package-window-{side.name}.log")
        last = output.strip().splitlines()[-1] if output.strip() else ""
        if not last.startswith("APK: "):
            raise GateError(f"[{side.name}] apk.sh did not name its package; the log is {logs / f'package-window-{side.name}.log'}")
        package = last.removeprefix("APK: ").strip()
        print(f"[{side.name}] adb install {package}", flush=True)
        run_tool(device.adb("install", "-r", package), log=logs / f"install-window-{side.name}.log")
    else:
        sdk = run_tool(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"]).strip()
        run_cargo(side, device_build_args(device), {"SDKROOT": sdk}, logs / f"build-window-{side.name}.log", echo)
        command = [
            "bash", str(side.root / "scripts/ios/bundle.sh"), "--package", IOS_PACKAGE, "--bin", IOS_BINARY,
            "--target", device.target, "--profile", "release", "--bundle-id", application, "--name", name,
            "--out", str(out), "--no-build",
        ]  # fmt: skip
        env = {**os.environ, "CARGO_TARGET_DIR": str(side.target)}
        output = run_tool(command, env, side.root, logs / f"package-window-{side.name}.log")
        bundle = output.strip().splitlines()[-1] if output.strip() else ""
        if not Path(bundle).is_dir():
            raise GateError(f"[{side.name}] bundle.sh did not name its bundle; the log is {logs / f'package-window-{side.name}.log'}")
        print(f"[{side.name}] simctl install {bundle}", flush=True)
        run_tool(["xcrun", "simctl", "install", device.simulator, bundle], log=logs / f"install-window-{side.name}.log")


def run_device_window(side: Side, device: Device, scene: str, seconds: float, log: Path, echo: bool) -> list[tuple[Metric, float]]:
    """Starts the application of `side` on the device with `scene` for `seconds` and returns its
    metrics (`window_metrics`). Android: the scene is an extra of the intent, and the lines are
    read from the log of the device, which is emptied first; the times are those of the device,
    from the start of the process (`AndroidClock`). iOS: the scene is in the environment of the
    process, and the lines are its standard output on the console of `simctl launch`; the times
    are from that command."""
    application = device_application(side)
    print(f"[{side.name}] {application} {scene} for {seconds:g} s", flush=True)
    if device.kind == "android":
        run_tool(device.adb("shell", "am", "force-stop", application))
        run_tool(device.adb("logcat", "-c"))
        start = device.adb("shell", "am", "start", "-n", f"{application}/{ANDROID_ACTIVITY}", "--es", "FRAMES_SCENE", scene)
        stream = device.adb("logcat", "-v", "threadtime", "-s", f"{ANDROID_TAG}:I", f"{ANDROID_SYSTEM_TAG}:I")
        clock = AndroidClock(application)
        try:
            report = read_perf(stream, None, None, seconds, log, echo, lambda: run_tool(start), clock)
        finally:
            run_tool(device.adb("shell", "am", "force-stop", application))
        if clock.process is None:
            raise GateError(
                f"[{side.name}] the log of the device does not say when the process of {application} started "
                f"(no `Start proc` line of {ANDROID_SYSTEM_TAG}); the log is {log}"
            )
    else:
        env = {**os.environ, "SIMCTL_CHILD_FRAMES_SCENE": scene, "SIMCTL_CHILD_PERF_REPORT": "1"}
        launch = ["xcrun", "simctl", "launch", "--console-pty", "--terminate-running-process", device.simulator, application]
        try:
            report = read_perf(launch, env, None, seconds, log, echo)
        finally:
            subprocess.run(["xcrun", "simctl", "terminate", device.simulator, application], capture_output=True)
    return window_metrics(side, scene, report, seconds, log)


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


def run_browser(root: Path, name: str, site: str, log: Path, echo: bool) -> tuple[list[tuple[Metric, float]], dict]:
    """Runs the page switches of `BROWSER_SCRIPT` on `site` (a folder or an address) in headless
    Chromium and returns, for every page and visit of its route, the masks the switch uploads,
    their pixels and the time to its first painted frame, and the whole result of the script
    (with `budget` and `over`, the switches over their budget)."""
    node = shutil.which("node")
    if node is None:
        raise GateError("the browser suite needs node on the PATH")
    if not re.match(r"^https?:", site) and not (Path(site) / "index.html").is_file():
        raise GateError(f"no site at {site}: build it with scripts/build-browser.sh control-catalog-browser")
    command = [node, str(root / BROWSER_SCRIPT), site, "--json"]
    print(f"[{name}] {' '.join(shlex.quote(part) for part in command)}", flush=True)
    log.parent.mkdir(parents=True, exist_ok=True)
    done = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, check=False)
    log.write_text(done.stdout, encoding="utf-8")
    if echo:
        print(done.stdout, flush=True)
    lines = [line for line in done.stdout.splitlines() if line.startswith("{")]
    if done.returncode != 0 or not lines:
        raise GateError(f"[{name}] the page switches of {site} were not measured; the log is {log}")
    result = json.loads(lines[-1])
    found = []
    for visit in ("first", "again"):
        for page, numbers in result[visit].items():
            key = f"browser/{page} {'first visit' if visit == 'first' else 'shown again'}"
            found.append((Metric(f"{key} masks", "count", BROWSER_COUNT_FACTOR, BROWSER_MASKS_FLOOR), float(numbers["masks"])))
            found.append(
                (Metric(f"{key} mask pixels", "count", BROWSER_COUNT_FACTOR, BROWSER_MASK_PIXELS_FLOOR), float(numbers["maskPixels"]))
            )
            if numbers.get("paintedMs") is not None:
                found.append((Metric(f"{key} painted", "ms", BROWSER_TIME_FACTOR, BROWSER_TIME_FLOOR_MS), float(numbers["paintedMs"])))
    return found, result


def run_browser_budget(candidate: Side, site: str, logs: Path, echo: bool) -> list[Row]:
    """Checks the page switches of the candidate's site against the budget of `BROWSER_SCRIPT` and
    returns a row of the table for each budget: what the switch uploaded, what it may upload."""
    _, result = run_browser(candidate.root, candidate.name, site, logs / "browser-budget.log", echo)
    over = result.get("over", [])
    rows = []
    for visit, pages in result["budget"].items():
        for page, limits in pages.items():
            numbers = result[visit].get(page)
            label = f"{page} {'first visit' if visit == 'first' else 'shown again'}"
            for name, limit in limits.items():
                key = f"browser/{label} budget of {'masks' if name == 'masks' else 'mask pixels'}"
                if numbers is None:
                    rows.append(Row(key, "-", "FAILED", "", f"{limit:,.0f}", "FAILED", True))
                    continue
                kept = numbers[name] <= limit
                rows.append(
                    Row(key, "-", format_value(float(numbers[name]), "count"), "", f"{limit:,.0f}", "ok" if kept else "OVER BUDGET", not kept)
                )
    for line in over:
        print(f"[{candidate.name}] browser: over the budget: {line}", flush=True)
    return rows


def record(side: Side, found: list[tuple[Metric, float]], instructions: dict[str, float] | None = None) -> None:
    for metric, value in found:
        side.metrics[metric.key] = metric
        side.values.setdefault(metric.key, []).append(value)
    for key, value in (instructions or {}).items():
        side.instructions.setdefault(key, []).append(value)


# --- the comparison -------------------------------------------------------------------------------


def format_value(value: float, unit: str) -> str:
    if unit == "ms":
        return f"{value:.3f} ms"
    if unit == "MB":
        return f"{value:.1f} MB"
    if unit == "bytes":
        return f"{value:.0f} bytes"
    if unit == "count":
        return f"{value:,.0f}" if value == int(value) else f"{value:,.1f}"
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
    # The change of the instructions of an operation, for a benchmark both sides counted them for.
    instructions: str = ""
    # Whether what regressed is the measured value (a time): more rounds can then tell a
    # disturbed round from a change, which they cannot for a count.
    measured: bool = False


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
        counted = key in base.instructions and key in candidate.instructions
        counts_b = statistics.median(base.instructions[key]) if counted else 0.0
        counts_c = statistics.median(candidate.instructions[key]) if counted else 0.0
        counts = (counts_c - counts_b) / counts_b if counts_b > 0 else 0.0
        more_work = (
            counts > INSTRUCTIONS_TOLERANCE
            and counts_c - counts_b > INSTRUCTIONS_FLOOR
            and min(candidate.instructions[key]) > max(base.instructions[key])
        )
        less_work = counted and counts < -INSTRUCTIONS_TOLERANCE and (
            max(candidate.instructions[key]) < min(base.instructions[key])
        )
        measured = change > limit and c - b > metric.floor
        if measured:
            # The same work in more time is a regression too, and says what it is.
            verdict, regressed = ("REGRESSED (time alone)" if counted and not more_work else "REGRESSED"), True
        elif more_work:
            verdict, regressed = "REGRESSED (instructions)", True
        elif change < -limit and b - c > metric.floor:
            # Less time for the same work is what a quieter round looks like, and what a change of
            # the memory or of the layout of the code looks like: the line says which it cannot be.
            verdict, regressed = ("faster (time alone)" if counted and not less_work else "faster"), False
        else:
            verdict, regressed = "ok", False
        rows.append(
            Row(
                key,
                cell_b,
                cell_c,
                f"{change * 100:+.1f} %",
                f"{limit * 100:g} %",
                verdict,
                regressed,
                f"{counts * 100:+.2f} %" if counted else "",
                measured,
            )
        )
    return rows


HEADER = ("Metric", "Base", "Candidate", "Change", "Instructions", "Tolerance", "Verdict")
INSTRUCTIONS_COLUMN = HEADER.index("Instructions")


def is_within_spread(base: Side, candidate: Side, key: str) -> bool:
    """Whether the change of a metric's median is smaller than the spread of its rounds on either
    side: then one disturbed round can account for it, and more rounds are needed to tell."""
    b, c = base.values.get(key), candidate.values.get(key)
    if not b or not c or len(b) < 2 or len(c) < 2:
        return False
    change = statistics.median(c) - statistics.median(b)
    return change < max(max(b) - min(b), max(c) - min(c))


def cells(row: Row) -> tuple[str, ...]:
    return (row.key, row.base, row.candidate, row.change, row.instructions, row.tolerance, row.verdict)


def table_of(rows: list[Row]) -> list[tuple[str, ...]]:
    """The header and the lines of `rows`, without the column of the instructions when no row
    has them."""
    table = [HEADER, *(cells(row) for row in rows)]
    if any(row.instructions for row in rows):
        return table
    return [line[:INSTRUCTIONS_COLUMN] + line[INSTRUCTIONS_COLUMN + 1 :] for line in table]


def text_table(rows: list[Row]) -> str:
    table = table_of(rows)
    widths = [max(len(line[column]) for line in table) for column in range(len(table[0]))]
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
    header, *lines = table_of(rows)
    out = ["| " + " | ".join(header) + " |", "|---|" + "---:|" * (len(header) - 2) + "---|"]
    for key, *rest in lines:
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
        help="what to measure, comma separated: counters, gauntlet, frames, framework, tdg, window, or all (default all); "
        "browser only when it is named",
    )
    parser.add_argument(
        "--browser-site",
        help="the ControlCatalog site of the candidate for the browser suite: a folder built by scripts/build-browser.sh or an address",
    )
    parser.add_argument("--browser-base-site", help="the site of the base, to measure the two against each other as well")
    parser.add_argument("--framework-filter", default=FRAMEWORK_FILTER, help=f"FERROUI_BENCH_FILTER of the framework run (default {FRAMEWORK_FILTER})")
    parser.add_argument("--window-seconds", type=float, default=12.0, help="how long the window suite runs each scene (default 12)")
    parser.add_argument("--window-scenes", default=",".join(WINDOW_SCENES), help=f"the scenes of the window suite (default {','.join(WINDOW_SCENES)})")
    parser.add_argument(
        "--window-device",
        choices=sorted(DEVICE_CRATES),
        help="run the window suite on a device instead of this machine: an Android device or emulator that is running, or an iOS simulator that is booted",
    )
    parser.add_argument("--device-id", help="the device of --window-device: the serial of adb, or the identifier of the simulator (default: the one that runs)")
    parser.add_argument(
        "--base-dir",
        help="the source tree of the base, as it is, instead of a worktree the gate checks out; needed where the candidate is not a git checkout",
    )
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
    # The candidate is the source tree this script is in: a git checkout, or a copy of one (on a
    # machine that gets its sources copied; the base is then a copy as well, `--base-dir`).
    root = Path(__file__).resolve().parents[2]
    checkout = checkout_root(root) is not None
    scratch = Path(options.scratch).resolve() if options.scratch else (root.parent / "ferroui-gate-scratch")
    base_dir = Path(options.base_dir).resolve() if options.base_dir else None
    if base_dir is not None:
        if not (base_dir / "Cargo.toml").is_file():
            raise GateError(f"--base-dir: {base_dir} is not a source tree of the workspace")
        base_sha = tree_commit(base_dir) or base_dir.name
        base_ref = options.base or f"the tree {base_dir.name}"
    elif not checkout:
        raise GateError(f"{root} is not a git checkout: pass the source tree of the base with --base-dir")
    else:
        base_ref = options.base
        if base_ref is None:
            if not git(root, "rev-parse", "--verify", "-q", "origin/main", check=False):
                raise GateError("origin/main is unknown here: pass --base")
            base_ref = git(root, "merge-base", "HEAD", "origin/main")
        base_sha = git(root, "rev-parse", "--verify", f"{base_ref}^{{commit}}")
    if checkout:
        head_sha = git(root, "rev-parse", "HEAD")
        dirty = bool(git(root, "status", "--porcelain", "--untracked-files=no"))
    else:
        head_sha = tree_commit(root) or root.name
        dirty = False
    candidate_label = head_sha[:10] + (" with uncommitted changes" if dirty else "")
    suites = ALL_SUITES if options.only == "all" else [suite.strip() for suite in options.only.split(",") if suite.strip()]
    unknown = [suite for suite in suites if suite not in SUITES]
    if unknown:
        raise GateError(f"unknown suites: {', '.join(unknown)} (known: {', '.join([*ALL_SUITES, *OPT_IN_SUITES])})")
    if "browser" in suites and not options.browser_site:
        raise GateError("the browser suite needs --browser-site: the gate does not build browser sites")
    window_scenes = [scene.strip() for scene in options.window_scenes.split(",") if scene.strip()]
    if options.window_device and "window" not in suites:
        raise GateError("--window-device is the device of the window suite, which is not among the suites")
    if options.device_id and not options.window_device:
        raise GateError("--device-id names the device of --window-device")

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
        "browser": {},
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

    # The browser suite is measured only against a site of the base; its budget needs none.
    browser = "browser" in suites
    if browser and not options.browser_base_site:
        suites = [suite for suite in suites if suite != "browser"]
    timed = [suite for suite in suites if suite != "counters"]
    what = [f"{options.rounds} rounds of {', '.join(timed)}"] if timed else []
    if browser:
        what.insert(0, "the budget of the page switches in a browser")
    if "counters" in suites:
        what.insert(0, "the counter checks")
    print(f"base: {base_ref} ({base_sha[:10]}), candidate: {candidate_label}, {' and '.join(what)}")
    print(f"scratch: {scratch}; cargo: {' '.join(cargo_command())}")
    device = None
    if options.window_device and options.dry_run:
        device = Device(options.window_device, options.device_id, "arm64-v8a")
    elif options.window_device:
        device = find_device(options.window_device, options.device_id)
    if device is not None:
        print(f"the window suite runs on {device}")
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
    if browser and options.dry_run:
        print(f"run:   node {BROWSER_SCRIPT} {options.browser_site} --json (the budget of the page switches)")
    elif browser:
        check_rows += run_browser_budget(candidate, options.browser_site, logs, options.verbose)
    if check_rows and not suites:
        failures = [row for row in check_rows if row.regressed]
        print()
        print(text_table(check_rows))
        print()
        print(f"{len(check_rows)} checks, {len(failures)} failed" if failures else f"{len(check_rows)} checks, none failed")
        print(f"logs: {logs}")
        return 1 if failures else 0

    if options.dry_run:
        for suite in suites:
            variables = " ".join(f"{name}={value}" for name, value in env[suite].items())
            if suite == "browser":
                print(f"run:   node {BROWSER_SCRIPT} <site> --json, on {options.browser_base_site} and {options.browser_site} in turn")
                continue
            if suite == "window" and device is not None:
                script = "scripts/android/apk.sh package --no-cargo" if device.kind == "android" else "scripts/ios/bundle.sh --no-build"
                print(f"build: {' '.join(cargo_command() + device_build_args(device))}, then {script} and the install on the device")
                print(f"run:   {DEVICE_APPLICATION}.<side> on the device with FRAMES_SCENE={','.join(window_scenes)}, {options.window_seconds:g} s each")
                continue
            print(f"build: {' '.join(cargo_command() + SUITES[suite]['build'])}")
            if suite == "window":
                print(f"run:   {FRAMES_BINARY} with FRAMES_SCENE={','.join(window_scenes)}, {options.window_seconds:g} s each")
            elif suite == "frames":
                print(f"run:   each test in a process of its own: {variables + ' ' if variables else ''}{' '.join(cargo_command() + FRAMES_TEST)} <test> --exact {' '.join(TEST_ARGS)}")
            else:
                print(f"run:   {variables + ' ' if variables else ''}{' '.join(cargo_command() + SUITES[suite]['run'])}")
                if suite in HARNESS_SUITES:
                    print(f"run:   each class of the benchmarks in a process of its own, with {BENCH_CLASS}=<category::Class>")
        return 0

    # The browser suite runs the candidate's script on two sites that exist: no worktree of the base.
    built = [suite for suite in suites if SUITES[suite]["build"]]
    base_root, copied = root, []
    if built:
        base_root = base_dir if base_dir is not None else checkout_base(root, scratch, base_sha)
        copied = copy_benchmarks(root, base_root, suites, options.copy_gate, device)
    if copied:
        reason = "--copy-gate" if options.copy_gate else "the base has no benchmark code of its own for them"
        print(f"note: {reason}: the candidate's {', '.join(copied)} were copied into the base, so both sides run the same benchmark code")

    base = Side("base", base_root, scratch / "target-base", f"{base_ref} ({base_sha[:10]})")

    # Both sides are built before anything is measured: a build does not run beside a measurement.
    for suite in built:
        for side in (base, candidate):
            if suite == "window" and device is not None:
                build_device(side, device, scratch, logs, options.verbose)
            else:
                run_cargo(side, SUITES[suite]["build"], env[suite], logs / f"build-{suite}-{side.name}.log", options.verbose)

    frames: dict[str, list[str]] = {}
    if "frames" in suites:
        for side in (base, candidate):
            frames[side.name] = frame_tests(side, env["frames"], logs / f"list-frames-{side.name}.log", options.verbose)

    # The classes of the benchmarks of each harness suite, a side: each runs in a process of its
    # own, when the harness of both sides can run one class (else both run all in one process,
    # so that the two are measured the same way).
    classes: dict[tuple[str, str], list[str] | None] = {}
    for suite in suites:
        if suite not in HARNESS_SUITES:
            continue
        one_class = runs_one_class(base) and runs_one_class(candidate)
        if not one_class:
            print(f"note: the harness of a side cannot run one class ({BENCH_CLASS}): the {suite} benchmarks of both run in one process")
        for side in (base, candidate):
            classes[suite, side.name] = (
                benchmark_classes(side, suite, env[suite], logs / f"list-{suite}-{side.name}.log", options.verbose)
                if one_class
                else None
            )

    suite_of: dict[str, str] = {}

    def measure(round_index: int, measured: list[str], of: int) -> None:
        # The side that goes first changes with the round: what a run leaves behind (the caches
        # of the system, the heat of the machine) is then not always the other side's start.
        sides = (base, candidate) if round_index % 2 == 0 else (candidate, base)
        for suite in measured:
            for side in sides:
                print(f"round {round_index + 1} of {of}: {suite}, {side.name}", flush=True)
                log = logs / f"{suite}-{side.name}-{round_index + 1}.log"
                if suite == "browser":
                    site = options.browser_base_site if side is base else options.browser_site
                    found, _ = run_browser(root, side.name, site, log, options.verbose)
                    record(side, found)
                    suite_of.update((metric.key, suite) for metric, _ in found)
                    continue
                if suite == "window":
                    for scene in window_scenes:
                        scene_log = logs / f"window-{scene.replace('+', '-')}-{side.name}-{round_index + 1}.log"
                        if device is not None:
                            found = run_device_window(side, device, scene, options.window_seconds, scene_log, options.verbose)
                        else:
                            found = run_window(side, scene, options.window_seconds, scene_log, options.verbose)
                        record(side, found)
                        suite_of.update((metric.key, suite) for metric, _ in found)
                    continue
                if suite == "frames":
                    output = run_frames(side, frames[side.name], env[suite], log, options.verbose)
                elif suite in HARNESS_SUITES:
                    output = run_benchmarks(side, suite, classes[suite, side.name], env[suite], log, options.verbose)
                else:
                    output = run_cargo(side, SUITES[suite]["run"], env[suite], log, options.verbose)
                if suite in ("gauntlet", "frames"):
                    found = parse_gate(output)
                    if not found:
                        raise GateError(f"[{side.name}] the {suite} run printed no GATE line; the log is {log}")
                    record(side, found)
                    suite_of.update((metric.key, suite) for metric, _ in found)
                else:
                    found, failed, instructions = parse_tdg(output)
                    if not found and not failed:
                        raise GateError(f"[{side.name}] the {suite} run printed no result; the log is {log}")
                    record(side, found, instructions)
                    suite_of.update((metric.key, suite) for metric, _ in found)
                    side.failed |= failed

    for round_index in range(options.rounds):
        measure(round_index, suites, options.rounds)

    rounds = options.rounds
    while rounds < options.max_rounds:
        doubtful = [row.key for row in compare(base, candidate, options.tolerance) if row.measured and is_within_spread(base, candidate, row.key)]
        if not doubtful:
            break
        measured = [suite for suite in suites if any(suite_of.get(key) == suite for key in doubtful)]
        print(f"over the tolerance by less than the spread of their rounds: {', '.join(doubtful)}; measuring another round", flush=True)
        measure(rounds, measured, rounds + 1)
        rounds += 1

    rows = [*check_rows, *compare(base, candidate, options.tolerance)]
    regressions = [row for row in rows if row.regressed]
    time_alone = sum(row.verdict == "REGRESSED (time alone)" for row in regressions)
    summary = (
        f"{len(rows)} metrics, {len(regressions)} regressed beyond their tolerance"
        + (f" ({time_alone} of them in time alone, with the instructions of the base)" if time_alone else "")
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
        if device is not None:
            notes.append(f"The window suite ran on {device}.")
        if copied:
            notes.append(f"The candidate's {', '.join(f'`{folder}`' for folder in copied)} were copied into the base, so both sides run the same benchmark code.")
        lines = [
            "### Local performance gate",
            "",
            f"Base `{base_sha[:10]}` ({base_ref}), candidate `{candidate_label}`; {options.rounds} rounds"
            + (f" (up to {rounds} where a metric was over its tolerance within the spread of its rounds)" if rounds > options.rounds else "")
            + ", the two sides alternately and each first in turn, "
            f"median of the rounds; tolerance {options.tolerance * 100:.0f} % times the factor of the metric "
            "(`scripts/perf/gate.py`). Frame gate: 1280 x 820 at a scaling of 2, raster Skia, render-thread mode. "
            f"TreeDataGrid: median time of an operation, {options.tdg_iterations} iterations of at least "
            f"{options.tdg_min_iteration_ms} ms after {options.tdg_warmup} of warm-up"
            + (f", filter `{options.tdg_filter}`" if options.tdg_filter else "")
            + (
                ". Benchmarks: a process per class; Instructions is the change of the instructions an "
                f"operation retires (tolerance {INSTRUCTIONS_TOLERANCE * 100:g} %), and a time over its "
                "tolerance with instructions within theirs is `REGRESSED (time alone)`"
                if any(row.instructions for row in rows)
                else ""
            )
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
