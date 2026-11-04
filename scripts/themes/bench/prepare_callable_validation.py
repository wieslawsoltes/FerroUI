#!/usr/bin/env python3
"""Materialize, build, check, or compare public Base callable/animation fixtures.

No command trains PGO or rebuilds Base. Each build requires an explicit Base
archive and its matching target/host dependency directories. Production builds
also require the exact profile used for that library graph. Run build, check,
and measure separately so compilers cannot overlap a timing cohort.

The animation wrapper is reconstructed from current repository tests: all 30
iteration cases and the first 14 style/preview cases. It is not claimed to be
byte-identical to the lost historical wrapper. The receiver wrapper preserves
the eight repository case bodies, with one explicitly recorded reference-only
strong-count adaptation (borrowed=2, cloned=3). No private implementation or
source-overlay substitution is compiled into any fixture.
"""

import argparse
import errno
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import time

from compare_registration import (
    dependency_artifacts, digest, executable_identity, native_linker_identity, paired_ratio,
)
from compact_unwind_link import RESERVE, inspect


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TARGET = "x86_64-unknown-linux-gnu"
TESTS = re.compile(r"(?m)^#\[test\]\nfn ([a-zA-Z0-9_]+)\(")
APPENDIX = "// --- The animation tests of the upstream style tests (`Styling/StyleTests.cs`) ---"
EXPECTED_EXCLUDED = [
    "animations_should_be_activated",
    "animations_with_trigger_should_be_activated_and_deactivated",
    "animations_with_activator_trigger_should_be_activated_and_deactivated",
]
SOURCES = {
    "receivers": "receiver_probe.rs", "getters": "actual_getters.rs",
    "animation": "animation_qa.rs", "keyframes": "keyframe_probe.rs",
    "animation-allocations": "animation_attachment_allocations.rs",
}
LANES = {
    "receivers": ["rc-get", "rc-set", "rc-method", "ref-get", "ref-set", "converted-get", "wrong-type", "wrong-arity"],
    "getters": ["success", "wrong-type", "wrong-arity"],
    "keyframes": ["normal", "reverse", "alternate", "attach"],
}
KEYFRAME_RESULTS = {
    "normal": (24_000, 4_788_000.0), "reverse": (24_000, 4_812_000.0),
    "alternate": (24_000, 4_599_955.0), "attach": (1_000, 199_500.0),
}


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def identity(path):
    return {"path": str(path.resolve()), "sha256": digest(path), "bytes": path.stat().st_size}


def storage(path):
    resolved = path.resolve(strict=True)
    stat = resolved.stat()
    return {"resolved_path": str(resolved), "st_dev": stat.st_dev, "uid": stat.st_uid}


def verify_files(directory, files):
    for name, expected in files.items():
        path = directory / name
        if not path.is_file() or digest(path) != expected:
            raise RuntimeError(f"immutable fixture input changed: {path}")


