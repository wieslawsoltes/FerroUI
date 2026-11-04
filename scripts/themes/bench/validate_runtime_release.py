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
from compare_registration import dependency_artifacts, executable_identity, native_linker_identity


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
HOST = "x86_64-unknown-linux-gnu"
EVIDENCE = "docs/benchmarks/compiled-theme-rust-idioms-linux-x86_64.json"
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
    for name in list(environment):
        if name in {"LLVM_PROFILE_FILE", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"} or name.startswith(("FERROUI_PGO_", "FERROUI_BENCH_EXPECT_")):
            environment.pop(name, None)
    environment["LLVM_PROFILE_FILE"] = "/dev/null"
    return environment


def temporary_directory(output: Path) -> Path:
    temporary = output / "runtime-tmp"
    temporary.mkdir(parents=True, exist_ok=True)
    if temporary.stat().st_uid != os.getuid() or temporary.resolve().parent != output.resolve():
        raise ValueError("Temporary directory must be owned and contained in the output directory")
    return temporary.resolve()


def link_inputs(args: argparse.Namespace, fixtures: dict, compiler: dict, linker: dict) -> dict:
    tools = {"rustc": identity(Path(compiler["invoked_as"])),
             "cc": identity(Path(linker["cc"]["invoked_as"])),
             "lld": identity(Path(linker["lld"]["invoked_as"]))}
    if "backend" in linker["lld"]:
        tools["lld_backend"] = identity(Path(linker["lld"]["backend"]["path"]))
    return {
        "sources": [identity(Path(item["path"])) for item in fixtures["files"]],
        "base": identity(args.base),
        "profile": identity(args.profile) if args.profile else None,
        "dependencies": {str(path.resolve()): dependency_artifacts(path.resolve()) for path in args.dependency},
        "tools": tools,
        "helpers": [identity(HERE / name) for name in
                    ("validate_runtime_release.py", "compact_unwind_link.py", "compact_unwind.py",
                     "attribute_codegen.py", "attribute_runtime.py", "compare_registration.py", "measure.py")],
    }


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
    if (output / "result.json").exists() or (output / args.command).exists():
        raise ValueError("Fixture compilation requires a fresh output directory")
    fixtures = materialize_fixtures(args.source.resolve(), output / "sources")
    entry = output / "sources" / (args.command + ".rs")
    executable = output / args.command
    environment = clean_environment()
    environment.update(
        CARGO_MANIFEST_DIR=str(args.source.resolve() / "src/FerroUI.Base"),
        TMPDIR=str(temporary_directory(output)), RAYON_NUM_THREADS="1", MALLOC_ARENA_MAX="1",
        MALLOC_CONF="narenas:1,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,background_thread:false,retain:false",
    )
    compiler = executable_identity(args.rustc, environment)
    compiler["verbose_version"] = subprocess.check_output([compiler["invoked_as"], "-vV"], env=environment, text=True)
    if f"host: {HOST}" not in compiler["verbose_version"]:
        raise ValueError(f"This link policy requires native {HOST}")
    linker = native_linker_identity(compiler["invoked_as"], HOST, environment)
    inputs = link_inputs(args, fixtures, compiler, linker)
    command = [compiler["invoked_as"], str(entry), "--edition=2021", "--crate-name", "runtime_" + args.command,
               "--target", HOST, "-Cpanic=unwind", "-Cstrip=symbols", "--extern",
               "ferroui_base=" + str(args.base.resolve()), "-o", str(executable),
               "-Clink-arg=-fuse-ld=lld"]
    for directory in args.dependency:
        command.append("-Ldependency=" + str(directory.resolve()))
    if args.command == "registry":
        command += ["--test", "-Copt-level=" + args.dev_opt_level, "-Clto=" + args.dev_lto,
                    "-Cdebug-assertions=yes", "-Ccodegen-units=16",
                    "-Clink-arg=-Wl,--threads=1", "-Clinker=" + linker["cc"]["invoked_as"]]
    else:
        if args.profile is None:
            raise ValueError("unwind requires the matching production graph and --profile")
        command += ["-Copt-level=z", "-Clto=fat", "-Ccodegen-units=1",
                    "-Cprofile-use=" + str(args.profile.resolve()), "-Cllvm-args=--addrsig",
                    "-Clink-arg=-Wl,--icf=safe", "-Clink-arg=-Wl,-z,pack-relative-relocs",
                    "-Clinker=" + str(args.source.resolve() / "scripts/themes/bench/compact_unwind_link.py")]
    report = {"kind": args.command + " fixture", "fixtures": fixtures, "base": identity(args.base),
              "profile": identity(args.profile) if args.profile else None, "command": command,
              "inputs": inputs, "compiler": compiler, "linker": linker, "complete": False,
              "development_options": {"lto": args.dev_lto, "opt_level": args.dev_opt_level}
              if args.command == "registry" else None,
              "controlled_environment": {name: environment[name] for name in
                                         ("PATH", "FERROUI_CC", "TMPDIR", "LLVM_PROFILE_FILE", "RAYON_NUM_THREADS", "MALLOC_ARENA_MAX", "MALLOC_CONF")},
              "scope": "Companion linked against the supplied real Base library; not a shipping size measurement."}
    report_path = output / "result.json"
    save(report_path, report)
    start = time.monotonic()
    with (output / "build.log").open("w") as log:
        built = subprocess.run(command, env=environment, stdout=log, stderr=log)
    report.update(compile_exit=built.returncode, compile_seconds=time.monotonic() - start)
    report["inputs_unchanged"] = inputs == link_inputs(args, fixtures, compiler, linker)
    save(report_path, report)
    if not report["inputs_unchanged"]:
        raise RuntimeError("Fixture source or production graph changed during compilation")
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
    if args.command == "registry" and report.get("counts") != {"passed": 47, "failed": 0, "ignored": 0}:
        raise RuntimeError("Registry fixture coverage changed; inspect the recorded test list")
    report["complete"] = True
    save(report_path, report)
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
              "shipping_budget_bytes": 10_000_000, "sections": elf_sections(args.executable), "cases": {},
              "complete": False, "structural_scope": "Font values after an insertion plus a scalar edit; the inserted resource itself is not separately queried."}
    report["remaining_to_budget_bytes"] = max(0, report["executable"]["bytes"] - 10_000_000)
    environment = clean_environment()
    environment["TMPDIR"] = str(temporary_directory(args.output.resolve().parent))
    # Without retain-archive, winui.rs ignores both the archive argument and
    # EXPECT variables. An impossible expected value must be rejected before
    # successful exits can count as editable-runtime correctness evidence.
    negative_command = [str(args.executable.resolve())]
    negative_environment = {**environment, "FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE": "999973"}
    negative = subprocess.run(negative_command, env=negative_environment, capture_output=True, text=True)
    report["expectation_negative_control"] = {"command": negative_command, "expected_normal_font": 999973,
                                               "exit": negative.returncode, "stdout": negative.stdout, "stderr": negative.stderr}
    save(args.output, report)
    if negative.returncode == 0 or "FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE" not in negative.stderr:
        raise RuntimeError("Executable did not reject the impossible expected resource value")
    for label, case in archives["cases"].items():
        normal, compact = case["font"]
        command = [str(args.executable.resolve())]
        if case["archive"]:
            command.append(case["archive"]["path"])
        case_environment = {**environment, "FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE": str(normal),
                            "FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE": str(compact)}
        result = subprocess.run(command, env=case_environment, capture_output=True, text=True)
        report["cases"][label] = {**case, "command": command, "exit": result.returncode,
                                  "stdout": result.stdout, "stderr": result.stderr}
        save(args.output, report)
        result.check_returncode()
    if len(report["cases"]) != 9:
        raise ValueError("All nine cases, including --installed, are required for the release check")
    if identity(args.executable) != report["executable"] or archive_manifest(args.archives) != archives:
        raise RuntimeError("Executable or archive inputs changed during validation")
    report["complete"] = True
    save(args.output, report)
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
              "candidate": identity(candidate), "cpu": args.cpu, "cases": {}, "complete": False,
              "protocol": "Alternating process order, warm launches, no expected-value or training work in timed children; shared host, no isolation claim.",
              "scope": "Process spawn to flushed typed Button ControlTheme readiness. No Button control or first painted frame is measured."}
    original_affinity = os.sched_getaffinity(0)
    original_environment = os.environ.copy()
    save(args.output, report)
    try:
        os.sched_setaffinity(0, {args.cpu})
        environment = clean_environment()
        environment["TMPDIR"] = str(temporary_directory(args.output.resolve().parent))
        os.environ.clear()
        os.environ.update(environment)
        for label, pairs in (("embedded", args.embedded_pairs), ("scalar-v3", 41),
                             ("struct-v3", 41), ("struct-v1", 21)):
            case = archives["cases"][label]
            extra = [case["archive"]["path"]] if case["archive"] else []
            commands = {"reference": [str(reference), *extra], "candidate": [str(candidate), *extra]}
            report["cases"][label] = {"expected_pairs": pairs, "commands": commands, "complete": False}

            def checkpoint(rows: dict) -> None:
                report["cases"][label].update(samples=rows, completed_pairs=len(rows["candidate"]))
                save(args.output, report)

            rows = measure_commands(commands, pairs, on_pair=checkpoint)
            diagnostics = {}
            for metric in ("create_ns", "button_lookup_ns", "create_and_button_ns", "process_ready_ns", "process_ns"):
                values = {side: [row[metric] for row in samples] for side, samples in rows.items()}
                diagnostics[metric] = {
                    "median_ns": {side: statistics.median(samples) for side, samples in values.items()},
                    "p95_ns": {side: sorted(samples)[math.ceil(.95 * len(samples)) - 1] for side, samples in values.items()},
                    "paired_ratio_95_interval": paired_interval(values["candidate"], values["reference"]),
                }
            report["cases"][label].update(pairs=pairs, samples=rows, diagnostics=diagnostics, complete=True)
            save(args.output, report)
            print(label, diagnostics["process_ready_ns"], flush=True)
    finally:
        os.sched_setaffinity(0, original_affinity)
        os.environ.clear()
        os.environ.update(original_environment)
    if identity(reference) != report["reference"] or identity(candidate) != report["candidate"] or archive_manifest(args.archives) != archives:
        raise RuntimeError("Executable or archive inputs changed during timing")
    report["complete"] = True
    save(args.output, report)
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
        if name == "registry":
            command.add_argument("--dev-lto", choices=("off", "thin"), default="off")
            command.add_argument("--dev-opt-level", choices=("0", "1"), default="1")
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
