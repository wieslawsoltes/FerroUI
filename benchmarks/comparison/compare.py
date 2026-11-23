#!/usr/bin/env python3
"""The framework comparison's FerroUI application, built two ways and measured by the
comparison's own harness: from the FerroUI sources of this checkout (`local`) and from the
published crates the comparison names (`official`).

    python3 benchmarks/comparison/compare.py build            # both sides
    python3 benchmarks/comparison/compare.py build local
    python3 benchmarks/comparison/compare.py run --scenario gauntlet:16 --rounds 2
    python3 benchmarks/comparison/compare.py run --sides local --scenario patchwork:17 --diagnostic

The sources of the application are the comparison's, unchanged, in the submodule `upstream`
(`git submodule update --init --checkout benchmarks/comparison/upstream`; a clone does not fetch
it by itself). `local` is `ferroui-local/Cargo.toml`, which names the crates of this checkout;
`official` is the comparison's own `ferro-app/Cargo.toml`, which names a published version.
Both are built in one target folder (`CARGO_TARGET_DIR`, else `benchmarks/comparison/target`)
and with the cargo of `FERROUI_CARGO` (else `cargo`).

`run` starts the comparison's desktop harness (`upstream/.../desktop.py`)
once a side, with that side's program in the place of its `ferro` application, and prints the
two beside each other: for a first start and a second start of each scenario, the frames a
second over the run, the time to the first frame and the CPU a frame. The harness needs macOS,
a display at 60 Hz and its frame counter installed (`upstream/.../framecount/build.sh`, which
asks for the Screen Recording permission once); it refuses to measure a display on which
something else moves. `--diagnostic` measures anyway and says so: numbers to work with, not to
publish. Every other argument goes to the harness (`--scenario NAME[:TIER]`, `--rounds N`,
`--run SECONDS`, `--tier N`).
"""

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
UPSTREAM = HERE / "upstream" / "benchmarks" / "compose-vs-cranpose"
SIDES = {
    # side: (manifest, name of the program it builds)
    "local": (HERE / "ferroui-local" / "Cargo.toml", "perf-compare-ferro-local"),
    "official": (UPSTREAM / "ferro-app" / "Cargo.toml", "perf-compare-ferro"),
}
METRICS = (
    ("fps", "fps", "{:.1f}"),
    ("first_frame_s", "first frame s", "{:.3f}"),
    ("cpu_ms_per_frame", "CPU ms/frame", "{:.1f}"),
)


def target_dir() -> Path:
    return Path(os.environ.get("CARGO_TARGET_DIR") or HERE / "target")


def program(side: str) -> Path:
    return target_dir() / "release" / SIDES[side][1]


def need_upstream() -> None:
    if not (UPSTREAM / "desktop.py").is_file():
        sys.exit(
            "the submodule of the comparison is not checked out:\n"
            "  git submodule update --init --checkout benchmarks/comparison/upstream"
        )


def build(sides: list[str]) -> None:
    need_upstream()
    cargo = os.environ.get("FERROUI_CARGO", "cargo")
    for side in sides:
        manifest, _ = SIDES[side]
        command = [cargo, "build", "--release", "--manifest-path", str(manifest), "--target-dir", str(target_dir())]
        print(f"[{side}] {' '.join(command)}", flush=True)
        if subprocess.run(command).returncode != 0:
            sys.exit(f"[{side}] the build failed")
        print(f"[{side}] {program(side)} ({program(side).stat().st_size / 1e6:.1f} MB)", flush=True)


def harness(binary: str, diagnostic: bool, arguments: list[str]) -> None:
    """The comparison's harness in this process, with `binary` as its `ferro` application."""
    sys.path.insert(0, str(UPSTREAM))
    import desktop  # noqa: PLC0415 (the harness of the submodule)

    desktop.APPS["ferro"] = [Path(binary)]
    if diagnostic:
        observe = desktop.observe_display

        def lenient(stage, label):
            state, problems = observe(stage, label)
            if problems:
                print("diagnostic run, display check skipped:", "; ".join(problems), flush=True)
            return state, []

        desktop.observe_display = lenient
    sys.argv = ["desktop.py", *arguments]
    desktop.main()