def materialize(args):
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=True)
    if any(directory.iterdir()):
        raise RuntimeError("materialization requires a fresh output directory")
    base = args.source_root.resolve() / "src/FerroUI.Base"
    original = base / "metadata/receiver_tests.rs"
    receiver = original.read_text()
    receiver_names = TESTS.findall(receiver)
    if len(receiver_names) != 8:
        raise RuntimeError("receiver test selection changed; review before updating the cohort")
    expectation = "assert_eq!(from_markup_value::<usize>(&get(&arguments).unwrap()), Some(2));"
    if receiver.count(expectation) != 1:
        raise RuntimeError("exact shared-receiver expectation is no longer uniquely identifiable")
    (directory / "receiver_tests-original.rs").write_text(receiver)
    if args.receiver_policy == "cloned":
        receiver = receiver.replace(expectation, expectation.replace("Some(2)", "Some(3)"))
    receiver = receiver.replace("#[test]\n", "")
    receiver += '\ninclude!("shared_receiver_workloads.rs");\n'
    (directory / "receiver_cases.rs").write_text(receiver)
    (directory / "receiver_probe.rs").write_text(
        'pub use ferroui_base::*;\n#[path = "receiver_cases.rs"] mod receiver_cases;\n'
        'fn main() { let mode = std::env::args().nth(1).unwrap_or_else(|| "check".into()); '
        'receiver_cases::run_receiver_probe(&mode); }\n'
    )
    animation = base / "animation/tests"
    support = (animation / "mod.rs").read_text()
    original_support = support
    support, removed = re.subn(r"(?m)^mod [a-z_]+;\n", "", support)
    if removed != 9:
        raise RuntimeError("animation support module selection changed")
    support += ('\n#[path = "animation_iteration_tests.rs"] mod animation_iteration_tests;\n'
                '#[path = "style_animation_tests.rs"] mod style_animation_tests;\n')
    iteration = (animation / "animation_iteration_tests.rs").read_text()
    style_original = (animation / "style_animation_tests.rs").read_text()
    if style_original.count(APPENDIX) != 1:
        raise RuntimeError("animation appendix boundary changed")
    style, excluded = style_original.split(APPENDIX)
    names = {
        "animation_iteration_tests": TESTS.findall(iteration),
        "style_animation_tests": TESTS.findall(style),
    }
    if [len(v) for v in names.values()] != [30, 14] or TESTS.findall(excluded) != EXPECTED_EXCLUDED:
        raise RuntimeError("animation named cohort changed; review the new selection")
    for name, contents in {
        "animation_support-original.rs": original_support,
        "style_animation_tests-original.rs": style_original,
        "animation_support.rs": support,
        "animation_iteration_tests.rs": iteration,
        "style_animation_tests.rs": style,
        "animation_qa.rs": 'pub use ferroui_base::*;\n#[path = "animation_support.rs"] mod animation_tests;\n',
    }.items():
        (directory / name).write_text(contents)
    inputs = [original, animation / "mod.rs", animation / "animation_iteration_tests.rs",
              animation / "style_animation_tests.rs"]
    for name in ("shared_receiver_workloads.rs", "actual_getters.rs", "keyframe_probe.rs",
                 "animation_attachment_allocations.rs"):
        source = HERE / name
        shutil.copy2(source, directory / name)
        inputs.append(source)
    report = {
        "schema": 1, "directory": str(directory), "producer": identity(Path(__file__)),
        "source_inputs": [identity(path) for path in inputs],
        "files": {path.name: digest(path) for path in sorted(directory.iterdir())},
        "receiver_policy": args.receiver_policy,
        "receiver_adaptation": {
            "removed_test_attributes": 8,
            "exact_in_call_strong_count": 2 if args.receiver_policy == "borrowed" else 3,
            "scope": "One optimization-specific count differs; all other test bodies are unchanged.",
            "cases": receiver_names,
        },
        "animation_cohort": {
            "scope": "Reconstructed wrapper; current named public cases, not historical wrapper bytes or results.",
            "included": names,
            "excluded": [
                {"name": name, "reason": "Private styling::test_support::Class1 and styling::testing::try_attach."}
                for name in EXPECTED_EXCLUDED[:2]
            ] + [{"name": EXPECTED_EXCLUDED[2],
                  "reason": "Shares the excluded historical appendix/helpers; its body has no direct private call."}],
        },
        "allocation_scope": "8 uncounted warmups, 16 counted attachments; includes Observable::return_(true) "
                            "and apply. Setup, stepping, assertions, disposal, lease drop and output are uncounted.",
    }
    save(directory / "fixture.json", report)
    print(directory / "fixture.json")


def read_fixture(path):
    fixture = json.loads(path.read_text())
    directory = Path(fixture["directory"])
    verify_files(directory, fixture["files"])
    return fixture, directory


