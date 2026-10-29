#!/usr/bin/env python3
"""Build the same scan driver against both libraries; compare complete scanner output."""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
import time


def build(source: Path, driver: Path, target: Path, output: Path, optimize: bool = False) -> tuple[Path, dict]:
    env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target), CARGO_TERM_COLOR="never",
               RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="", CARGO_BUILD_RUSTC_WRAPPER="", CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER="")
    command = ["cargo", "build", "--locked", "--offline", "-p", "ferroui-build-scan", "--lib", "--message-format=json-render-diagnostics"]
    if optimize:
        command.extend(["--config", str(source / ".cargo/build-tools.toml")])
    started = time.perf_counter()
    result = subprocess.run(command, cwd=source, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    library_seconds = time.perf_counter() - started
    (output / "build.jsonl").write_text(result.stdout)
    (output / "build.stderr").write_text(result.stderr)
    (output / "build-result.json").write_text(json.dumps({"command": command, "seconds": library_seconds, "exit_code": result.returncode}, indent=2) + "\n")
    if result.returncode:
        raise RuntimeError(f"scanner library failed ({result.returncode}); see {output / 'build.stderr'}\n{result.stderr[-6000:]}")
    artifacts = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    libraries = [Path(filename) for artifact in artifacts
                 if artifact.get("reason") == "compiler-artifact" and artifact["target"]["name"] == "ferroui_build_scan"
                 for filename in artifact["filenames"] if filename.endswith(".rlib")]
    if len(libraries) != 1:
        raise RuntimeError(f"Expected one scanner library from Cargo, got {libraries}")
    dependency_directories = sorted({str(Path(filename).parent) for artifact in artifacts
                                     if artifact.get("reason") == "compiler-artifact"
                                     for filename in artifact["filenames"] if filename.endswith(".rlib")})
    search = [argument for directory in dependency_directories for argument in ("-L", f"dependency={directory}")]
    executable = output / ("scan-bench.exe" if os.name == "nt" else "scan-bench")
    started = time.perf_counter()
    subprocess.run([os.environ.get("RUSTC", "rustc"), str(driver), "--edition=2021", "-C", "debuginfo=0", "-C", "opt-level=0",
                    "--extern", f"ferroui_build_scan={libraries[0]}", *search,
                    "-o", str(executable)], cwd=source, env=env, check=True)
    return executable, {"library_build_seconds": library_seconds,
                        "driver_build_seconds": time.perf_counter() - started, "command": command}


def fingerprints(directory: Path) -> dict:
    return {path.relative_to(directory).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(directory.rglob("*")) if path.is_file() and path.name != "timings.json"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--repetitions", default=5, type=int)
    parser.add_argument("--minimum-speedup", default=0.0, type=float)
    parser.add_argument("--candidate-build-tools", action="store_true")
    args = parser.parse_args()
    if args.repetitions < 1:
        parser.error("repetitions must be positive")
    baseline, candidate, output = (path.resolve() for path in (args.baseline, args.candidate, args.output))
    rustc = os.environ.get("RUSTC", "rustc")
    for command in ([rustc, "-Vv"], ["cargo", "-V"]):
        if subprocess.check_output(command, cwd=baseline) != subprocess.check_output(command, cwd=candidate):
            parser.error("both checkouts must use exactly the same Cargo and rustc toolchains")
    output.mkdir(parents=True, exist_ok=False)
    driver = candidate / "scripts/build-perf/scan.rs"
    report = {"schema_version": 1, "scenario": "full-framework-source-scan-and-model-export",
              "toolchain": subprocess.check_output([rustc, "-Vv"], cwd=candidate, text=True),
              "source": str(candidate), "driver_sha256": hashlib.sha256(driver.read_bytes()).hexdigest(),
              "baseline_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=baseline, text=True).strip(),
              "candidate_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=candidate, text=True).strip(),
              "repetitions": args.repetitions, "samples": [], "builds": {},
              "candidate_optimized_build_tools": args.candidate_build_tools}
    try:
        with tempfile.TemporaryDirectory(prefix="ferroui-scan-benchmark-") as temporary:
            executables = {}
            for source in (baseline, candidate):
                subprocess.run(["cargo", "fetch", "--locked"], cwd=source, check=True)
            for label, source in (("baseline", baseline), ("candidate", candidate)):
                build_dir = output / label
                build_dir.mkdir()
                executables[label], report["builds"][label] = build(
                    source, driver, Path(temporary) / label, build_dir,
                    optimize=label == "candidate" and args.candidate_build_tools)
            reference = None
            for repeat in range(1, args.repetitions + 1):
                labels = ["baseline", "candidate"] if repeat % 2 else ["candidate", "baseline"]
                for label in labels:
                    destination = output / f"{label}-{repeat}"
                    start = time.perf_counter()
                    subprocess.run([str(executables[label]), str(candidate), str(destination), "1"], check=True)
                    elapsed = time.perf_counter() - start
                    hashes = fingerprints(destination)
                    if not hashes:
                        raise RuntimeError("scanner emitted no semantic evidence")
                    if reference is None:
                        reference = hashes
                    identical = hashes == reference
                    sample = {"label": label, "repeat": repeat, "wall_seconds": elapsed,
                              "semantic_sha256": hashes, "equal_to_baseline": identical,
                              "timings": json.loads((destination / "timings.json").read_text())}
                    report["samples"].append(sample)
                    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
                    if not identical:
                        changed = sorted(key for key in hashes.keys() | reference.keys() if hashes.get(key) != reference.get(key))
                        raise RuntimeError(f"scanner output changed: {changed}")
                    print(f"{label} scan {repeat}: {elapsed:.3f}s; all {len(hashes)} semantic outputs match", flush=True)
            medians = {label: statistics.median(sample["wall_seconds"] for sample in report["samples"] if sample["label"] == label)
                       for label in executables}
            report["summary"] = {"median_seconds": medians, "speedup": medians["baseline"] / medians["candidate"],
                                 "semantic_outputs_equal": True, "semantic_files": len(reference)}
            (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
            text = ("## Source scanner comparison\n\n"
                    "Same driver, source tree and compiler; alternating independent processes. "
                    f"Candidate uses explicit build-tool opt-level experiment: **{args.candidate_build_tools}**.\n\n"
                    "| Baseline median | Candidate median | Speedup |\n|---:|---:|---:|\n"
                    f"| {medians['baseline']:.3f}s | {medians['candidate']:.3f}s | {report['summary']['speedup']:.3f}× |\n\n"
                    f"All {len(reference)} model, diagnostic, source inventory, function and statistic files agree byte for byte. "
                    "This measures the build-time scanner, not whole-project Rust compilation. "
                    "Library and driver compilation costs are recorded separately in report.json.\n")
            (output / "summary.md").write_text(text)
            print(text)
            if os.environ.get("GITHUB_STEP_SUMMARY"):
                with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as handle:
                    handle.write(text)
            if report["summary"]["speedup"] < args.minimum_speedup:
                raise RuntimeError("scanner speedup below the explicitly requested acceptance threshold")
        return 0
    except (RuntimeError, subprocess.CalledProcessError) as error:
        report["error"] = str(error)
        (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
