#!/usr/bin/env python3
"""Materialize and replay public property gates against an explicitly supplied Base rlib.

The historical QA and mixed-property inputs come from the committed, hash-indexed
evidence, not private API substitutes. Nine current public regressions are copied
verbatim from the repository. Preparation never invokes a compiler. Build, check
and measure are separate commands so compiler and timing windows can be serialized.

Example:
  prepare_property_validation.py materialize --work-dir ../property-gates
  prepare_property_validation.py build --fixture ../property-gates/fixture.json \
      --work-dir ../property-gates/candidate --kind qa --mode dev \
      --rustc /path/to/rustc --base /graph/libferroui_base-HASH.rlib \
      --bitflags /graph/libbitflags-HASH.rlib --dependency /graph --dependency /host/deps
  prepare_property_validation.py check --build ../property-gates/candidate/qa/build.json

Production companion links use ordinary unwind, opt-z, FatLTO, addrsig, safe ICF
and RELR. They do not claim the shipping executable's compact-unwind layout.
Allocation companions are separate binaries and never supply timing evidence.
Development links default to opt1 with LTO off. --dev-opt-level 0 --dev-lto thin
selects the low-memory bitcode-library validation experiment without changing
the production policy, debug assertions, reload cfg, or selected test cases.
"""

import argparse
import errno
import hashlib
import json
import os
from pathlib import Path
import random
import re
import shutil
import statistics
import subprocess
import time


HERE = Path(__file__).resolve().parent
REPOSITORY = HERE.parents[2]
EVIDENCE = Path("docs/benchmarks/compiled-theme-rust-idioms-linux-x86_64.json")
OLD_FIXTURE = "/tmp/recover-xaml/property-qa"
MIXED_SOURCE = "/tmp/ferro-run/property-stage5/property_workload.rs"
REGRESSIONS = {
    "ferro_object_tests_binding": [
        "boxed_bindings_snapshot_values_instead_of_retaining_the_incoming_box",
        "retained_boxed_source_observer_does_not_keep_a_disposed_bindings_values_alive",
        "boxed_frame_keeps_its_first_fallback_across_unsubscribe_and_resubscribe",
        "boxed_notification_keeps_snapshots_and_drops_them_after_payload_clone",
        "boxed_binding_conversion_releases_its_temporary_before_delivery_or_clone_unwind",
        "boxed_input_drop_runs_after_the_delivery_releases_its_binding",
    ],
    "ferro_object_tests_set_value": [
        "untyped_markers_preserve_style_and_direct_priority_rules",
        "untyped_conversion_temporary_drops_before_validation_or_coercion",
        "untyped_conversion_releases_its_temporary_when_value_clone_panics",
    ],
}
SETTER_LANES = [
    "styled_set", "styled_current", "direct_set", "direct_current",
    "styled_markers", "direct_markers", "numeric_conversion",
]
COLD_LANES = ["local_notification", "style_notification", "local_conversion", "style_conversion"]
BOXED_LANES = ["local_notify", "style_notify", "local_install", "style_install"]
TESTS = {
    "bindings": "boxed_timing::profile_boxed_binding",
    "cold": "cold_timing::profile_cold_binding",
    "setters": "setter_routes::profile_property_setters",
}
HOT_RELOAD_TEST = "typed_cases::typed_binding_reload_preserves_subscription_and_tracks_context_during_rollback"


def sha_bytes(data):
    return hashlib.sha256(data).hexdigest()