def build_environment(directory, source_root):
    temporary = directory / "compiler-tmp"
    temporary.mkdir(parents=True, exist_ok=True)
    if temporary.stat().st_uid != os.getuid() or temporary.resolve().parent != directory.resolve():
        raise RuntimeError("compiler temporary directory must be owned and contained in the build output")
    env = os.environ.copy()
    for key in ("FERROUI_PGO_TRAIN", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"):
        env.pop(key, None)
    overlay = {
        "TMPDIR": str(temporary), "RAYON_NUM_THREADS": "1", "MALLOC_ARENA_MAX": "1",
        "LLVM_PROFILE_FILE": str(directory / "qa-only-%m-%p.profraw"),
        "CARGO_MANIFEST_DIR": str(source_root.resolve() / "src/FerroUI.Base"),
        "CARGO_PKG_NAME": "ferroui-callable-validation", "CARGO_PKG_VERSION": "0.0.0",
        "CARGO_PKG_VERSION_MAJOR": "0", "CARGO_PKG_VERSION_MINOR": "0", "CARGO_PKG_VERSION_PATCH": "0",
    }
    env.update(overlay)
    return env, overlay, storage(temporary)


def build(args):
    fixture, directory = read_fixture(args.fixture)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    manifest = output / "build.json"
    binary = output / args.kind
    if manifest.exists() or binary.exists():
        raise RuntimeError("build requires a fresh output directory")
    if args.mode == "production" and args.profile is None:
        raise RuntimeError("production comparison requires the graph's matching PGO profile")
    if args.mode == "dev" and args.profile is not None:
        raise RuntimeError("development checks do not use PGO")
    if args.mode == "production" and (args.dev_lto != "off" or args.dev_opt_level != "1"):
        raise RuntimeError("development LTO/optimization switches cannot alter a production build")
    env, overlay, temporary_storage = build_environment(output, args.source_root)
    toolchain = executable_identity(args.rustc, env)
    verbose = subprocess.check_output([toolchain["invoked_as"], "-vV"], env=env, text=True)
    if f"host: {TARGET}" not in verbose:
        raise RuntimeError(f"the fixtures/link policy require native {TARGET}")
    linker = native_linker_identity(toolchain["invoked_as"], TARGET, env)
    dependencies = []
    for path in dict.fromkeys(p.resolve() for p in args.dependency):
        files = dependency_artifacts(path)
        inventory = hashlib.sha256(json.dumps(files, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
        dependencies.append({"path": str(path), "inventory_sha256": inventory, "files": files})
    source = directory / SOURCES[args.kind]
    command = [toolchain["invoked_as"], str(source), "--edition=2021", "--crate-name",
               "callable_" + args.kind.replace("-", "_"), "--target", TARGET,
               "-Cpanic=unwind", "-Cdebuginfo=0", "-Clink-arg=-fuse-ld=lld"]
    if args.kind == "animation":
        command += ["--test"]
    if args.mode == "dev":
        command += [f"-Copt-level={args.dev_opt_level}", "-Cdebug-assertions=yes",
                    f"-Clto={args.dev_lto}", "-Ccodegen-units=16",
                    "-Clink-arg=-Wl,--threads=1"]
    else:
        command += ["-Copt-level=z", "-Cdebug-assertions=no", "-Clto=fat", "-Ccodegen-units=1",
                    "-Cstrip=symbols", f"-Cprofile-use={args.profile.resolve()}", "-Cllvm-args=--addrsig",
                    "-Clink-arg=-Wl,--icf=safe", "-Clink-arg=-Wl,-z,pack-relative-relocs",
                    f"-Clinker={HERE / 'compact_unwind_link.py'}"]
    command += [f"-Ldependency={item['path']}" for item in dependencies]
    command += ["--extern", f"ferroui_base={args.base.resolve()}", "-o", str(binary)]
    report = {
        "schema": 1, "kind": args.kind, "mode": args.mode,
        "development_options": {"lto": args.dev_lto, "opt_level": args.dev_opt_level,
                                "debug_assertions": True, "codegen_units": 16}
                               if args.mode == "dev" else None,
        "fixture": identity(args.fixture), "fixture_contents": fixture,
        "base": identity(args.base), "dependencies": dependencies,
        "profile": identity(args.profile) if args.profile else None,
        "rustc": {**toolchain, "verbose_version": verbose}, "native_linker": linker,
        "producer": identity(Path(__file__)),
        "helpers": {name: digest(HERE / name) for name in
                    ("compare_registration.py", "compact_unwind_link.py", "compact_unwind.py", "attribute_codegen.py")},
        "compiler": {"command": command, "environment": overlay,
                     "path": env.get("PATH"), "ferroui_cc": env.get("FERROUI_CC"),
                     "temporary_storage": temporary_storage},
    }
    save(manifest, report)
    started = time.monotonic()
    with (output / "build.log").open("w") as log:
        result = subprocess.run(command, env=env, stdout=log, stderr=log)
    report["compiler"].update(exit_code=result.returncode, seconds=time.monotonic() - started,
                              log=identity(output / "build.log"))
    if result.returncode == 0:
        report["executable"] = {**identity(binary), "storage": storage(binary)}
        report["compact_unwind_applied"] = RESERVE in inspect(binary)[2]
    save(manifest, report)
    try:
        verify_files(directory, fixture["files"])
        if identity(args.fixture) != report["fixture"]:
            raise RuntimeError("fixture manifest changed during the build")
        if identity(args.base) != report["base"]:
            raise RuntimeError("Base archive changed during the build")
        if args.profile is not None and identity(args.profile) != report["profile"]:
            raise RuntimeError("PGO profile changed during the build")
        if identity(Path(__file__)) != report["producer"]:
            raise RuntimeError("validation producer changed during the build")
        for name, expected in report["helpers"].items():
            if digest(HERE / name) != expected:
                raise RuntimeError(f"validation helper changed during the build: {name}")
        for dependency in dependencies:
            if dependency_artifacts(Path(dependency["path"])) != dependency["files"]:
                raise RuntimeError(f"dependency graph changed during the build: {dependency['path']}")
        compiler_path = Path(toolchain["invoked_as"]).resolve(strict=True)
        if str(compiler_path) != toolchain["path"] or digest(compiler_path) != toolchain["sha256"]:
            raise RuntimeError("selected Rust compiler changed during the build")
        for name in ("cc", "lld"):
            selected = linker[name]
            selected_path = Path(selected["invoked_as"]).resolve(strict=True)
            if str(selected_path) != selected["path"] or digest(selected_path) != selected["sha256"]:
                raise RuntimeError(f"selected native linker component changed during the build: {name}")
        if "backend" in linker["lld"]:
            backend = linker["lld"]["backend"]
            if digest(Path(backend["path"])) != backend["sha256"]:
                raise RuntimeError("selected native linker backend changed during the build")
    except (OSError, RuntimeError) as error:
        report["input_verification_error"] = str(error)
        save(manifest, report)
        raise
    report["inputs_unchanged_after_compiler"] = True
    save(manifest, report)
    if result.returncode:
        raise RuntimeError(f"{args.kind} build failed; see {output / 'build.log'}")
    print(manifest)


def read_build(path):
    report = json.loads(path.read_text())
    if report["compiler"].get("exit_code") != 0:
        raise RuntimeError(f"{path}: no successful build")
    if report.get("inputs_unchanged_after_compiler") is not True:
        raise RuntimeError(f"{path}: build inputs were not verified after the compiler")
    executable = Path(report["executable"]["path"])
    if identity(executable) != {k: report["executable"][k] for k in ("path", "sha256", "bytes")}:
        raise RuntimeError(f"{path}: executable changed")
    fixture = report["fixture_contents"]
    verify_files(Path(fixture["directory"]), fixture["files"])
    return report, executable


def runtime_environment(directory):
    temporary = directory / "runtime-tmp"
    temporary.mkdir(parents=True, exist_ok=True)
    if temporary.stat().st_uid != os.getuid() or temporary.resolve().parent != directory.resolve():
        raise RuntimeError("runtime temporary directory must be owned and contained in the output")
    env = os.environ.copy()
    removed = []
    for key in list(env):
        if key in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "LLVM_PROFILE_FILE"} or key.startswith(("FERROUI_PGO_", "FERROUI_BENCH_EXPECT_")):
            removed.append(key)
            env.pop(key)
    overlay = {"LLVM_PROFILE_FILE": "/dev/null", "TMPDIR": str(temporary),
               "RAYON_NUM_THREADS": "1", "MALLOC_ARENA_MAX": "1"}
    env.update(overlay)
    return env, {"removed_names": sorted(removed), **overlay, "temporary_storage": storage(temporary)}


def invoke(executable, arguments, env):
    completed = subprocess.run([str(executable), *arguments], env=env, capture_output=True, text=True)
    return {"command": [str(executable), *arguments], "exit_code": completed.returncode,
            "stdout": completed.stdout, "stderr": completed.stderr}


def parse_timing(kind, lane, output):
    fields = output.strip().split()
    if kind == "keyframes":
        if len(fields) != 3:
            raise RuntimeError(f"invalid keyframe output: {output!r}")
        count, checksum, wall = int(fields[0]), float(fields[1]), int(fields[2])
        if (count, checksum) != KEYFRAME_RESULTS[lane] or wall <= 0:
            raise RuntimeError(f"invalid keyframe output: {output!r}")
        return {"operations": count, "checksum": checksum, "wall_ns": wall, "metric_ns_per_operation": wall / count}
    if len(fields) != 4:
        raise RuntimeError(f"invalid callable output: {output!r}")
    mode, count, cpu, wall = fields[0], int(fields[1]), int(fields[2]), int(fields[3])
    if mode != lane or min(count, cpu, wall) <= 0:
        raise RuntimeError(f"invalid callable output: {output!r}")
    return {"operations": count, "thread_cpu_ns": cpu, "wall_ns": wall, "metric_ns_per_operation": cpu / count}


def check(args):
    report, executable = read_build(args.build)
    kind = report["kind"]
    rows = []
    args.out.parent.mkdir(parents=True, exist_ok=True)
    if args.out.exists():
        raise RuntimeError("check output already exists; retain prior results and select a fresh path")
    env, environment = runtime_environment(args.out.resolve().parent)
    checked = {"build": identity(args.build), "kind": kind, "results": rows,
               "environment": environment, "complete": False,
               "scope": "Current external Base library; no historical pass is inferred."}
    save(args.out, checked)

    def run(arguments):
        row = invoke(executable, arguments, env)
        rows.append(row)
        save(args.out, checked)
        if row["exit_code"]:
            raise RuntimeError(f"{kind} failed; see {args.out}")
        return row

    if kind == "animation":
        listed = run(["--list", "--format=terse"])
        names = [line.removesuffix(": test") for line in listed["stdout"].splitlines() if line.endswith(": test")]
        expected = sorted("animation_tests::" + module + "::" + name
                          for module, names in report["fixture_contents"]["animation_cohort"]["included"].items()
                          for name in names)
        if sorted(names) != expected:
            raise RuntimeError("compiled animation test names differ from the frozen cohort")
        ran = run(["--test-threads=1"])
        if not re.search(r"test result: ok\. 44 passed; 0 failed; 0 ignored;", ran["stdout"]):
            raise RuntimeError("animation result is not the required 44 cases")
    elif kind in ("receivers", "getters"):
        ran = run(["check"])
        if kind == "receivers" and ran["stdout"].strip() != "receiver_checks 8":
            raise RuntimeError("receiver cases did not complete")
        if kind == "getters" and len(ran["stdout"].splitlines()) != 88:
            raise RuntimeError("getter case count differs")
    elif kind == "keyframes":
        for lane in LANES[kind]:
            ran = run([lane])
            ran["parsed"] = parse_timing(kind, lane, ran["stdout"])
    else:
        ran = run([])
        samples = []
        for index, line in enumerate(ran["stdout"].splitlines()):
            label, sample, requests, requested_bytes = line.split()
            if label != "attachment" or int(sample) != index:
                raise RuntimeError("unexpected allocation sample ordering")
            samples.append({"allocation_requests": int(requests), "requested_bytes": int(requested_bytes)})
        if len(samples) != 16 or any(sample != samples[0] for sample in samples):
            raise RuntimeError("allocation cohort must contain 16 stable warmed samples")
        ran["samples"] = samples
    read_build(args.build)
    checked["complete"] = True
    save(args.out, checked)
    print(args.out)


def stage_binary(source, destination):
    try:
        os.link(source, destination)
        policy = "hardlink"
    except OSError as error:
        if error.errno != errno.EXDEV:
            raise
        shutil.copy2(source, destination)
        policy = "copy2 after EXDEV"
    if source.stat().st_size != destination.stat().st_size or digest(source) != digest(destination):
        raise RuntimeError("staged executable identity mismatch")
    return {"policy": policy, "source": identity(source), "staged": identity(destination),
            "source_storage": storage(source), "staged_storage": storage(destination)}


def measure(args):
    if args.cpu not in os.sched_getaffinity(0) or args.pairs < 3:
        raise RuntimeError("choose an allowed CPU and at least three pairs")
    output = args.out.resolve()
    if output.exists():
        raise RuntimeError("measurement output already exists; retain the previous cohort")
    work = output.parent / (output.stem + "-executables")
    work.mkdir(parents=True, exist_ok=False)
    builds, executables, staged = {}, {}, {}
    for arm in ("reference", "candidate"):
        builds[arm], original = read_build(getattr(args, arm))
        if builds[arm]["mode"] != "production":
            raise RuntimeError("performance comparisons require production builds")
        executables[arm] = work / arm
        staged[arm] = stage_binary(original, executables[arm])
    kind = builds["reference"]["kind"]
    if kind not in LANES or builds["candidate"]["kind"] != kind:
        raise RuntimeError("require the same timed fixture kind")
    if storage(executables["reference"])["st_dev"] != storage(executables["candidate"])["st_dev"]:
        raise RuntimeError("comparison executables must use the same filesystem")
    # The receiver module has an intentional single expectation difference.
    # Every benchmark body and all other source inputs must still match.
    reference_files = builds["reference"]["fixture_contents"]["files"]
    candidate_files = builds["candidate"]["fixture_contents"]["files"]
    for name in reference_files:
        if name != "receiver_cases.rs" and candidate_files.get(name) != reference_files[name]:
            raise RuntimeError(f"fixture inputs differ: {name}")
    reference_receiver = (Path(builds["reference"]["fixture_contents"]["directory"]) / "receiver_cases.rs").read_text()
    candidate_receiver = (Path(builds["candidate"]["fixture_contents"]["directory"]) / "receiver_cases.rs").read_text()
    expected = "assert_eq!(from_markup_value::<usize>(&get(&arguments).unwrap()), Some(2));"
    if builds["reference"]["fixture_contents"]["receiver_policy"] == "cloned":
        reference_receiver = reference_receiver.replace(expected.replace("Some(2)", "Some(3)"), expected, 1)
    if builds["candidate"]["fixture_contents"]["receiver_policy"] != "borrowed" or reference_receiver != candidate_receiver:
        raise RuntimeError("receiver source differs beyond the explicit count expectation")
    for key in ("producer", "helpers"):
        reference = builds["reference"][key]
        candidate = builds["candidate"][key]
        if (reference["sha256"] if key == "producer" else reference) != (candidate["sha256"] if key == "producer" else candidate):
            raise RuntimeError(f"build producers differ: {key}")
    if builds["reference"]["rustc"]["sha256"] != builds["candidate"]["rustc"]["sha256"]:
        raise RuntimeError("compiler identities differ")
    os.sched_setaffinity(0, {args.cpu})
    env, environment = runtime_environment(work)
    lanes = LANES[kind]
    report = {
        "schema": 1, "kind": kind, "cpu": args.cpu, "pairs": args.pairs, "warmup_rounds": 2,
        "complete": False, "environment": environment,
        "build_manifests": {arm: identity(getattr(args, arm)) for arm in builds},
        "executables": staged, "samples": {lane: [] for lane in lanes},
        "warmup_samples": {lane: [] for lane in lanes},
        "measurement_scope": "Alternating fresh processes, rotated lanes, two warmup rounds excluded from statistics; "
                             "both exact executables staged on the same filesystem. Storage describes this "
                             "warmed cohort, not a historical executable location. No claim of machine-wide idle CPUs.",
        "primary_metric": "wall ns/operation (unchanged keyframe workload)" if kind == "keyframes"
                          else "thread CPU ns/operation",
        "host_stat_before": Path("/proc/stat").read_text(),
    }
    save(output, report)
    for round_index in range(-2, args.pairs):
        offset = abs(round_index) % len(lanes)
        order = ("reference", "candidate") if round_index % 2 == 0 else ("candidate", "reference")
        for lane in lanes[offset:] + lanes[:offset]:
            pair = {}
            for arm in order:
                ran = invoke(executables[arm], [lane], env)
                if ran["exit_code"]:
                    report["failed_run"] = {"round": round_index, "lane": lane, "arm": arm, **ran}
                    save(output, report)
                    raise RuntimeError(f"timing process failed; see {output}")
                try:
                    pair[arm] = {**parse_timing(kind, lane, ran["stdout"]), **ran}
                except (ValueError, RuntimeError):
                    report["failed_run"] = {"round": round_index, "lane": lane, "arm": arm, **ran}
                    save(output, report)
                    raise
            if pair["reference"]["operations"] != pair["candidate"]["operations"]:
                raise RuntimeError("operation count mismatch")
            if round_index >= 0:
                report["samples"][lane].append(pair)
            else:
                report["warmup_samples"][lane].append(pair)
            save(output, report)
    report["host_stat_after"] = Path("/proc/stat").read_text()
    report["summary"] = {}
    for lane, samples in report["samples"].items():
        reference = [row["reference"]["metric_ns_per_operation"] for row in samples]
        candidate = [row["candidate"]["metric_ns_per_operation"] for row in samples]
        report["summary"][lane] = {"reference_median": statistics.median(reference),
                                   "candidate_median": statistics.median(candidate),
                                   "candidate_over_reference": paired_ratio(candidate, reference)}
    for arm, executable in executables.items():
        if identity(executable) != staged[arm]["staged"]:
            raise RuntimeError("executable changed during timing")
    report["complete"] = True
    save(output, report)
    print(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="action", required=True)
    materializer = subparsers.add_parser("materialize")
    materializer.add_argument("--source-root", type=Path, default=ROOT)
    materializer.add_argument("--output", type=Path, required=True)
    materializer.add_argument("--receiver-policy", choices=("borrowed", "cloned"), default="borrowed")
    builder = subparsers.add_parser("build")
    builder.add_argument("--fixture", type=Path, required=True)
    builder.add_argument("--kind", choices=SOURCES, required=True)
    builder.add_argument("--base", type=Path, required=True)
    builder.add_argument("--dependency", type=Path, action="append", required=True)
    builder.add_argument("--profile", type=Path)
    builder.add_argument("--rustc", default="rustc")
    builder.add_argument("--mode", choices=("dev", "production"), required=True)
    builder.add_argument("--dev-lto", choices=("off", "thin"), default="off",
                         help="development-only LTO; thin can link a bitcode-only Base archive")
    builder.add_argument("--dev-opt-level", choices=("0", "1"), default="1",
                         help="development-only optimization level; assertions remain enabled")
    builder.add_argument("--source-root", type=Path, default=ROOT)
    builder.add_argument("--output", type=Path, required=True)
    checker = subparsers.add_parser("check")
    checker.add_argument("--build", type=Path, required=True)
    checker.add_argument("--out", type=Path, required=True)
    measurement = subparsers.add_parser("measure")
    measurement.add_argument("--reference", type=Path, required=True)
    measurement.add_argument("--candidate", type=Path, required=True)
    measurement.add_argument("--out", type=Path, required=True)
    measurement.add_argument("--cpu", type=int, required=True)
    measurement.add_argument("--pairs", type=int, default=31)
    args = parser.parse_args()
    {"materialize": materialize, "build": build, "check": check, "measure": measure}[args.action](args)


if __name__ == "__main__":
    main()
