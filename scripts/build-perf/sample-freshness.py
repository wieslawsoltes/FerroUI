#!/usr/bin/env python3
"""Measure unchanged sample builds and test optional-file invalidation with real Cargo.

The graph workload is `cargo build --lib -p treedatagrid-demo --no-default-features`
(or the desktop catalog with --catalog). Initial target population is reported
separately. At least three subsequent baseline/candidate pairs alternate order;
neither revision uses the optional build-tool optimization config. The regression
fixture calls the actual SampleBuild library, consumes its generated assets.rs,
and checks creation, edit and deletion of excluded.txt, Rust source edits and asset
edits by running a dependent executable. Only disposable fixture files are edited.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time

from compare import generated_outputs, identity, normalized, units


FIXTURE_PACKAGE = "sample-freshness-fixture"
CONSUMER_PACKAGE = "sample-freshness-consumer"


def command_text(command: list[str], cwd: Path, env: dict) -> str:
    return subprocess.check_output(command, cwd=cwd, env=env, text=True).strip()


def write_json(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, indent=2) + "\n")


def definitions(record: dict) -> list[dict]:
    return sorted(({key: value for key, value in unit.items() if key != "fresh"}
                   for unit in record["units"]), key=lambda value: json.dumps(value, sort_keys=True))


def is_fresh(record: dict) -> bool:
    return bool(record["units"]) and all(unit["fresh"] for unit in record["units"])


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def graph_inputs(source: Path, catalog: bool) -> dict:
    directories = [source / "samples/TreeDataGridDemo"]
    if catalog:
        directories.extend([source / "samples/ControlCatalog", source / "samples/ControlCatalog.Desktop"])
    return {path.relative_to(source).as_posix(): sha256(path)
            for directory in directories for path in sorted(directory.rglob("*"))
            if path.is_file()}


def build(source: Path, cwd: Path, target: Path, output: Path, label: str, phase: str,
          package: str, env: dict, jobs: int | None, *, library: bool = False,
          no_default_features: bool = False, fixture: bool = False) -> dict:
    destination = output / label / phase
    destination.mkdir(parents=True)
    command = ["cargo", "build", "--locked", "--offline", "--timings",
               "--message-format=json-render-diagnostics", "--verbose", "--verbose", "-p", package]
    if library:
        command.append("--lib")
    if no_default_features:
        command.append("--no-default-features")
    if jobs is not None:
        command.extend(["--jobs", str(jobs)])
    build_env = dict(env, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target))
    print(f"{label} {phase}: {' '.join(command)}", flush=True)
    started = time.perf_counter()
    with (destination / "cargo.jsonl").open("w") as stdout, (destination / "cargo.stderr").open("w") as stderr:
        result = subprocess.run(command, cwd=cwd, env=build_env, stdout=stdout, stderr=stderr)
    seconds = time.perf_counter() - started
    messages = []
    for line in (destination / "cargo.jsonl").read_text().splitlines():
        try:
            messages.append(json.loads(line))
        except json.JSONDecodeError:
            pass  # The unmodified line remains in the raw log.
    stderr_text = (destination / "cargo.stderr").read_text()
    record = {"label": label, "phase": phase, "seconds": seconds, "command": command,
              "exit_code": result.returncode, "units": units(messages, source, target),
              "dirty_reasons": [normalized(line.strip(), source, target)
                                for line in stderr_text.splitlines() if line.lstrip().startswith("Dirty ")]}
    if fixture:
        record = json.loads(normalized(json.dumps(record), cwd, target))
    if result.returncode == 0:
        if not any(message.get("reason") == "build-finished" and message.get("success") for message in messages):
            raise RuntimeError(f"Cargo reported no successful build-finished message; see {destination}")
        generated_messages = messages
        if fixture:
            generated_messages = [message for message in messages
                                  if f"#{FIXTURE_PACKAGE}@" in message.get("package_id", "")]
        record["generated"] = generated_outputs(generated_messages, cwd if fixture else source, target)
        record["all_units_fresh"] = is_fresh(record)
        if fixture:
            executables = [message["executable"] for message in messages
                           if message.get("reason") == "compiler-artifact"
                           and message["target"]["name"] == CONSUMER_PACKAGE and message.get("executable")]
            if len(executables) != 1:
                raise RuntimeError(f"Expected one fixture consumer executable, got {executables}")
            with (destination / "consumer.stdout").open("w") as stdout, (destination / "consumer.stderr").open("w") as stderr:
                subprocess.run(executables, cwd=cwd, env=build_env, stdout=stdout, stderr=stderr, check=True)
            record["consumer_stdout"] = (destination / "consumer.stdout").read_text()
    timing = target / "cargo-timings/cargo-timing.html"
    if timing.exists():
        shutil.copyfile(timing, destination / "cargo-timing.html")
    write_json(destination / "result.json", record)
    if result.returncode:
        raise RuntimeError(f"{label} {phase} failed ({result.returncode}); see {destination}\n{stderr_text[-6000:]}")
    print(f"{label} {phase}: {seconds:.3f}s; all units fresh: {record['all_units_fresh']}", flush=True)
    return record


def make_fixture(directory: Path, source: Path, env: dict) -> None:
    (directory / "fixture/Assets").mkdir(parents=True)
    (directory / "consumer").mkdir()
    (directory / "Cargo.toml").write_text('[workspace]\nmembers = ["fixture", "consumer"]\nresolver = "3"\n')
    (directory / "fixture/Cargo.toml").write_text(
        '[package]\nname = "sample-freshness-fixture"\nversion = "0.1.0"\nedition = "2024"\n'
        '[lib]\npath = "lib.rs"\n[build-dependencies]\nsample-build = { path = '
        + json.dumps(str(source / "samples/SampleBuild")) + ' }\n')
    (directory / "fixture/build.rs").write_text(
        'fn main() { sample_build::SampleBuild::new("FreshnessFixture").run(); }\n')
    (directory / "fixture/lib.rs").write_text(
        'include!(concat!(env!("OUT_DIR"), "/assets.rs"));\n'
        'pub fn excluded() -> &\'static [(&\'static str, bool, &\'static str)] { EXCLUDED }\n'
        'pub fn marker() -> &\'static str { "source-one" }\n'
        'pub fn asset() -> &\'static [u8] { ASSETS[0].1 }\n')
    (directory / "fixture/Assets/value.txt").write_text("asset-one")
    (directory / "consumer/Cargo.toml").write_text(
        '[package]\nname = "sample-freshness-consumer"\nversion = "0.1.0"\nedition = "2024"\n'
        '[[bin]]\nname = "sample-freshness-consumer"\npath = "main.rs"\n[dependencies]\n'
        'sample-freshness-fixture = { path = "../fixture" }\n')
    (directory / "consumer/main.rs").write_text(
        'fn main() {\n'
        ' println!("{}|{}", sample_freshness_fixture::marker(), '
        'std::str::from_utf8(sample_freshness_fixture::asset()).unwrap());\n'
        ' for (path, page, reason) in sample_freshness_fixture::excluded() { '
        'println!("{path}|{page}|{reason}"); }\n}\n')
    shutil.copyfile(source / "Cargo.lock", directory / "Cargo.lock")
    # Update only the disposable fixture's lockfile, retaining the copied repository
    # dependency versions. All timed builds stay locked/offline.
    subprocess.run(["cargo", "metadata", "--offline", "--format-version=1"], cwd=directory,
                   env=env, stdout=subprocess.DEVNULL, check=True)


def fixture_checks(sources: dict[str, Path], temporary: Path, output: Path, env: dict,
                   jobs: int | None, report: dict, save) -> None:
    reference_outputs: dict[str, dict] = {}
    for label, source in sources.items():
        directory = temporary / f"fixture-{label}"
        make_fixture(directory, source, env)
        target = temporary / f"target-{label}"
        excluded = directory / "fixture/excluded.txt"
        rust_source = directory / "fixture/lib.rs"
        asset = directory / "fixture/Assets/value.txt"
        marker, asset_text, exclusion = "source-one", "asset-one", ""
        initial_generated = None
        previous_generated = None
        cases = [("populate", None), ("absent-repeat", None),
                 ("create", "create"), ("create-repeat", None),
                 ("edit", "edit"), ("edit-repeat", None),
                 ("delete", "delete"), ("delete-repeat", None),
                 ("source-edit", "source"), ("source-repeat", None),
                 ("asset-edit", "asset"), ("asset-repeat", None)]
        for phase, change in cases:
            if change == "create":
                excluded.write_text("Assets/value.txt | first reason\n")
                exclusion = "/Assets/value.txt|false|first reason\n"
            elif change == "edit":
                excluded.write_text("Assets/value.txt | page: second reason\n")
                exclusion = "/Assets/value.txt|true|second reason\n"
            elif change == "delete":
                excluded.unlink()
                exclusion = ""
            elif change == "source":
                rust_source.write_text(rust_source.read_text().replace('"source-one"', '"source-two"'))
                marker = "source-two"
            elif change == "asset":
                asset.write_text("asset-two")
                asset_text = "asset-two"
            record = build(source, directory, target, output / "fixture", label, phase,
                           CONSUMER_PACKAGE, env, jobs, fixture=True)
            expected = f"{marker}|{asset_text}\n{exclusion}"
            record["runtime_output_correct"] = record["consumer_stdout"] == expected
            if not record["runtime_output_correct"]:
                raise RuntimeError(f"{label} fixture {phase}: stale or incorrect generated/compiled output")
            if not record["generated"]:
                raise RuntimeError("SampleBuild fixture produced no generated-output evidence")
            if initial_generated is None:
                initial_generated = record["generated"]
            if change in {"create", "edit", "delete"} and record["generated"] == previous_generated:
                raise RuntimeError(f"{label} fixture {phase}: generated exclusion table did not change")
            if phase == "delete" and record["generated"] != initial_generated:
                raise RuntimeError(f"{label}: deleting excluded.txt did not restore the original generated files")
            if change:
                affected = [unit for unit in record["units"]
                            if unit["name"] in {FIXTURE_PACKAGE.replace('-', '_'), CONSUMER_PACKAGE}
                            and "custom-build" not in unit["kind"]]
                if len(affected) != 2 or any(unit["fresh"] for unit in affected):
                    raise RuntimeError(f"{label} fixture {phase}: change did not rebuild fixture and consumer")
            if label == "candidate" and phase.endswith("repeat") and not record["all_units_fresh"]:
                raise RuntimeError(f"candidate fixture {phase}: unchanged build was unnecessarily rebuilt")
            equivalence = {"generated": record["generated"], "runtime": record["consumer_stdout"],
                           "units": definitions(record)}
            if label == "baseline":
                reference_outputs[phase] = equivalence
            elif equivalence != reference_outputs[phase]:
                raise RuntimeError(f"fixture {phase}: baseline/candidate generated output, runtime or graph differs")
            previous_generated = record["generated"]
            report["fixture"].append(record)
            save()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--jobs", type=int)
    parser.add_argument("--catalog", action="store_true", help="Build the complete desktop catalog instead of the TreeDataGrid library")
    parser.add_argument("--fixture-only", action="store_true", help="Run the real-SampleBuild regression fixture without the larger graph benchmark")
    parser.add_argument("--offline", action="store_true", help="Dependencies are already downloaded; skip the untimed cargo fetch")
    parser.add_argument("--minimum-speedup", type=float, default=0,
                        help="Opt-in acceptance gate for unchanged graph rebuilds only; 0 records the measured ratio")
    args = parser.parse_args()
    if args.repetitions < 3 or (args.jobs is not None and args.jobs < 1):
        parser.error("use at least three repetitions and a positive job count")
    if not math.isfinite(args.minimum_speedup) or args.minimum_speedup < 0:
        parser.error("minimum-speedup must be finite and nonnegative")
    if args.fixture_only and args.minimum_speedup:
        parser.error("a speedup gate requires the actual graph benchmark")
    sources = {"baseline": args.baseline.resolve(), "candidate": args.candidate.resolve()}
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, CARGO_TERM_COLOR="never", RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="",
               CARGO_BUILD_RUSTC_WRAPPER="", CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER="")
    rustc = env.get("RUSTC", "rustc")
    for command in ([rustc, "-Vv"], ["cargo", "-V"]):
        if len({command_text(command, source, env) for source in sources.values()}) != 1:
            parser.error("both revisions must use exactly the same Cargo and rustc")
    report = {"schema_version": 1,
              "scenario": "sample-build-input-invalidation-fixture" if args.fixture_only else "unchanged-sample-build-after-target-population",
              "toolchain": command_text([rustc, "-Vv"], sources["candidate"], env),
              "cargo": command_text(["cargo", "-V"], sources["candidate"], env),
              "harness_sha256": sha256(Path(__file__)),
              "platform": platform.platform(), "cpu_count": os.cpu_count(),
              "sources": {label: {**identity(source),
                                  "sample_build_sha256": sha256(source / "samples/SampleBuild/lib.rs"),
                                  "sample_inputs": graph_inputs(source, args.catalog)}
                          for label, source in sources.items()},
              "environment": {key: value for key, value in env.items()
                              if key.startswith("CARGO_PROFILE_") or key in {
                                  "CARGO_INCREMENTAL", "CARGO_BUILD_JOBS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"}},
              "explicit_build_tool_optimization": False, "population": [], "repeats": [], "fixture": []}
    report["requested_minimum_repeat_speedup"] = args.minimum_speedup

    def save() -> None:
        write_json(output / "report.json", report)

    save()
    try:
        if report["sources"]["baseline"]["sample_inputs"] != report["sources"]["candidate"]["sample_inputs"]:
            raise RuntimeError("Sample input files differ between revisions; select equivalent workloads")
        if not args.offline:
            for source in sources.values():
                subprocess.run(["cargo", "fetch", "--locked"], cwd=source, env=env, check=True)
        with tempfile.TemporaryDirectory(prefix="ferroui-sample-freshness-") as temporary_path:
            temporary = Path(temporary_path).resolve()
            if not args.fixture_only:
                package = "control-catalog-desktop" if args.catalog else "treedatagrid-demo"
                reference = None
                for label, source in sources.items():
                    target = temporary / f"target-{label}"
                    record = build(source, source, target, output / "graph", label, "populate", package,
                                   env, args.jobs, library=not args.catalog, no_default_features=not args.catalog)
                    equivalent = (definitions(record), record["generated"])
                    if reference is None:
                        reference = equivalent
                    elif equivalent != reference:
                        raise RuntimeError("Baseline/candidate graph units, features, profiles or generated files differ")
                    report["population"].append(record)
                    save()
                for repetition in range(1, args.repetitions + 1):
                    order = list(sources.items())
                    if repetition % 2 == 0:
                        order.reverse()
                    for label, source in order:
                        record = build(source, source, temporary / f"target-{label}", output / "graph", label,
                                       f"repeat-{repetition}", package, env, args.jobs,
                                       library=not args.catalog, no_default_features=not args.catalog)
                        if (definitions(record), record["generated"]) != reference:
                            raise RuntimeError(f"{label} repeat {repetition}: graph or generated output changed")
                        if label == "candidate" and not record["all_units_fresh"]:
                            raise RuntimeError("Unchanged candidate graph rebuilt; inspect recorded dirty reasons")
                        report["repeats"].append(record)
                        save()
            fixture_checks(sources, temporary, output, env, args.jobs, report, save)
        medians = {label: statistics.median(row["seconds"] for row in report["repeats"] if row["label"] == label)
                   for label in sources} if report["repeats"] else {}
        report["summary"] = {"repeat_median_seconds": medians,
                             "repeat_speedup": medians["baseline"] / medians["candidate"] if medians else None,
                             "initial_population_seconds": {row["label"]: row["seconds"] for row in report["population"]},
                             "same_graph_features_profiles_and_generated_files": True if medians else None,
                             "candidate_unchanged_builds_all_fresh": True,
                             "optional_file_and_source_and_asset_invalidation": True,
                             "fixture_runtime_and_generated_outputs_equal": True}
        report["summary"]["requested_speedup_gate_passed"] = (
            report["summary"]["repeat_speedup"] >= args.minimum_speedup
            if args.minimum_speedup else None)
        save()
        text = "## Sample build freshness\n\n"
        if medians:
            text += ("| Workload | Baseline median | Candidate median | Speedup |\n|---|---:|---:|---:|\n"
                     f"| Unchanged repeated Cargo build | {medians['baseline']:.3f}s | {medians['candidate']:.3f}s | "
                     f"{report['summary']['repeat_speedup']:.2f}× |\n\n"
                     "Initial population of each target is reported separately; these numbers do not measure cold compilation. "
                     "The same real dependency graph, features, profiles and generated Rust/metadata were verified across "
                     "all alternating repeats. No optional build-tool optimization config was used.\n\n")
        text += ("The regression fixture uses the real SampleBuild library and runs a dependent executable. "
                 "Creating, editing and deleting the optional exclusion file, editing Rust source and editing an asset "
                 "all rebuilt the affected crate and consumer with the correct runtime output. Every unchanged candidate "
                 "build was fresh. Generated files and runtime results agree with the baseline. Raw Cargo JSON, verbose "
                 "dirty reasons, timings and output hashes are in the artifact.\n")
        if args.minimum_speedup:
            text += (f"\nRequested unchanged-build threshold: **{args.minimum_speedup:.2f}×**; "
                     f"passed: **{report['summary']['requested_speedup_gate_passed']}**.\n")
        (output / "summary.md").write_text(text)
        print(text)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as handle:
                handle.write(text)
        if args.minimum_speedup and not report["summary"]["requested_speedup_gate_passed"]:
            raise RuntimeError(f"Unchanged-build speedup {report['summary']['repeat_speedup']:.3f}x is below "
                               f"the requested {args.minimum_speedup:.3f}x; cold population is not part of this ratio")
        return 0
    except (RuntimeError, subprocess.CalledProcessError, OSError) as error:
        report["error"] = str(error)
        save()
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