def sha_file(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(path):
    path = Path(path).resolve(strict=True)
    return {"path": str(path), "sha256": sha_file(path), "bytes": path.stat().st_size}


def storage(path):
    path = Path(path).resolve(strict=True)
    return {"resolved_path": str(path), "st_dev": path.stat().st_dev}


def write_json(path, data):
    Path(path).write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def write_once(path, data):
    path = Path(path)
    if isinstance(data, str):
        data = data.encode()
    if path.exists():
        if path.read_bytes() != data:
            raise RuntimeError(f"refusing to replace an existing fixture input: {path}")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def checked_source(evidence, source_sha):
    entry = evidence["sources_by_sha256"][source_sha]
    data = entry["text"].encode(entry["encoding"])
    if len(data) != entry["bytes"] or sha_bytes(data) != source_sha:
        raise RuntimeError(f"retained source identity mismatch: {source_sha}")
    return data.decode("utf-8")


def located_source(evidence, path):
    return checked_source(evidence, evidence["source_locations"][path]["sha256"])


def append_regressions(repository, fixture, adaptations):
    for module, names in REGRESSIONS.items():
        original = repository / "src/FerroUI.Base/tests" / (module + ".rs")
        source = original.read_text()
        start = source.index("#[test]\nfn " + names[0] + "()")
        if module == "ferro_object_tests_binding":
            end = len(source)
            destination = fixture / "binding_tests.rs"
        else:
            end = source.index("#[test]\nfn clear_value_clears_value()", start)
            destination = fixture / (module + ".rs")
        fragment = source[start:end].rstrip() + "\n"
        actual = re.findall(r"(?m)^fn (\w+)\(", fragment)
        if actual != names or fragment.count("#[test]") != len(names):
            raise RuntimeError(f"regression fragment boundaries changed: {original}: {actual}")
        if any("fn " + name + "(" in destination.read_text() for name in names):
            raise RuntimeError(f"regression already present in retained fixture: {destination}")
        destination.write_text(destination.read_text().rstrip() + "\n\n" + fragment)
        adaptations.append({
            "operation": "append exact current public regression bodies",
            "source": identity(original), "character_range": [start, end],
            "fragment_sha256": sha_bytes(fragment.encode()), "destination": destination.name,
            "test_names": [f"tests::{module}::{name}" for name in names],
            "scope": "Bodies are copied verbatim; existing Class1/Class2 and public test helpers are reused.",
        })


def allocation_timing(source):
    additions = [
        ("    let start = thread_cpu_ns();\n", "    let start = thread_cpu_ns();\n    crate::allocation::begin();\n"),
        ("    let ns = thread_cpu_ns() - start;\n", "    let allocation = crate::allocation::end();\n    let ns = thread_cpu_ns() - start;\n"),
        ("    (checksum, ns, wall)\n", """    let lane = match (priority, mode) {
        (BindingPriority::LocalValue, Mode::Notify) => "local_notify",
        (BindingPriority::LocalValue, Mode::Install) => "local_install",
        (_, Mode::Notify) => "style_notify",
        (_, Mode::Install) => "style_install",
    };
    println!("ALLOCATION {lane} {count} {checksum} {} {}", allocation.requests, allocation.requested_bytes);
    (checksum, ns, wall)
"""),
    ]
    for before, after in additions:
        if source.count(before) != 1:
            raise RuntimeError("retained boxed allocation adapter no longer matches its exact source")
        source = source.replace(before, after)
    return source, [{"before": before, "after": after} for before, after in additions]


def materialize(args):
    repository = args.source_root.resolve()
    evidence_path = args.evidence.resolve() if args.evidence else repository / EVIDENCE
    evidence = json.loads(evidence_path.read_text())
    work = args.work_dir.resolve()
    fixture = work / "fixture"
    retained = work / "retained"
    manifest_path = work / "fixture.json"
    if manifest_path.exists():
        raise RuntimeError("use a fresh materialization directory; existing fixture manifests are immutable")
    adaptations = []
    historical_hashes = evidence["qa_materialization"]["exact_reconstruction_sha256"]
    for name, source_sha in historical_hashes.items():
        if Path(name).name != name:
            raise RuntimeError(f"unsafe fixture filename: {name}")
        text = checked_source(evidence, source_sha)
        write_once(retained / name, text)
        destination = str(fixture).replace("\\", "\\\\").replace('"', '\\"')
        adapted = text.replace(OLD_FIXTURE, destination)
        write_once(fixture / name, adapted)
        if text != adapted:
            adaptations.append({"operation": "replace recorded fixture path", "file": name,
                                "from": OLD_FIXTURE, "to": str(fixture), "occurrences": text.count(OLD_FIXTURE)})
    append_regressions(repository, fixture, adaptations)
    setter_source = repository / "scripts/themes/bench/property_registration_routes.rs"
    setter_text = setter_source.read_text()
    setter_adapter = """
#[test]
#[ignore]
fn profile_property_setters() {
    print!("SETTER ");
    run_markers();
}
"""
    write_once(fixture / "setter_routes.rs", setter_text + setter_adapter)
    adaptations.append({"operation": "append ignored wrapper around unchanged run_markers",
                        "source": identity(setter_source), "appended_text": setter_adapter})
    for name in ["property_validation_cold.rs", "property_allocation_counter.rs"]:
        path = HERE / name
        write_once(fixture / name, path.read_bytes())
        adaptations.append({"operation": "copy producer companion", "source": identity(path)})
    qa = (fixture / "qa.rs").read_text() + """
#[path = "property_validation_cold.rs"] mod cold_timing;
#[path = "setter_routes.rs"] mod setter_routes;
"""
    (fixture / "qa.rs").write_text(qa)
    allocation_source, allocation_adapter = allocation_timing((fixture / "timing.rs").read_text())
    write_once(fixture / "timing_alloc.rs", allocation_source)
    qa_alloc = qa.replace(str(fixture / "timing.rs"), str(fixture / "timing_alloc.rs"))
    if qa_alloc == qa:
        raise RuntimeError("allocation root did not replace the retained timing module")
    qa_alloc += """
#[path = "property_allocation_counter.rs"] mod allocation;
#[global_allocator] static ALLOCATION_COUNTER: allocation::Counter = allocation::Counter;
"""
    write_once(fixture / "qa_alloc.rs", qa_alloc)
    adaptations.append({"operation": "derive separate count-only boxed timing module",
                        "source": "timing.rs", "destination": "timing_alloc.rs", "edits": allocation_adapter,
                        "scope": "Original notify and bind/get/dispose loops are unchanged. End-of-iteration lease drop is counted. Setup, warmup results and post-loop disposal are excluded from reported full-count results."})
    mixed = located_source(evidence, MIXED_SOURCE)
    write_once(fixture / "mixed_property.rs", mixed)
    mixed_rows = evidence["evidence"]["stage5.property_cpu6"]["data"]["raw_samples"]["reference"][0]["rows"]
    expectations = {
        "bindings": {name: {"operations": 12_000 if name.endswith("install") or name.startswith("typed_") else 40_000,
                             "checksum": int(value)}
                     for name, value in evidence["evidence"]["stage5.bindings_cpu6"]["data"]["checksums"].items()},
        "mixed": {row["lane"]: {"operations": row["operations"], "checksum": row["checksum"]} for row in mixed_rows},
        "cold": {name: {"operations": 40_000, "checksum": 400_000} for name in COLD_LANES},
        "setters": {name: {"operations": 90_000, "checksum": None} for name in SETTER_LANES},
    }
    private_exclusions = {}
    for name in historical_hashes:
        lines = (retained / name).read_text().splitlines()
        snippets = ["\n".join(lines[i:i + 5]) for i, line in enumerate(lines) if "#[cfg(any())]" in line]
        if snippets:
            private_exclusions[name] = snippets
    manifest = {
        "schema_version": 1, "source_root": str(repository), "producer": identity(Path(__file__)),
        "evidence": identity(evidence_path), "historical_source_hashes": historical_hashes,
        "fixture_directory": str(fixture), "files": {p.name: identity(p) for p in sorted(fixture.glob("*.rs"))},
        "adaptations": adaptations, "preserved_private_exclusions": private_exclusions,
        "new_regressions": [f"tests::{module}::{name}" for module, names in REGRESSIONS.items() for name in names],
        "expected_correctness": {"passed": 347, "failed": 0, "ignored": 3},
        "expected_dev_hot_reload_correctness": {"passed": 348, "failed": 0, "ignored": 3},
        "gated_hot_reload_test": HOT_RELOAD_TEST,
        "expectations": expectations,
        "scope": {"qa": "Historical 338 public cases plus nine verbatim current regressions; no private API substitutes.",
                  "bindings": "Historical seven-lane timing.rs is byte-identical; its path-bearing parent wrapper is relocated.",
                  "setters": "Current committed run_markers is copied unchanged; only an ignored test wrapper is appended.",
                  "mixed": "Historical 11-lane stage5 property workload is byte-identical.",
                  "cold": "Reconstructed four-lane fixture; do not label it byte-identical to lost temporary stage6 inputs.",
                  "allocation": "Separate counter link; requests and cumulative requested bytes, not live or retained bytes."},
    }
    write_json(manifest_path, manifest)
    print(manifest_path)


def verify_fixture(path):
    manifest = json.loads(Path(path).read_text())
    for expected in manifest["files"].values():
        if identity(expected["path"]) != expected:
            raise RuntimeError(f"fixture changed: {expected['path']}")
    return manifest


def clean_environment(tmpdir):
    env = os.environ.copy()
    removed = {}
    for key in list(env):
        if key in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "LLVM_PROFILE_FILE"} or key.startswith(("FERROUI_PGO_", "FERROUI_BENCH_EXPECT_")):
            removed[key] = env.pop(key)
    env["LLVM_PROFILE_FILE"] = "/dev/null"
    env["TMPDIR"] = str(tmpdir)
    return env, {"removed_names": sorted(removed), "LLVM_PROFILE_FILE": "/dev/null", "TMPDIR": str(tmpdir)}


