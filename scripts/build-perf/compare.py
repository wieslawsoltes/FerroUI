#!/usr/bin/env python3
"""Compare identical Cargo workloads on two checkouts, with genuinely empty target dirs.

Registry downloads are outside the timer. Each measured command uses --offline,
--locked, the same toolchain and environment, and a new target directory. The order
alternates between repetitions. Cargo JSON identifies units and build-script output
locations; the documented --timings HTML is preserved without parsing its internals.
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


def command_output(args: list[str], cwd: Path) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def normalized(text: str, source: Path, target: Path) -> str:
    # JSON/Rust strings may escape Windows separators. Longest prefix wins.
    replacements = []
    for path, replacement in ((source, "$SOURCE"), (target, "$TARGET")):
        for spelling in {str(path), path.as_posix(), str(path).replace("\\", "\\\\")}:
            replacements.append((spelling, replacement))
    for spelling, replacement in sorted(replacements, key=lambda pair: -len(pair[0])):
        text = text.replace(spelling, replacement)
    return text


def identity(source: Path) -> dict:
    return {
        "path": str(source),
        "commit": command_output(["git", "rev-parse", "HEAD"], source),
        "dirty": bool(command_output(["git", "status", "--porcelain", "--untracked-files=no"], source)),
        "lock_sha256": hashlib.sha256((source / "Cargo.lock").read_bytes()).hexdigest(),
    }


def generated_outputs(messages: list[dict], source: Path, target: Path) -> dict:
    """Hash generated Rust and metadata, retaining different feature/host variants.

    Do not rely on Cargo's private deps/build directory naming. out_dir is from its
    public JSON protocol. Group equal outputs as multisets to preserve multiplicity.
    """
    outputs: dict[str, list[str]] = {}
    # Cargo chooses different private directory hashes for different checkout
    # paths. Normalize only locations that Cargo itself identified as OUT_DIR,
    # not arbitrary hex strings in generated code or input data.
    output_locations = []
    for message in messages:
        if message.get("reason") == "build-script-executed":
            package = normalized(message["package_id"], source, target)
            directory = Path(message["out_dir"])
            for spelling in {str(directory), directory.as_posix(), str(directory).replace("\\", "\\\\")}:
                output_locations.append((spelling, "$OUT/" + package))
    for message in messages:
        if message.get("reason") != "build-script-executed":
            continue
        directory = Path(message["out_dir"])
        package = normalized(message["package_id"], source, target)
        for path in sorted(directory.rglob("*")):
            if not path.is_file() or path.suffix not in {".rs", ".xamlmeta"}:
                continue
            content = path.read_text(encoding="utf-8")
            for spelling, replacement in sorted(output_locations, key=lambda pair: -len(pair[0])):
                content = content.replace(spelling, replacement)
            content = normalized(content, source, target)
            key = package + "/" + path.relative_to(directory).as_posix()
            outputs.setdefault(key, []).append(hashlib.sha256(content.encode()).hexdigest())
    return {key: sorted(values) for key, values in sorted(outputs.items())}


def units(messages: list[dict], source: Path, target: Path) -> list[dict]:
    found = []
    for message in messages:
        if message.get("reason") == "compiler-artifact":
            found.append({
                "package": normalized(message["package_id"], source, target),
                "name": message["target"]["name"],
                "kind": message["target"]["kind"],
                "features": sorted(message["features"]),
                "profile": message["profile"],
                "fresh": message["fresh"],
            })
    return sorted(found, key=lambda item: json.dumps(item, sort_keys=True))


def measure(source: Path, output: Path, label: str, sample: int, args) -> dict:
    sample_dir = output / f"{label}-{sample}"
    sample_dir.mkdir()
    # tempfile owns precisely this directory; no deletion of a developer's target.
    with tempfile.TemporaryDirectory(prefix="ferroui-build-perf-", dir=args.target_root) as temporary:
        target = Path(temporary).resolve()
        env = dict(os.environ, CARGO_TARGET_DIR=str(target), CARGO_BUILD_BUILD_DIR=str(target), CARGO_TERM_COLOR="never",
                   RUSTC_WRAPPER="", RUSTC_WORKSPACE_WRAPPER="", CARGO_BUILD_RUSTC_WRAPPER="", CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER="")
        command = ["cargo", args.command, "--locked", "--offline", "--timings", "--message-format=json-render-diagnostics", "--profile", args.profile]
        if label == "candidate" and args.candidate_build_tools:
            command.extend(["--config", str(source / ".cargo/build-tools.toml")])
        if args.command == "test":
            command.extend(["--no-run", "--lib"])
        for package in args.package:
            command.extend(["-p", package])
        for feature in args.features:
            command.extend(["--features", feature])
        if args.no_default_features:
            command.append("--no-default-features")
        if args.jobs is not None:
            command.extend(["--jobs", str(args.jobs)])
        print(f"{label} sample {sample}: {' '.join(command)}", flush=True)
        start = time.perf_counter()
        with (sample_dir / "cargo.jsonl").open("w") as stdout, (sample_dir / "cargo.stderr").open("w") as stderr:
            result = subprocess.run(command, cwd=source, env=env, stdout=stdout, stderr=stderr)
        elapsed = time.perf_counter() - start
        messages = []
        for line in (sample_dir / "cargo.jsonl").read_text().splitlines():
            try:
                messages.append(json.loads(line))
            except json.JSONDecodeError:
                pass  # Retained in the raw log, never interpreted as success.
        record = {"label": label, "sample": sample, "seconds": elapsed,
                  "command": command, "exit_code": result.returncode,
                  "units": units(messages, source, target)}
        timing = target / "cargo-timings" / "cargo-timing.html"
        if timing.exists():
            shutil.copy2(timing, sample_dir / "cargo-timing.html")
        if result.returncode == 0:
            if not any(message.get("reason") == "build-finished" and message.get("success") for message in messages):
                raise RuntimeError("Cargo exited successfully without a successful build-finished message")
            record["generated"] = generated_outputs(messages, source, target)
        (sample_dir / "result.json").write_text(json.dumps(record, indent=2) + "\n")
        if result.returncode:
            print((sample_dir / "cargo.stderr").read_text()[-12000:], file=sys.stderr)
            raise RuntimeError(f"{label} failed with exit code {result.returncode}; see {sample_dir}")
        print(f"{label} sample {sample}: {elapsed:.3f}s, {len(record['units'])} compilation units", flush=True)
        return record


BUILD_TOOL_PACKAGES = {"ferroui-build-scan", "syn", "proc-macro2", "quote"}
BUILD_TOOL_OPT_LEVEL = "2"


def comparable_units(items: list[dict], allow_build_tool_optimization: bool) -> list[dict]:
    result = json.loads(json.dumps(items))
    for item in result:
        package = item.get("package", "").rsplit("#", 1)[-1].split("@", 1)[0]
        if allow_build_tool_optimization and package in BUILD_TOOL_PACKAGES:
            profile = item.get("profile", {})
            # Permit exactly the disclosed optimization experiment. Features,
            # debug assertions, overflow checks and every other field still match.
            if str(profile.get("opt_level")) in {"0", BUILD_TOOL_OPT_LEVEL}:
                profile["opt_level"] = "explicit-build-tool-experiment"
    return sorted(result, key=lambda item: json.dumps(item, sort_keys=True))


def summarize(records: list[dict], allow_build_tool_optimization: bool = False) -> dict:
    baseline = [row for row in records if row["label"] == "baseline"]
    candidate = [row for row in records if row["label"] == "candidate"]
    base_seconds = [row["seconds"] for row in baseline]
    candidate_seconds = [row["seconds"] for row in candidate]
    reference_units = baseline[0]["units"]
    reference_generated = baseline[0]["generated"]
    return {
        "baseline_seconds": base_seconds,
        "candidate_seconds": candidate_seconds,
        "baseline_median_seconds": statistics.median(base_seconds),
        "candidate_median_seconds": statistics.median(candidate_seconds),
        "speedup": statistics.median(base_seconds) / statistics.median(candidate_seconds),
        "same_units_features_profiles": all(row["units"] == reference_units for row in records),
        "same_units_features_and_allowed_profiles": all(
            comparable_units(row["units"], allow_build_tool_optimization) == comparable_units(reference_units, allow_build_tool_optimization)
            for row in records),
        "build_tool_optimization_experiment": allow_build_tool_optimization,
        "same_generated_rust_and_metadata": all(row["generated"] == reference_generated for row in records),
        "generated_files_per_build": sum(map(len, reference_generated.values())),
        "all_units_cold": all(not unit["fresh"] for row in records for unit in row["units"]),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--target-root", type=Path, help="Parent of temporary target directories; never a target to delete")
    parser.add_argument("--package", action="append", required=True)
    parser.add_argument("--features", action="append", default=[])
    parser.add_argument("--no-default-features", action="store_true")
    parser.add_argument("--profile", default="dev")
    parser.add_argument("--command", choices=["build", "check", "test"], default="build")
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--jobs", type=int)
    parser.add_argument("--candidate-build-tools", action="store_true",
                        help="Explicitly benchmark candidate .cargo/build-tools.toml; only four build-tool opt levels may differ")
    parser.add_argument("--minimum-speedup", type=float, default=0,
                        help="Explicit acceptance gate; 0 records evidence without claiming a target")
    args = parser.parse_args()
    if args.repetitions < 1 or (args.jobs is not None and args.jobs < 1):
        parser.error("repetitions and jobs must be positive")
    if not math.isfinite(args.minimum_speedup) or args.minimum_speedup < 0:
        parser.error("minimum-speedup must be finite and nonnegative")
    args.baseline = args.baseline.resolve()
    args.candidate = args.candidate.resolve()
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=False)
    environment = {key: value for key, value in os.environ.items()
                   if key.startswith("CARGO_PROFILE_") or key in {"CARGO_INCREMENTAL", "CARGO_BUILD_JOBS", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"}}
    if any(os.environ.get(key) for key in ("RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER")):
        parser.error("cold compiler comparison requires no compiler cache wrapper; benchmark cache reuse separately")
    rustc = os.environ.get("RUSTC", "rustc")
    for command in ([rustc, "-Vv"], ["cargo", "-V"]):
        if command_output(command, args.baseline) != command_output(command, args.candidate):
            parser.error("both checkouts must use exactly the same Cargo and rustc toolchains")
    report = {"schema_version": 1, "scenario": "empty-target-dependencies-downloaded",
              "toolchain": command_output([rustc, "-Vv"], args.candidate),
              "cargo": command_output(["cargo", "-V"], args.candidate),
              "platform": platform.platform(), "cpu_count": os.cpu_count(),
              "environment": environment, "sources": {"baseline": identity(args.baseline), "candidate": identity(args.candidate)},
              "records": []}
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    try:
        for source in (args.baseline, args.candidate):
            subprocess.run(["cargo", "fetch", "--locked"], cwd=source, check=True)
        for sample in range(1, args.repetitions + 1):
            order = [("baseline", args.baseline), ("candidate", args.candidate)]
            if sample % 2 == 0:
                order.reverse()
            for label, source in order:
                report["records"].append(measure(source, args.output, label, sample, args))
                (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        report["summary"] = summary = summarize(report["records"], args.candidate_build_tools)
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        text = ("## Cargo build comparison\n\n"
                f"Scenario: `{report['scenario']}`. Same workload, toolchain and feature set. "
                f"Explicit build-tool opt-level experiment: **{args.candidate_build_tools}**.\n\n"
                "| Baseline median | Candidate median | Speedup |\n|---:|---:|---:|\n"
                f"| {summary['baseline_median_seconds']:.3f}s | {summary['candidate_median_seconds']:.3f}s | {summary['speedup']:.3f}× |\n\n"
                f"Units/features/profiles equal: **{summary['same_units_features_profiles']}**. "
                f"Equivalent apart from explicitly permitted build-tool opt levels: **{summary['same_units_features_and_allowed_profiles']}**. "
                f"Generated Rust/metadata equal: **{summary['same_generated_rust_and_metadata']}** "
                f"({summary['generated_files_per_build']} files/build).\n\n"
                "Individual samples, source SHAs, Cargo logs and timing HTML are in the artifact. "
                "This is a compile measurement; tests and cache replay are separate scenarios.\n")
        (args.output / "summary.md").write_text(text)
        print(text)
        if os.environ.get("GITHUB_STEP_SUMMARY"):
            with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as handle:
                handle.write(text)
        valid = (summary["same_units_features_and_allowed_profiles"] and summary["same_generated_rust_and_metadata"]
                 and summary["all_units_cold"])
        if not valid:
            raise RuntimeError("comparison changed compilation units, features, profiles or generated outputs")
        if summary["speedup"] < args.minimum_speedup:
            raise RuntimeError(f"measured speedup {summary['speedup']:.3f}x is below required {args.minimum_speedup:.3f}x")
        return 0
    except (RuntimeError, subprocess.CalledProcessError) as error:
        report["error"] = str(error)
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