def summaries(report: Path) -> dict[tuple[str, str], dict]:
    """By scenario and start (`first`, `second`): the harness's summary of the `ferro` application."""
    found = {}
    data = json.loads(report.read_text())
    for index, scenario in enumerate(data.get("scenarios", [])):
        # The harness names the screen and, in `extras`, its tier ("tier 16, 1280 x 820 ...").
        name = str(scenario.get("scenario") or f"scenario {index + 1}")
        tier = str(scenario.get("extras", "")).split(",")[0].strip()
        label = f"{name}, {tier}" if tier else name
        for start, body in scenario.get("starts", {}).items():
            summary = body.get("summary", {}).get("ferro")
            if summary:
                found[(label, start)] = summary
    return found


def run(sides: list[str], diagnostic: bool, output: Path, arguments: list[str]) -> None:
    need_upstream()
    results = {}
    for side in sides:
        if not program(side).is_file():
            sys.exit(f"{program(side)} is not built: python3 {Path(__file__).name} build {side}")
        out = output / side
        script = str(Path(__file__).resolve())
        command = [sys.executable, script, "_harness", str(program(side)), "--output", str(out), "--apps", "ferro"]
        if diagnostic:
            command.append("--diagnostic")
        print(f"[{side}] the comparison's harness: {' '.join(arguments)}", flush=True)
        if subprocess.run([*command, *arguments], cwd=ROOT).returncode != 0:
            sys.exit(f"[{side}] the harness failed")
        results[side] = summaries(out / "report.json")
    keys = list(dict.fromkeys(key for side in sides for key in results[side]))
    header = ["scenario", "start", *(f"{title} ({side})" for _, title, _ in METRICS for side in sides)]
    rows = [header]
    for scenario, start in keys:
        row = [scenario, start]
        for metric, _, shape in METRICS:
            for side in sides:
                value = results[side].get((scenario, start), {}).get(metric)
                row.append(shape.format(value) if isinstance(value, (int, float)) else "-")
        rows.append(row)
    widths = [max(len(row[column]) for row in rows) for column in range(len(header))]
    print()
    if diagnostic:
        print("diagnostic run: the display was not checked")
    for index, row in enumerate(rows):
        cells = (cell.ljust(widths[at]) if at < 2 else cell.rjust(widths[at]) for at, cell in enumerate(row))
        print("  ".join(cells))
        if index == 0:
            print("  ".join("-" * width for width in widths))


def main() -> None:
    if len(sys.argv) > 1 and sys.argv[1] == "_harness":
        binary, rest = sys.argv[2], sys.argv[3:]
        diagnostic = "--diagnostic" in rest
        harness(binary, diagnostic, [argument for argument in rest if argument != "--diagnostic"])
        return
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    build_parser = commands.add_parser("build", help="build a side, or both")
    build_parser.add_argument("side", nargs="?", default="both", choices=[*SIDES, "both"])
    run_parser = commands.add_parser("run", help="measure the sides with the comparison's harness")
    run_parser.add_argument("--sides", default="local,official", help="comma separated: local, official (default both)")
    run_parser.add_argument("--diagnostic", action="store_true", help="measure although the display is busy")
    run_parser.add_argument(
        "--output",
        type=Path,
        default=HERE / "results",
        help="where the reports go (default benchmarks/comparison/results)",
    )
    options, rest = parser.parse_known_args()
    if options.command == "build":
        if rest:
            parser.error(f"unknown arguments: {' '.join(rest)}")
        build(list(SIDES) if options.side == "both" else [options.side])
    else:
        sides = [side.strip() for side in options.sides.split(",") if side.strip()]
        unknown = [side for side in sides if side not in SIDES]
        if unknown or not sides:
            parser.error(f"--sides takes local and official, not {', '.join(unknown) or 'nothing'}")
        run(sides, options.diagnostic, options.output, rest)


if __name__ == "__main__":
    main()