def dependency_inventory(path):
    path = path.resolve(strict=True)
    files = {p.name: sha_file(p) for p in sorted(path.iterdir())
             if p.is_file() and (p.suffix in {".rlib", ".rmeta", ".so", ".a", ".o"} or ".so." in p.name)}
    return {"path": str(path), "files": files, "inventory_sha256": sha_bytes(json.dumps(files, sort_keys=True).encode())}


def run_logged(command, directory, stem, env):
    stdout, stderr = directory / (stem + ".stdout"), directory / (stem + ".stderr")
    started = time.time()
    with stdout.open("w") as out, stderr.open("w") as err:
        result = subprocess.run(command, env=env, stdout=out, stderr=err)
    return {"command": [str(x) for x in command], "exit_code": result.returncode,
            "elapsed_seconds": time.time() - started, "stdout": identity(stdout), "stderr": identity(stderr)}


def build(args):
    if args.mode == "production" and (args.dev_lto is not None or args.dev_opt_level is not None):
        raise RuntimeError("--dev-lto and --dev-opt-level are development-only options")
    development_policy = None
    if args.mode == "dev":
        development_policy = {
            "lto": args.dev_lto if args.dev_lto is not None else "off",
            "opt_level": args.dev_opt_level if args.dev_opt_level is not None else 1,
        }
    fixture_manifest = verify_fixture(args.fixture)
    fixture = Path(fixture_manifest["fixture_directory"])
    output = args.work_dir.resolve() / args.kind
    output.mkdir(parents=True, exist_ok=True)
    manifest_path = output / "build.json"
    if manifest_path.exists() or (output / "planned-build.json").exists() or (output / "companion").exists():
        raise RuntimeError("use a fresh build directory; previous build results are preserved")
    temporary = output / "tmp"
    temporary.mkdir(exist_ok=True)
    env, environment = clean_environment(temporary)
    rustc = str(Path(shutil.which(str(args.rustc), path=env.get("PATH")) or args.rustc).resolve(strict=True))
    rustc_version = subprocess.check_output([rustc, "-vV"], env=env, text=True)
    target = args.target
    if target != "x86_64-unknown-linux-gnu":
        raise RuntimeError("these clock and linker fixtures require x86_64-unknown-linux-gnu")
    sysroot = Path(subprocess.check_output([rustc, "--print=sysroot"], env=env, text=True).strip())
    bundled = sysroot / "lib/rustlib" / target / "bin/gcc-ld"
    if (bundled / "ld.lld").is_file():
        env["PATH"] = str(bundled) + os.pathsep + env.get("PATH", os.defpath)
    linker = shutil.which(str(args.linker), path=env.get("PATH"))
    if linker is None:
        raise RuntimeError(f"linker compiler driver is not available: {args.linker}")
    source = fixture / {"qa": "qa.rs", "allocations": "qa_alloc.rs", "mixed": "mixed_property.rs"}[args.kind]
    executable = output / "companion"
    command = [rustc, "--edition=2021", "--crate-name", "property_companion", "--target", target,
               "-Cpanic=unwind", "-Cdebuginfo=0", "-Clink-arg=-fuse-ld=lld"]
    if args.mode == "production":
        if not args.profile:
            raise RuntimeError("production companion links require the matching --profile")
        command += ["-Copt-level=z", "-Cdebug-assertions=no", "-Ccodegen-units=1", "-Clto=fat", "-Cstrip=symbols",
                    "-Cllvm-args=--addrsig", "-Clink-arg=-Wl,--icf=safe",
                    "-Clink-arg=-Wl,-z,pack-relative-relocs", "-Cprofile-use=" + str(args.profile.resolve())]
    else:
        command += [f"-Copt-level={development_policy['opt_level']}",
                    f"-Clto={development_policy['lto']}", "-Cdebug-assertions=yes"]
    if args.hot_reload:
        command += ["--cfg", 'feature="hot-reload"']
    if args.kind != "mixed":
        command += ["--test"]
    if args.kind == "allocations":
        command += ["--cfg", "ferro_count_allocations"]
    command += ["-C", "linker=" + linker, "-Clink-arg=-Wl,--threads=1"]
    dependencies = [args.base.resolve().parent, *[p.resolve() for p in args.dependency]]
    dependencies = list(dict.fromkeys(dependencies))
    for directory in dependencies:
        command += ["-L", "dependency=" + str(directory)]
    command += ["--extern", "ferroui_base=" + str(args.base.resolve()),
                "--extern", "bitflags=" + str(args.bitflags.resolve()), str(source), "-o", str(executable)]
    manifest = {
        "schema_version": 1, "kind": args.kind, "mode": args.mode, "fixture": identity(args.fixture),
        "producer": identity(Path(__file__)),
        "development_policy": development_policy,
        "hot_reload_cfg": args.hot_reload, "debug_assertions": args.mode == "dev", "target": target,
        "source": identity(source), "base": identity(args.base), "bitflags": identity(args.bitflags),
        "dependencies": [dependency_inventory(p) for p in dependencies],
        "profile": identity(args.profile) if args.profile else None,
        "rustc": {**identity(rustc), "version": rustc_version},
        "linker": {**identity(linker), "invoked_as": linker,
                   "version": subprocess.check_output([linker, "--version"], env=env, text=True)},
        "lld": identity(bundled / "ld.lld") if (bundled / "ld.lld").is_file() else None,
        "environment": {**environment, "PATH": env.get("PATH"), "tmp_storage": storage(temporary)},
        "link_policy": "ordinary unwind; no compact-unwind relink", "command": command,
    }
    write_json(output / "planned-build.json", manifest)
    if args.cpu is not None:
        os.sched_setaffinity(0, {args.cpu})
    manifest["compiler"] = run_logged(command, output, "compiler", env)
    if manifest["compiler"]["exit_code"] == 0:
        manifest["executable"] = {**identity(executable), "storage": storage(executable)}
    write_json(manifest_path, manifest)
    # Preserve even a failed compiler result before checking its inputs again.
    # A library rebuilt in place must not inherit the earlier identity record.
    verify_fixture(args.fixture)
    for name, path in [("base", args.base), ("bitflags", args.bitflags), ("profile", args.profile)]:
        if path is not None and identity(path) != manifest[name]:
            raise RuntimeError(f"{name} changed while the companion was compiling")
    manifest["inputs_unchanged_after_compiler"] = True
    write_json(manifest_path, manifest)
    print(manifest_path)
    if manifest["compiler"]["exit_code"] != 0:
        raise RuntimeError(f"companion compilation failed; inspect {output / 'compiler.stderr'}")


