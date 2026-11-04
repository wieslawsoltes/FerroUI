#!/usr/bin/env python3
"""Reproduce registry, unwind, editable-archive and paired startup checks.

The executable budget applies to the complete stripped shipping executable.
Fixture executables and compressed archives are recorded separately. Compile
fixtures sequentially; run startup only after every compiler has stopped.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import random
import re
import shutil
import statistics
import subprocess
import time

from measure import elf_sections, measure_commands


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
HOST = "x86_64-unknown-linux-gnu"
EVIDENCE = "docs/benchmarks/compiled-theme-rust-idioms-linux-x86_64.json"
TRAINING_VARIABLES = (
    "LLVM_PROFILE_FILE", "FERROUI_PGO_TRAIN",
    "FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE", "FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE",
)


def identity(path: Path) -> dict:
    path = path.resolve()
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest}


def save(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")
    temporary.replace(path)


def clean_environment() -> dict:
    environment = os.environ.copy()
    for name in TRAINING_VARIABLES:
        environment.pop(name, None)
    return environment


def retained_source(evidence: dict, location: str) -> str:
    expected = evidence["source_locations"][location]
    source = evidence["sources_by_sha256"][expected["sha256"]]["text"]
    actual = hashlib.sha256(source.encode()).hexdigest()
    if actual != expected["sha256"] or len(source.encode()) != expected["bytes"]:
        raise ValueError(f"Invalid retained source identity: {location}")
    return source


def materialize_fixtures(source: Path, output: Path) -> dict:
    evidence_path = source / EVIDENCE
    evidence = json.loads(evidence_path.read_text())
    output.mkdir(parents=True, exist_ok=True)
    registry = retained_source(evidence, "/tmp/ferro-run/final-registry-tests.rs")
    inputs = []
    for name in ("class_registration_tests.rs", "element_ref_registration_tests.rs", "virtual_dispatch_tests.rs"):
        original = source / "src/FerroUI.Base/tests" / name
        destination = output / name
        shutil.copyfile(original, destination)
        registry = registry.replace(f'/tmp/ferro496-base/src/FerroUI.Base/tests/{name}', name)
        inputs.append(identity(original))
    nullable = retained_source(evidence, "/tmp/recover-xaml/property-qa/registration.rs")
    (output / "nullable_registration_tests.rs").write_text(nullable)
    registry = registry.replace("/tmp/recover-xaml/property-qa/registration.rs", "nullable_registration_tests.rs")
    if "/tmp/" in registry:
        raise ValueError("Unresolved historical path in registry fixture")
    (output / "registry.rs").write_text(registry)
    (output / "unwind.rs").write_text(retained_source(evidence, "/tmp/ferro-run/unwind_compatibility.rs"))
    report = {
        "producer": identity(Path(__file__)), "evidence": identity(evidence_path),
        "source_inputs": inputs,
        "adaptation": "Current public registration test modules; retained nullable fixture; relative module paths only. No runtime API overlays.",
        "files": [identity(path) for path in sorted(output.glob("*.rs"))],
    }
    save(output / "fixtures.json", report)
    return report


def compile_fixture(args: argparse.Namespace) -> dict:
    output = args.output.resolve()
    fixtures = materialize_fixtures(args.source.resolve(), output / "sources")
    entry = output / "sources" / (args.command + ".rs")
    executable = output / args.command
    command = [args.rustc, str(entry), "--edition=2021", "--crate-name", "runtime_" + args.command,
               "--target", HOST, "-Cpanic=unwind", "-Cstrip=symbols", "--extern",
               "ferroui_base=" + str(args.base.resolve()), "-o", str(executable),
               "-Clink-arg=-fuse-ld=lld"]
    for directory in args.dependency:
        command.append("-Ldependency=" + str(directory.resolve()))
    if args.command == "registry":
        command += ["--test", "-Copt-level=1", "-Clto=off", "-Ccodegen-units=16",
                    "-Clink-arg=-Wl,--threads=1"]
    else:
        if args.profile is None:
            raise ValueError("unwind requires the matching production graph and --profile")
        command += ["-Copt-level=z", "-Clto=fat", "-Ccodegen-units=1",
                    "-Cprofile-use=" + str(args.profile.resolve()), "-Cllvm-args=--addrsig",
                    "-Clink-arg=-Wl,--icf=safe", "-Clink-arg=-Wl,-z,pack-relative-relocs",
                    "-Clinker=" + str(args.source.resolve() / "scripts/themes/bench/compact_unwind_link.py")]
    report = {"kind": args.command + " fixture", "fixtures": fixtures, "base": identity(args.base),
              "profile": identity(args.profile) if args.profile else None, "command": command,
              "scope": "Companion linked against the supplied real Base library; not a shipping size measurement."}
    report_path = output / "result.json"
    environment = clean_environment()
    environment["CARGO_MANIFEST_DIR"] = str(args.source.resolve() / "src/FerroUI.Base")
    start = time.monotonic()
    with (output / "build.log").open("w") as log:
        built = subprocess.run(command, env=environment, stdout=log, stderr=log)
    report.update(compile_exit=built.returncode, compile_seconds=time.monotonic() - start)
    save(report_path, report)
    built.check_returncode()
    report["executable"] = identity(executable)
    report["sections"] = elf_sections(executable)
    if args.command == "unwind" and ".cfi_reserve" not in report["sections"]:
        raise RuntimeError("Compact unwind was not applied to the unwind fixture")
    run_command = [str(executable), *(["--test-threads=1"] if args.command == "registry" else [])]
    result = subprocess.run(run_command, env=environment, capture_output=True, text=True)
    report.update(run_command=run_command, run_exit=result.returncode, stdout=result.stdout, stderr=result.stderr)
    match = re.search(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;", result.stdout)
    if match:
        report["counts"] = dict(zip(("passed", "failed", "ignored"), map(int, match.groups())))
    save(report_path, report)
    result.check_returncode()
    if args.command == "registry" and report.get("counts", {}).get("passed") != 47:
        raise RuntimeError("Registry fixture coverage changed; inspect the recorded test list")
    return report


def replace_once(path: Path, old: str, new: str) -> None:
    text = path.read_text()
    if text.count(old) != 1:
        raise ValueError(f"Expected one edit anchor in {path}")
    path.write_text(text.replace(old, new))


def make_archives(args: argparse.Namespace) -> dict:
    output = args.output.resolve()
    documents = output / "sources"
    source = args.source.resolve() / "src/FerroUI.Themes.WinUI"
    output.mkdir(parents=True, exist_ok=True)
    copied = []
    for original in sorted(source.rglob("*.xaml")):
        if not original.is_file() or original.is_symlink():
            continue
        destination = documents / original.relative_to(source)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(original, destination)
        copied.append(identity(original))
    if len(copied) != 169:
        raise ValueError(f"Theme document inventory changed: {len(copied)}")
    report = {"producer": identity(Path(__file__)), "packer": identity(args.packer),
              "theme_documents": copied, "cases": {"embedded": {"archive": None, "font": [14, 14]}}}
    if args.installed:
        report["cases"]["installed"] = {"archive": identity(args.installed), "font": [14, 14]}

    def pack(label: str, option: str, font: list[int]) -> None:
        archive = output / (label + ".fxp")
        command = [str(args.packer.resolve()), option, str(documents), str(archive)]
        result = subprocess.run(command, capture_output=True, text=True)
        result.check_returncode()
        report["cases"][label] = {"archive": identity(archive), "font": font,
                                  "command": command, "stdout": result.stdout, "stderr": result.stderr}
        save(output / "archives.json", report)

    token = '<x:Double x:Key="ControlContentThemeFontSize">14</x:Double>'
    compact = documents / "DensityStyles/Compact.xaml"
    replace_once(compact, token, token.replace(">14<", ">17<"))
    for version, option in ((1, "--pack"), (2, "--pack-solid"), (3, "--pack-paged")):
        pack(f"scalar-v{version}", option, [14, 17])
    replace_once(documents / "Resources/Generic.xaml", token, token.replace(">14<", ">19<"))
    pack("two-edits-v3", "--pack-paged", [19, 17])
    replace_once(compact, token.replace(">14<", ">17<"),
                 '<x:Double x:Key="AdditionalEditProbe">31</x:Double>\n  ' + token.replace(">14<", ">29<"))
    for version, option in ((1, "--pack"), (2, "--pack-solid"), (3, "--pack-paged")):
        pack(f"struct-v{version}", option, [19, 29])
    return report


def archive_manifest(path: Path) -> dict:
    report = json.loads(path.read_text())
    for case in report["cases"].values():
        expected = case["archive"]
        if expected and identity(Path(expected["path"])) != expected:
            raise ValueError(f"Archive changed since materialization: {expected['path']}")
    return report


def compatibility(args: argparse.Namespace) -> dict:
    archives = archive_manifest(args.archives)
    report = {"producer": identity(Path(__file__)), "executable": identity(args.executable),
              "shipping_budget_bytes": 10_000_000, "sections": elf_sections(args.executable), "cases": {}}
    report["remaining_to_budget_bytes"] = max(0, report["executable"]["bytes"] - 10_000_000)
    for label, case in archives["cases"].items():
        normal, compact = case["font"]
        command = [str(args.executable.resolve())]
        if case["archive"]:
            command.append(case["archive"]["path"])
        environment = {**clean_environment(), "FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE": str(normal),
                       "FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE": str(compact)}
        result = subprocess.run(command, env=environment, capture_output=True, text=True)
        report["cases"][label] = {**case, "command": command, "exit": result.returncode,
                                  "stdout": result.stdout, "stderr": result.stderr}
        save(args.output, report)
        result.check_returncode()
    if len(report["cases"]) != 9:
        raise ValueError("All nine cases, including --installed, are required for the release check")
    return report


def paired_interval(candidate: list[int], reference: list[int]) -> list[float]:
    ratios = [c / r for c, r in zip(candidate, reference, strict=True)]
    rng = random.Random(51517)
    resampled = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(7000))
    return [statistics.median(ratios), resampled[174], resampled[6824]]


def startup(args: argparse.Namespace) -> dict:
    archives = archive_manifest(args.archives)
    reference, candidate = args.reference.resolve(), args.candidate.resolve()
    if reference.stat().st_dev != candidate.stat().st_dev:
        raise ValueError("Place both measured executables on the same filesystem")
    report = {"producer": identity(Path(__file__)), "reference": identity(reference),
              "candidate": identity(candidate), "cpu": args.cpu, "cases": {},
              "protocol": "Alternating process order, warm launches, no expected-value or training work in timed children; shared host, no isolation claim.",
              "scope": "Process spawn to flushed first typed Button readiness. First painted frame is not measured."}
    original_affinity = os.sched_getaffinity(0)
    original_environment = {name: os.environ.get(name) for name in TRAINING_VARIABLES}
    try:
        os.sched_setaffinity(0, {args.cpu})
        for name in TRAINING_VARIABLES:
            os.environ.pop(name, None)
        for label, pairs in (("embedded", args.embedded_pairs), ("scalar-v3", 41),
                             ("struct-v3", 41), ("struct-v1", 21)):
            case = archives["cases"][label]
            extra = [case["archive"]["path"]] if case["archive"] else []
            commands = {"reference": [str(reference), *extra], "candidate": [str(candidate), *extra]}
            rows = measure_commands(commands, pairs)
            diagnostics = {}
            for metric in ("create_ns", "button_lookup_ns", "create_and_button_ns", "process_ready_ns", "process_ns"):
                values = {side: [row[metric] for row in samples] for side, samples in rows.items()}
                diagnostics[metric] = {
                    "median_ns": {side: statistics.median(samples) for side, samples in values.items()},
                    "p95_ns": {side: sorted(samples)[math.ceil(.95 * len(samples)) - 1] for side, samples in values.items()},
                    "paired_ratio_95_interval": paired_interval(values["candidate"], values["reference"]),
                }
            report["cases"][label] = {"pairs": pairs, "commands": commands, "samples": rows, "diagnostics": diagnostics}
            save(args.output, report)
            print(label, diagnostics["process_ready_ns"], flush=True)
    finally:
        os.sched_setaffinity(0, original_affinity)
        for name, value in original_environment.items():
            if value is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = value
    return report


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=ROOT)
    subcommands = parser.add_subparsers(dest="command", required=True)
    fixture = subcommands.add_parser("fixtures")
    fixture.add_argument("--output", type=Path, required=True)
    for name in ("registry", "unwind"):
        command = subcommands.add_parser(name)
        command.add_argument("--base", type=Path, required=True)
        command.add_argument("--dependency", type=Path, action="append", required=True)
        command.add_argument("--profile", type=Path)
        command.add_argument("--rustc", default="rustc")
        command.add_argument("--output", type=Path, required=True)
    archive = subcommands.add_parser("archives")
    archive.add_argument("--packer", type=Path, required=True)
    archive.add_argument("--installed", type=Path)
    archive.add_argument("--output", type=Path, required=True)
    compat = subcommands.add_parser("compat")
    compat.add_argument("--executable", type=Path, required=True)
    compat.add_argument("--archives", type=Path, required=True)
    compat.add_argument("--output", type=Path, required=True)
    timing = subcommands.add_parser("startup")
    timing.add_argument("--reference", type=Path, required=True)
    timing.add_argument("--candidate", type=Path, required=True)
    timing.add_argument("--archives", type=Path, required=True)
    timing.add_argument("--cpu", type=int, required=True)
    timing.add_argument("--embedded-pairs", type=int, default=101)
    timing.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "fixtures":
        report = materialize_fixtures(args.source.resolve(), args.output.resolve())
    elif args.command in ("registry", "unwind"):
        report = compile_fixture(args)
    elif args.command == "archives":
        report = make_archives(args)
    elif args.command == "compat":
        report = compatibility(args)
    else:
        if args.embedded_pairs < 1:
            parser.error("--embedded-pairs must be positive")
        report = startup(args)
    print(json.dumps({"command": args.command, "output": str(args.output.resolve()),
                      "cases": len(report.get("cases", {})), "counts": report.get("counts")}, indent=2))


if __name__ == "__main__":
    main()
