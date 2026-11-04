#!/usr/bin/env python3
"""Run focused development gates against one immutable repository revision.

This compiles actual framework test crates for the selected source change. The
metadata suite checks Base and downstream Controls expansion; the control-theme
suites check the runtime, emitter and differential corpus, then type-check fresh
WinUI output. The explicit control-theme-regions suite additionally requires the
region runtime/grammar cases and both generated program forms. Filters narrow
execution, not compilation. The coercion suite checks owned-value cleanup,
metadata registration/inheritance, property behavior and downstream typed APIs.
These development suites make no
production size, throughput, PGO or shipping-acceptance claim. Toolchain, commands,
source identity, lockfile, logs and resource reports survive success or failure.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time


HERE = Path(__file__).resolve().parent
HOST = "x86_64-unknown-linux-gnu"

# Counts belong only to the explicit frozen region/coercion suites. Existing
# metadata and whole-theme command plans and acceptance rules remain unchanged.
REGION_EXPECTED_SUMMARIES = {
    "construction-runtime": {"passed": 2, "failed": 0, "ignored": 0},
    "region-runtime": {"passed": 3, "failed": 0, "ignored": 0},
    "construction-grammar": {"passed": 6, "failed": 0, "ignored": 0},
    "region-grammar": {"passed": 7, "failed": 0, "ignored": 0},
    "emitter-differential": {"passed": 25, "failed": 0, "ignored": 2},
}

COERCION_EXPECTED_SUMMARIES = {
    "coercion-oracles": {"passed": 5, "failed": 0, "ignored": 0},
    "property-runtime": {"passed": 307, "failed": 0, "ignored": 0},
    "property-metadata": {"passed": 14, "failed": 0, "ignored": 0},
    "coercer-registration": {"passed": 1, "failed": 0, "ignored": 0},
    "metadata": {"passed": 66, "failed": 0, "ignored": 0},
}


def identity(path):
    path = Path(path).resolve(strict=True)
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest}


def save(path, value):
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def git(source, *arguments):
    return subprocess.check_output(["git", "-C", str(source), *arguments], text=True).strip()


def suite_commands(cargo, suite):
    """Fixed command plans: an input selects a suite, never arbitrary shell code."""
    common = [cargo, "test", "--locked", "--target", HOST, "--lib"]
    if suite == "metadata":
        base = [*common, "-p", "ferroui-base", "--features", "compiler-metadata,hot-reload"]
        tests = [
            (label, [*base, test_filter, "--", "--test-threads=1"])
            for label, test_filter in (("metadata", "metadata::"), ("object-smoke", "ferro_object_tests::"),
                                       ("properties", "tests::ferro_object_tests"))
        ]
        downstream = [cargo, "check", "--locked", "--target", HOST, "--lib", "-p", "ferroui-base",
                      "-p", "ferroui-controls", "--features", "ferroui-base/compiler-metadata,ferroui-base/hot-reload"]
    elif suite == "coercion":
        base = [*common, "-p", "ferroui-base", "--features", "compiler-metadata,hot-reload"]
        tests = [
            (label, [*base, test_filter, "--", "--test-threads=1"])
            for label, test_filter in (
                ("coercion-oracles", "property_store::effective_value::coercion_tests::"),
                ("property-runtime", "tests::ferro_object_tests"),
                ("property-metadata", "tests::ferro_property_tests::"),
                ("coercer-registration", "ferro_property::styled_property_options_tests::"),
                ("metadata", "metadata::"),
            )
        ]
        downstream = [cargo, "check", "--locked", "--target", HOST, "--lib", "-p", "ferroui-base",
                      "-p", "ferroui-controls", "--features", "ferroui-base/compiler-metadata,ferroui-base/hot-reload"]
    elif suite in {"control-theme", "control-theme-regions"}:
        tests = [
            ("construction-runtime", [*common, "-p", "ferroui-markup-xaml",
                                       "control_theme_program::tests::", "--", "--test-threads=1"]),
            ("construction-grammar", [*common, "-p", "ferroui-markup-xaml-loader", "--features", "emitter",
                                       "control_theme_programs::tests::", "--", "--test-threads=1"]),
            ("emitter-differential", [*common, "-p", "ferroui-markup-xaml-tests",
                                      "emitter::differential_tests::", "--", "--test-threads=1"]),
        ]
        if suite == "control-theme-regions":
            tests.insert(1, (
                "region-runtime", [*common, "-p", "ferroui-markup-xaml",
                                   "style_region::tests::", "--", "--test-threads=1"],
            ))
            tests.insert(3, (
                "region-grammar", [*common, "-p", "ferroui-markup-xaml-loader", "--features", "emitter",
                                   "control_theme_programs::regions::tests::", "--", "--test-threads=1"],
            ))
        downstream = [cargo, "check", "--locked", "--target", HOST, "--lib", "-p", "ferroui-themes-winui",
                      "--features", "solid-theme"]
    else:
        raise ValueError(f"unknown source validation suite: {suite}")
    return [*tests, ("downstream", downstream)], {label for label, _ in tests}


def successful_tests(contents, label):
    summaries = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", contents)
    if len(summaries) != 1 or int(summaries[0][0]) == 0 or int(summaries[0][1]) != 0:
        raise RuntimeError(f"{label} did not report one nonempty successful test suite")
    passed = set(re.findall(r"(?m)^test ([A-Za-z_][A-Za-z_0-9:]*)(?: - should panic)? \.\.\. ok$", contents))
    if len(passed) != int(summaries[0][0]):
        raise RuntimeError(f"{label} test names do not match its successful summary")
    return dict(zip(("passed", "failed", "ignored"), map(int, summaries[0]))), passed


def collect_generated_winui(target, evidence):
    generated = []
    for path in sorted(target.rglob("compiled_xaml.rs")):
        if any(part.startswith("ferroui-themes-winui-") for part in path.parts):
            row = identity(path)
            contents = path.read_text()
            row["construction_programs"] = contents.count("rt::control_theme_program::run(")
            row["region_programs"] = contents.count("rt::style_region::run(")
            retained = evidence / "generated" / f"{len(generated):03d}-{path.parent.parent.name}.rs"
            retained.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, retained)
            row["retained"] = identity(retained)
            generated.append(row)
    return generated


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--suite", choices=("metadata", "coercion", "control-theme", "control-theme-regions"), default="metadata")
    parser.add_argument("--required-test", action="append", default=[])
    parser.add_argument("--plan-only", action="store_true")
    args = parser.parse_args()
    source = args.source.resolve(strict=True)
    if not re.fullmatch(r"[0-9a-f]{40}", args.revision):
        parser.error("--revision must be a full immutable commit SHA")
    if any(not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9:]*", name) for name in args.required_test):
        parser.error("--required-test must be a Rust test path")
    if git(source, "rev-parse", "HEAD") != args.revision:
        parser.error("source checkout does not match --revision")
    if git(source, "status", "--porcelain", "--untracked-files=all"):
        parser.error("source checkout has modified or untracked inputs")
    output = args.output.resolve()
    if output.exists() or output.is_relative_to(source):
        parser.error("use a fresh output directory outside the source checkout")
    evidence = output / "evidence"
    evidence.mkdir(parents=True)
    temporary = output / "temporary"
    temporary.mkdir()
    prefix = output / "toolchain/install"
    environment = os.environ.copy()
    removed = []
    for key in list(environment):
        if key.startswith(("CARGO_PROFILE_", "FERROUI_PGO_")) or key in {
            "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC", "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER", "RUSTC_BOOTSTRAP", "LLVM_PROFILE_FILE",
        }:
            removed.append(key)
            del environment[key]
    controlled = {
        "TMPDIR": str(temporary), "CARGO_HOME": str(output / "cargo-home"),
        "CARGO_TARGET_DIR": str(output / "target"), "CARGO_BUILD_JOBS": "1",
        "CARGO_INCREMENTAL": "0", "RAYON_NUM_THREADS": "1", "MALLOC_ARENA_MAX": "1",
        "MALLOC_CONF": "narenas:1,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,background_thread:false,retain:false",
        "CARGO_TERM_COLOR": "never", "RUST_BACKTRACE": "1",
        "CARGO_PROFILE_DEV_OPT_LEVEL": "0", "CARGO_PROFILE_DEV_DEBUG": "0",
        "CARGO_PROFILE_DEV_DEBUG_ASSERTIONS": "true",
        "CARGO_PROFILE_DEV_CODEGEN_UNITS": "128", "CARGO_PROFILE_DEV_LTO": "false",
        "CARGO_PROFILE_TEST_OPT_LEVEL": "0", "CARGO_PROFILE_TEST_DEBUG": "0",
        "CARGO_PROFILE_TEST_DEBUG_ASSERTIONS": "true",
        "CARGO_PROFILE_TEST_CODEGEN_UNITS": "128", "CARGO_PROFILE_TEST_LTO": "false",
    }
    environment.update(controlled)
    commands = [
        ("bootstrap", [sys.executable, str(HERE / "bootstrap_toolchain.py"),
                       "--work-dir", str(output / "toolchain")]),
    ]
    cargo = str(prefix / "bin/cargo")
    planned, test_labels = suite_commands(cargo, args.suite)
    commands.extend(planned)
    report = {
        "schema": 2, "scope": __doc__, "suite": args.suite, "revision": args.revision,
        "tree": git(source, "rev-parse", "HEAD^{tree}"), "source": str(source),
        "producer": identity(__file__), "bootstrap_producer": identity(HERE / "bootstrap_toolchain.py"),
        "lockfile_before": identity(source / "Cargo.lock"), "commands": [],
        "environment": {"removed_names": sorted(removed), "controlled": controlled},
        "started_unix": time.time(), "complete": False, "plan_only": args.plan_only,
        "production_acceptance": False,
        "required_tests": args.required_test,
        "required_metadata_tests": args.required_test if args.suite == "metadata" else [],
    }
    if args.suite == "control-theme-regions":
        report["expected_test_summaries"] = REGION_EXPECTED_SUMMARIES
    elif args.suite == "coercion":
        report["expected_test_summaries"] = COERCION_EXPECTED_SUMMARIES
    tracked = evidence / "tracked-inputs.txt"
    tracked.write_text(git(source, "ls-files", "--stage") + "\n")
    report["tracked_inputs"] = identity(tracked)
    manifest = evidence / "validation.json"
    save(manifest, report)
    if args.plan_only:
        report["planned_commands"] = [{"label": label, "argv": command} for label, command in commands]
        save(manifest, report)
        print(manifest)
        return
    def interrupted(signum, _frame):
        raise InterruptedError(f"Source validation interrupted by signal {signum}")

    previous_sigterm = signal.signal(signal.SIGTERM, interrupted)
    passed_tests = set()
    try:
        for label, command in commands:
            # Every requested regression must have run before the downstream
            # compile. An empty or stale filter can never become acceptance.
            if label == "downstream":
                missing = set(args.required_test) - passed_tests
                if missing:
                    raise RuntimeError(f"required regressions did not pass: {sorted(missing)}")
                report["required_tests_passed"] = sorted(set(args.required_test) & passed_tests)
            log = evidence / (label + ".log")
            resources = evidence / (label + "-resources.txt")
            argv = ["/usr/bin/time", "-v", "-o", str(resources), *command]
            row = {"label": label, "argv": argv, "cwd": str(source), "started_unix": time.time(), "complete": False}
            report["commands"].append(row)
            save(manifest, report)
            print(f"START {label}", flush=True)
            process = None
            try:
                with log.open("w") as stream:
                    process = subprocess.Popen(argv, cwd=source, env=environment, stdout=stream,
                                               stderr=subprocess.STDOUT, start_new_session=True)
                    row["exit"] = process.wait()
                if row["exit"]:
                    raise RuntimeError(f"{label} exited {row['exit']}; see {log}")
                if label in test_labels:
                    contents = log.read_text()
                    row["test_summary"], passed = successful_tests(contents, label)
                    row["passed_tests"] = sorted(passed)
                    expected = report.get("expected_test_summaries", {}).get(label)
                    if expected is not None and row["test_summary"] != expected:
                        raise RuntimeError(f"{label} summary {row['test_summary']} does not match frozen {args.suite} suite {expected}")
                    passed_tests.update(passed)
                    if label == "metadata" and args.suite == "metadata":
                        for name in args.required_test:
                            if name not in passed:
                                raise RuntimeError(f"required metadata regression did not pass: {name}")
                row["complete"] = True
            finally:
                if process is not None and process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.wait(timeout=15)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.wait()
                    row["exit"] = process.returncode
                row["finished_unix"] = time.time()
                if log.exists():
                    row["log"] = identity(log)
                if resources.exists():
                    row["resources"] = identity(resources)
                save(manifest, report)
                print(f"END {label}: exit={row.get('exit')} complete={row['complete']}", flush=True)
            if label == "bootstrap":
                installation = output / "toolchain/installation.json"
                shutil.copy2(installation, evidence / "installation.json")
                report["toolchain"] = identity(evidence / "installation.json")
                environment["RUSTC"] = str(prefix / "bin/rustc")
                environment["PATH"] = str(prefix / "bin") + os.pathsep + environment["PATH"]
                environment["LD_LIBRARY_PATH"] = str(prefix / "lib") + (
                    os.pathsep + environment["LD_LIBRARY_PATH"] if environment.get("LD_LIBRARY_PATH") else "")
                report["environment"]["toolchain_path_prefix"] = str(prefix / "bin")
                report["environment"]["toolchain_library_prefix"] = str(prefix / "lib")
                report["environment"]["rustc"] = environment["RUSTC"]
        if args.suite in {"control-theme", "control-theme-regions"}:
            report["generated_winui"] = collect_generated_winui(output / "target", evidence)
            if not any(row["construction_programs"] for row in report["generated_winui"]):
                raise RuntimeError("fresh WinUI output contains no accepted ControlTheme construction programs")
            if args.suite == "control-theme-regions":
                report["generated_program_counts"] = {
                    key: sum(row[key] for row in report["generated_winui"])
                    for key in ("construction_programs", "region_programs")
                }
                if not any(row["region_programs"] for row in report["generated_winui"]):
                    raise RuntimeError("fresh WinUI output contains no accepted Style region programs")
        report["complete"] = True
    except BaseException as error:
        report["failure"] = str(error)
        raise
    finally:
        previously_complete = report["complete"]
        if args.suite in {"control-theme", "control-theme-regions"} and "generated_winui" not in report:
            try:
                report["generated_winui"] = collect_generated_winui(output / "target", evidence)
            except Exception as error:
                report["generated_evidence_error"] = str(error)
                report["complete"] = False
                report.setdefault("failure", "Could not preserve generated WinUI evidence")
        try:
            report["source_after"] = {
                "head": git(source, "rev-parse", "HEAD"), "tree": git(source, "rev-parse", "HEAD^{tree}"),
                "status": git(source, "status", "--porcelain", "--untracked-files=all"),
                "lockfile": identity(source / "Cargo.lock"),
            }
            after = report["source_after"]
            report["source_unchanged"] = (after["head"] == report["revision"] and after["tree"] == report["tree"]
                                          and not after["status"] and after["lockfile"] == report["lockfile_before"])
        except Exception as error:
            report["source_verification_error"] = str(error)
            report["source_unchanged"] = False
        if not report["source_unchanged"]:
            report["complete"] = False
            report.setdefault("failure", "Source identity changed or could not be verified after execution")
        report["finished_unix"] = time.time()
        save(manifest, report)
        signal.signal(signal.SIGTERM, previous_sigterm)
        if previously_complete and not report["source_unchanged"]:
            raise RuntimeError(report["failure"])
    print(manifest)


if __name__ == "__main__":
    main()