def verified_build(path):
    manifest = json.loads(Path(path).read_text())
    if manifest["compiler"]["exit_code"] != 0:
        raise RuntimeError(f"build did not complete: {path}")
    if manifest.get("inputs_unchanged_after_compiler") is not True:
        raise RuntimeError(f"build inputs were not verified after the compiler: {path}")
    expected = manifest["executable"]
    if identity(expected["path"]) != {k: expected[k] for k in ("path", "sha256", "bytes")}:
        raise RuntimeError(f"executable changed: {expected['path']}")
    if sha_file(manifest["fixture"]["path"]) != manifest["fixture"]["sha256"]:
        raise RuntimeError("fixture manifest changed after compilation")
    return manifest, verify_fixture(manifest["fixture"]["path"])


def test_summary(stdout):
    matches = re.findall(r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;", stdout)
    if len(matches) != 1:
        raise RuntimeError("expected exactly one Rust test summary")
    status, passed, failed, ignored = matches[0]
    if status != "ok":
        raise RuntimeError("Rust test result was not successful")
    return {"passed": int(passed), "failed": int(failed), "ignored": int(ignored)}


def check(args):
    manifest, fixture = verified_build(args.build)
    if manifest["kind"] not in {"qa", "allocations"}:
        raise RuntimeError("correctness checks require a QA or allocation companion")
    output = args.work_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if (output / "check.json").exists():
        raise RuntimeError("use a fresh check directory; previous successes and failures are preserved")
    temporary = output / "tmp"
    temporary.mkdir(exist_ok=True)
    env, environment = clean_environment(temporary)
    binary = manifest["executable"]["path"]
    report = {"build": identity(args.build), "suite": args.suite, "environment": environment, "runs": []}
    inventory = run_logged([binary, "--list", "--format=terse"], output, "test-inventory", env)
    report["test_inventory"] = inventory
    write_json(output / "check.json", report)
    if inventory["exit_code"] != 0:
        raise RuntimeError("compiled test inventory failed")
    names = {line.removesuffix(": test") for line in Path(inventory["stdout"]["path"]).read_text().splitlines()
             if line.endswith(": test")}
    if not set(fixture["new_regressions"]).issubset(names):
        raise RuntimeError("one or more exact current regression names are absent from the executable")
    reload_enabled = manifest["debug_assertions"] and manifest["hot_reload_cfg"]
    if (HOT_RELOAD_TEST in names) != reload_enabled:
        raise RuntimeError("compiled hot-reload case does not match the recorded cfg/debug-assertion policy")
    report["compiled_test_names"] = sorted(names)
    if args.suite == "correctness":
        expected = fixture["expected_dev_hot_reload_correctness"] if reload_enabled else fixture["expected_correctness"]
        cases = [("all", ["--test-threads=1"], expected)]
    elif args.suite == "regressions":
        cases = [(name.rsplit("::", 1)[-1], ["--exact", name, "--test-threads=1"], {"passed": 1, "failed": 0, "ignored": 0})
                 for name in fixture["new_regressions"]]
    else:
        if manifest["kind"] != "allocations":
            raise RuntimeError("allocation counts require the separate allocation companion")
        cases = [(suite, ["--exact", TESTS[suite], "--ignored", "--nocapture", "--test-threads=1"],
                  {"passed": 1, "failed": 0, "ignored": 0}) for suite in ("bindings", "cold")]
    for stem, arguments, expected in cases:
        run = run_logged([binary, *arguments], output, stem, env)
        report["runs"].append(run)
        write_json(output / "check.json", report)
        if run["exit_code"] != 0:
            raise RuntimeError(f"public fixture failed: {stem}; inspect {run['stdout']['path']}")
        stdout = Path(run["stdout"]["path"]).read_text()
        run["summary"] = test_summary(stdout)
        if run["summary"] != expected:
            raise RuntimeError(f"unexpected public fixture counts: {run['summary']} != {expected}")
        if args.suite == "allocations":
            rows = re.findall(r"ALLOCATION (\w+) (\d+) (-?\d+) (\d+) (\d+)", stdout)
            collected = {}
            for lane, count, checksum, requests, requested_bytes in rows:
                row = dict(operations=int(count), checksum=int(checksum), requests=int(requests), requested_bytes=int(requested_bytes))
                if lane not in collected or row["operations"] > collected[lane]["operations"]:
                    collected[lane] = row
            names = BOXED_LANES if stem == "bindings" else COLD_LANES
            if set(collected) != set(names):
                raise RuntimeError(f"unexpected allocation lanes: {collected.keys()}")
            for name, row in collected.items():
                expected_row = fixture["expectations"][stem][name]
                if {key: row[key] for key in expected_row} != expected_row:
                    raise RuntimeError(f"allocation checksum/count mismatch: {name}")
            run["allocation_counts"] = collected
    report["complete"] = True
    write_json(output / "check.json", report)
    print(output / "check.json")


def stage_binary(source, destination):
    source = Path(source).resolve(strict=True)
    expected = identity(source)
    if destination.exists():
        if destination.stat().st_size != expected["bytes"] or sha_file(destination) != expected["sha256"]:
            raise RuntimeError(f"existing staged binary has different bytes: {destination}")
        method = "verified existing artifact"
    else:
        temporary = destination.with_suffix(".staging")
        if temporary.exists():
            raise RuntimeError(f"unfinished previous staging operation: {temporary}")
        try:
            os.link(source, temporary)
            method = "hard link"
        except OSError as error:
            if error.errno != errno.EXDEV:
                raise
            shutil.copy2(source, temporary)
            method = "verified copy2 after EXDEV"
        if temporary.stat().st_size != expected["bytes"] or sha_file(temporary) != expected["sha256"]:
            raise RuntimeError(f"staged binary identity mismatch: {temporary}")
        temporary.rename(destination)
    return {"original": expected, "staging": method, **identity(destination), "storage": storage(destination)}


def host_activity(cpu):
    result = {"unix_seconds": time.time(), "pid": os.getpid(), "affinity": sorted(os.sched_getaffinity(0))}
    for name in ["loadavg", "pressure/cpu", "pressure/memory"]:
        path = Path("/proc") / name
        if path.exists():
            result[name] = path.read_text().strip()
    result["cpu_stat"] = [line for line in Path("/proc/stat").read_text().splitlines()
                          if line.startswith("cpu ") or line.startswith(f"cpu{cpu} ")]
    cgroup = Path("/sys/fs/cgroup/cpu.stat")
    if cgroup.exists():
        result["cgroup_cpu_stat"] = cgroup.read_text()
    return result


def timing_rows(suite, stdout, expectations):
    if suite == "setters":
        lines = [line.split("SETTER ", 1)[1] for line in stdout.splitlines() if "SETTER " in line]
        if len(lines) != 1:
            raise RuntimeError("expected one unchanged run_markers output row")
        times = [int(value) for value in lines[0].split()]
        if len(times) != len(SETTER_LANES):
            raise RuntimeError("expected seven setter timing fields")
        rows = {name: {"operations": 90_000, "checksum": None, "thread_cpu_ns": cpu, "wall_ns": None}
                for name, cpu in zip(SETTER_LANES, times, strict=True)}
    else:
        prefix = {"bindings": "BOXED ", "cold": "COLD ", "mixed": ""}[suite]
        rows = {}
        for line in stdout.splitlines():
            if prefix:
                if prefix not in line:
                    continue
                line = line.split(prefix, 1)[1]
            fields = line.split()
            if len(fields) != 5 or fields[0] not in expectations:
                continue
            lane, count, checksum, cpu, wall = fields
            if lane in rows:
                raise RuntimeError(f"duplicate timing lane: {lane}")
            rows[lane] = {"operations": int(count), "checksum": int(checksum),
                          "thread_cpu_ns": int(cpu), "wall_ns": int(wall)}
    if set(rows) != set(expectations):
        raise RuntimeError(f"timing lane mismatch: {sorted(rows)} != {sorted(expectations)}")
    for name, row in rows.items():
        if {key: row[key] for key in expectations[name]} != expectations[name] or row["thread_cpu_ns"] <= 0:
            raise RuntimeError(f"timing checksum/count mismatch: {name}: {row}")
    return rows


def ratio(candidate, reference, suite):
    ratios = [a / b for a, b in zip(candidate, reference, strict=True)]
    draws, seed = (9000, 8676) if suite == "bindings" else (7000, 9961)
    rng = random.Random(seed)
    bootstrap = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(draws))
    low = max(0, int(draws * 0.025) - 1)
    high = min(draws - 1, int(draws * 0.975) - 1)
    return {"candidate_over_reference_median": statistics.median(ratios),
            "paired_bootstrap_95_percent": [bootstrap[low], bootstrap[high]],
            "bootstrap_draws": draws, "bootstrap_seed": seed,
            "reference_median_ns_per_operation": statistics.median(reference),
            "candidate_median_ns_per_operation": statistics.median(candidate)}


def measure(args):
    if not hasattr(os, "sched_getaffinity") or args.cpu not in os.sched_getaffinity(0):
        raise RuntimeError("--cpu must name an allowed Linux CPU")
    os.sched_setaffinity(0, {args.cpu})
    output = args.work_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    report_path = output / "measurement.json"
    if report_path.exists():
        raise RuntimeError("use a fresh timing directory; completed or interrupted cohorts are preserved")
    temporary = output / "tmp"
    temporary.mkdir(exist_ok=True)
    env, environment = clean_environment(temporary)
    builds, fixtures, executables, full_builds = {}, {}, {}, {}
    for label, path in [("reference", args.reference_build), ("candidate", args.candidate_build)]:
        build_manifest, fixture = verified_build(path)
        full_builds[label] = build_manifest
        wanted = "mixed" if args.suite == "mixed" else "qa"
        if build_manifest["kind"] != wanted:
            raise RuntimeError(f"{label}: {args.suite} needs a {wanted} link; allocation companions are not timing artifacts")
        builds[label] = {"manifest": identity(path), "mode": build_manifest["mode"],
                         "base": build_manifest["base"], "profile": build_manifest["profile"]}
        fixtures[label] = fixture
        executables[label] = stage_binary(build_manifest["executable"]["path"], output / label)
    for key in ("mode", "development_policy", "link_policy", "target", "debug_assertions", "hot_reload_cfg"):
        if full_builds["reference"][key] != full_builds["candidate"][key]:
            raise RuntimeError(f"paired builds differ in {key}")
    for key in ("rustc", "linker", "producer"):
        if full_builds["reference"][key]["sha256"] != full_builds["candidate"][key]["sha256"]:
            raise RuntimeError(f"paired builds have different {key} identities")
    if full_builds["reference"]["lld"] != full_builds["candidate"]["lld"]:
        raise RuntimeError("paired builds have different LLD identities")
    if fixtures["reference"]["expectations"] != fixtures["candidate"]["expectations"]:
        raise RuntimeError("reference and candidate fixture expectations differ")
    reference_files = fixtures["reference"]["files"]
    candidate_files = fixtures["candidate"]["files"]
    # Both arms must be linked from one immutable fixture. Different absolute
    # paths in wrappers can affect module-qualified PGO keys and are not hidden.
    if reference_files != candidate_files:
        raise RuntimeError("paired links must use the same immutable fixture manifest")
    if executables["reference"]["storage"]["st_dev"] != executables["candidate"]["storage"]["st_dev"]:
        raise RuntimeError("comparison executables are not on the same output filesystem")
    pairs = args.pairs or (25 if args.suite == "bindings" else 31)
    warmups = args.warmups if args.warmups is not None else (2 if args.suite == "bindings" else 3)
    if pairs < 3 or warmups < 0:
        raise RuntimeError("require at least three measured pairs and a nonnegative warmup count")
    expectations = fixtures["reference"]["expectations"][args.suite]
    report = {
        "schema_version": 1, "suite": args.suite, "cpu": args.cpu, "pairs": pairs, "warmups": warmups,
        "builds": builds, "executables": executables, "environment": environment,
        "host_before": host_activity(args.cpu), "raw_runs": [], "complete": False,
        "policy": "Alternating paired processes; startup excluded from thread CPU samples; staged on the same filesystem and warmed in-cohort. Shared-host activity is retained, not assumed absent.",
        "setter_validation": "run_markers public assertions are unchanged; it has no numeric checksum output.",
    }
    write_json(report_path, report)
    samples = {label: {lane: [] for lane in expectations} for label in builds}
    for pair in range(-warmups, pairs):
        labels = ["reference", "candidate"] if pair % 2 == 0 else ["candidate", "reference"]
        for label in labels:
            command = [executables[label]["path"]]
            if args.suite == "mixed":
                command += [str(abs(pair))]
            else:
                command += ["--ignored", "--exact", TESTS[args.suite], "--nocapture", "--test-threads=1"]
            run_env = {**env, "BINDING_ROUND": str(abs(pair))}
            stem = f"{args.suite}-{pair + warmups:03d}-{label}"
            run = run_logged(command, output, stem, run_env)
            run.update(pair=pair, label=label, warmup=pair < 0)
            report["raw_runs"].append(run)
            # Preserve a successful process's raw output even if parsing then
            # rejects a checksum or the next paired process is interrupted.
            write_json(report_path, report)
            if run["exit_code"] != 0:
                raise RuntimeError(f"timing process failed: {stem}")
            stdout = Path(run["stdout"]["path"]).read_text()
            run["rows"] = timing_rows(args.suite, stdout, expectations)
            if args.suite != "mixed" and test_summary(stdout) != {"passed": 1, "failed": 0, "ignored": 0}:
                raise RuntimeError("unexpected ignored benchmark test result")
            if pair >= 0:
                for lane, row in run["rows"].items():
                    samples[label][lane].append(row["thread_cpu_ns"] / row["operations"])
            write_json(report_path, report)
    report["samples_ns_per_operation"] = samples
    report["ratios"] = {lane: ratio(samples["candidate"][lane], samples["reference"][lane], args.suite) for lane in expectations}
    report["host_after"] = host_activity(args.cpu)
    for executable in executables.values():
        if identity(executable["path"]) != {key: executable[key] for key in ("path", "sha256", "bytes")}:
            raise RuntimeError("a comparison executable changed during timing")
        if storage(executable["path"]) != executable["storage"]:
            raise RuntimeError("comparison executable storage changed during timing")
    report["complete"] = True
    write_json(report_path, report)
    print(report_path)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    prepare = commands.add_parser("materialize", help="recover immutable inputs without compiling")
    prepare.add_argument("--source-root", type=Path, default=REPOSITORY)
    prepare.add_argument("--evidence", type=Path)
    prepare.add_argument("--work-dir", type=Path, required=True)
    prepare.set_defaults(handler=materialize)
    compile_parser = commands.add_parser("build", help="compile one companion against an actual supplied Base")
    compile_parser.add_argument("--fixture", type=Path, required=True)
    compile_parser.add_argument("--work-dir", type=Path, required=True)
    compile_parser.add_argument("--base", type=Path, required=True)
    compile_parser.add_argument("--bitflags", type=Path, required=True)
    compile_parser.add_argument("--dependency", type=Path, action="append", default=[])
    compile_parser.add_argument("--profile", type=Path)
    compile_parser.add_argument("--rustc", default="rustc")
    compile_parser.add_argument("--linker", default="cc")
    compile_parser.add_argument("--target", default="x86_64-unknown-linux-gnu")
    compile_parser.add_argument("--mode", choices=["dev", "production"], default="dev")
    compile_parser.add_argument("--dev-lto", choices=["off", "thin"],
                                help="development-only LTO policy (default: off)")
    compile_parser.add_argument("--dev-opt-level", type=int, choices=[0, 1],
                                help="development-only optimization level (default: 1)")
    compile_parser.add_argument("--kind", choices=["qa", "allocations", "mixed"], default="qa")
    compile_parser.add_argument("--cpu", type=int)
    compile_parser.add_argument("--hot-reload", action="store_true",
                                help="set the companion feature cfg; Base must have been built with matching public reload APIs")
    compile_parser.set_defaults(handler=build)
    checks = commands.add_parser("check", help="run public correctness, nine regressions, or count-only loops")
    checks.add_argument("--build", type=Path, required=True)
    checks.add_argument("--work-dir", type=Path, required=True)
    checks.add_argument("--suite", choices=["correctness", "regressions", "allocations"], default="correctness")
    checks.set_defaults(handler=check)
    measurements = commands.add_parser("measure", help="run a quiet paired cohort using two completed build manifests")
    measurements.add_argument("--reference-build", type=Path, required=True)
    measurements.add_argument("--candidate-build", type=Path, required=True)
    measurements.add_argument("--work-dir", type=Path, required=True)
    measurements.add_argument("--suite", choices=["bindings", "cold", "setters", "mixed"], required=True)
    measurements.add_argument("--cpu", type=int, required=True)
    measurements.add_argument("--pairs", type=int)
    measurements.add_argument("--warmups", type=int)
    measurements.set_defaults(handler=measure)
    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
