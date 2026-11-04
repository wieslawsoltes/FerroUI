#!/usr/bin/env python3
"""Replay the retained 156 template activations against an immutable production graph.

Materialize, build and check are separate phases. The original stage5 source and
recipe are recovered byte-for-byte from committed evidence and never executed
with their historical paths. The active fixture replaces only its optional added
resource check with a mandatory structural-case check for AdditionalEditProbe=31.
The original Button/ScrollViewer/NavigationView/Slider activation bodies and
root/tree expectations remain unchanged.

This is a correctness-only companion: opt-z, ThinLTO, 16 codegen units, ordinary
unwind, and no additional profile-use on the companion source. Its size and link
time are not shipping-executable or performance measurements. The supplied graph
profile is recorded for provenance; this script never trains or rebuilds libraries.

Example:
  prepare_template_validation.py materialize --output ../template-gate/fixture
  prepare_template_validation.py build --fixture ../template-gate/fixture/fixture.json \
      --graph /preserved/target-graph --dependency /matching/host-deps \
      --profile /preserved/graph.profdata --rustc /toolchain/bin/rustc \
      --cpu 4 --output ../template-gate/build
  prepare_template_validation.py check --build ../template-gate/build/build.json \
      --scalar-archive /archives/compact.fxp --structural-archive /archives/structural.fxp \
      --output ../template-gate/check
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TARGET = "x86_64-unknown-linux-gnu"
SOURCE_SHA = "574b3eb7938f79f1449a0af0b7e500df581bf9f420d69ee9d226308844c18674"
RECIPE_SHA = "8019de0af1a8fe468a3a10cf1a4adc3a249ee6a6f891073035a2604cd9fb60ce"
CRATES = ("ferroui_base", "ferroui_controls", "ferroui_markup_xaml",
          "ferroui_markup_xaml_loader", "ferroui_themes_winui")
TREES = {"Button": 2, "ScrollViewer": 8, "NavigationView": 26, "Slider": 2}
CASES = {
    "embedded": {"normal": 14, "compact": 14, "added_resource": None},
    "scalar": {"normal": 14, "compact": 17, "added_resource": None},
    "structural": {"normal": 19, "compact": 29, "added_resource": 31},
}
ORIGINAL_OPTIONAL_CHECK = '''    if let Ok(expected) = std::env::var("FERROUI_TEMPLATE_EXPECT_EXTRA_RESOURCE") {
        let value = theme.try_get_resource(
            &"AdditionalEditProbe".into(), Some(&ThemeVariant::dark()),
        ).flatten();
        assert_eq!(from_markup_value::<f64>(&value), Some(expected.parse::<f64>().unwrap()));
    }
'''
MANDATORY_CASE_CHECK = '''    match std::env::var("FERROUI_TEMPLATE_ARCHIVE_CASE").as_deref() {
        Ok("embedded" | "scalar") => {}
        Ok("structural") => structural_resource::assert_additional_edit_probe(&theme),
        _ => panic!("select the embedded, scalar, or structural template companion case"),
    }
'''
MODULE = '\n#[path = "template_structural_resource.rs"] mod structural_resource;\n'


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(path):
    path = Path(path).resolve(strict=True)
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest(path)}


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def fresh_directory(path):
    path = Path(path).resolve()
    path.mkdir(parents=True, exist_ok=True)
    if any(path.iterdir()):
        raise RuntimeError(f"use a fresh output directory; previous evidence is preserved: {path}")
    return path


def retained_source(evidence, source_sha):
    entry = evidence["sources_by_sha256"][source_sha]
    encoded = entry["text"].encode(entry["encoding"])
    if len(encoded) != entry["bytes"] or hashlib.sha256(encoded).hexdigest() != source_sha:
        raise RuntimeError(f"retained template source identity mismatch: {source_sha}")
    return encoded


def materialize(args):
    output = fresh_directory(args.output)
    evidence_path = args.source_root.resolve() / "docs/benchmarks/compiled-theme-rust-idioms-linux-x86_64.json"
    evidence = json.loads(evidence_path.read_text())
    original = retained_source(evidence, SOURCE_SHA).decode()
    recipe = retained_source(evidence, RECIPE_SHA)
    if evidence["evidence"]["stage5.templates_source"]["source_text_sha256"] != SOURCE_SHA:
        raise RuntimeError("retained stage5 template fixture selection changed")
    if evidence["evidence"]["stage5.templates_recipe"]["source_text_sha256"] != RECIPE_SHA:
        raise RuntimeError("retained stage5 template recipe selection changed")
    if original.count(ORIGINAL_OPTIONAL_CHECK) != 1:
        raise RuntimeError("original optional resource check is not uniquely identifiable")
    active = original.replace(ORIGINAL_OPTIONAL_CHECK, MANDATORY_CASE_CHECK) + MODULE
    # Invert the complete adaptation: no loader, factory, activation loop,
    # font assertion, visual-root assertion, or tree expectation may drift.
    if active.removesuffix(MODULE).replace(MANDATORY_CASE_CHECK, ORIGINAL_OPTIONAL_CHECK) != original:
        raise RuntimeError("template adaptation changed more than the specified resource check")
    (output / "retained_fixture.rs").write_text(original)
    (output / "retained_recipe.py").write_bytes(recipe)
    (output / "fixture.rs").write_text(active)
    probe = HERE / "template_structural_resource.rs"
    shutil.copy2(probe, output / probe.name)
    manifest = {
        "schema_version": 1, "directory": str(output), "producer": identity(Path(__file__)),
        "evidence": identity(evidence_path),
        "retained_source": {"sha256": SOURCE_SHA, "evidence_key": "stage5.templates_source"},
        "retained_recipe": {"sha256": RECIPE_SHA, "evidence_key": "stage5.templates_recipe", "executed": False},
        "added_check_source": identity(probe),
        "adaptation": {"before": ORIGINAL_OPTIONAL_CHECK, "after": MANDATORY_CASE_CHECK, "appended_module": MODULE,
                       "original_scope": "The retained fixture already had an optional environment-selected added-resource assertion.",
                       "added_scope": "Structural mode now requires a fixed 31.0 query/assert and emits a separate EDITABILITY marker; absence is a failure."},
        "files": {path.name: identity(path) for path in sorted(output.iterdir()) if path.is_file()},
        "cases": CASES, "nodes_including_control": TREES, "roots_per_activation": 1,
        "activations_per_control_per_case": 13, "expected_total_activations": 156,
        "scope": "Actual supplied Base/Controls/XAML/Theme libraries, unchanged original 156 activations, no runtime implementation overlays.",
    }
    save(output / "fixture.json", manifest)
    print(output / "fixture.json")


def verify_fixture(path):
    manifest = json.loads(Path(path).read_text())
    for name, expected in manifest["files"].items():
        if identity(expected["path"]) != expected:
            raise RuntimeError(f"immutable template input changed: {name}")
    return manifest


def environment(directory):
    temporary = directory / "tmp"
    temporary.mkdir()
    if temporary.stat().st_uid != os.getuid() or temporary.resolve().parent != directory.resolve():
        raise RuntimeError("temporary directory must be owned and contained in the output")
    env = os.environ.copy()
    removed = []
    for key in list(env):
        if key in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "LLVM_PROFILE_FILE"} or key.startswith((
            "FERROUI_PGO_", "FERROUI_BENCH_EXPECT_", "FERROUI_TEMPLATE_",
        )):
            removed.append(key)
            env.pop(key)
    values = {"TMPDIR": str(temporary), "LLVM_PROFILE_FILE": "/dev/null", "RAYON_NUM_THREADS": "1",
              "MALLOC_ARENA_MAX": "1",
              "MALLOC_CONF": "narenas:1,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,background_thread:false,retain:false"}
    env.update(values)
    return env, {"removed_names": sorted(removed), **values, "temporary_storage": {
        "resolved_path": str(temporary.resolve()), "st_dev": temporary.stat().st_dev}}


def inventory(directory):
    directory = Path(directory).resolve(strict=True)
    files = {path.name: digest(path) for path in sorted(directory.iterdir())
             if path.is_file() and (path.suffix in {".rlib", ".rmeta", ".so", ".a", ".o"} or ".so." in path.name)}
    return {"path": str(directory), "files": files,
            "inventory_sha256": hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()}


def run(command, output, stem, env):
    started = time.monotonic()
    with (output / (stem + ".stdout")).open("w") as stdout, (output / (stem + ".stderr")).open("w") as stderr:
        result = subprocess.run(command, env=env, stdout=stdout, stderr=stderr)
    return {"command": command, "exit_code": result.returncode, "seconds": time.monotonic() - started,
            "stdout": identity(output / (stem + ".stdout")), "stderr": identity(output / (stem + ".stderr"))}


def build(args):
    if args.cpu not in os.sched_getaffinity(0):
        raise RuntimeError("--cpu must name an allowed Linux CPU")
    os.sched_setaffinity(0, {args.cpu})
    fixture = verify_fixture(args.fixture)
    output = fresh_directory(args.output)
    env, settings = environment(output)
    rustc = shutil.which(str(args.rustc), path=env.get("PATH"))
    linker = shutil.which(str(args.linker), path=env.get("PATH"))
    if rustc is None or linker is None:
        raise RuntimeError("the selected rustc and native linker compiler driver must exist")
    version = subprocess.check_output([rustc, "-vV"], env=env, text=True)
    if f"host: {TARGET}" not in version:
        raise RuntimeError(f"this companion's retained linker policy requires native {TARGET}")
    sysroot = Path(subprocess.check_output([rustc, "--print=sysroot"], env=env, text=True).strip())
    lld = sysroot / "lib/rustlib" / TARGET / "bin/gcc-ld/ld.lld"
    if not lld.is_file():
        raise RuntimeError("require the selected rustc toolchain's bundled LLD")
    env["PATH"] = str(lld.parent) + os.pathsep + env.get("PATH", os.defpath)
    lld_backend = lld.parent.parent / "rust-lld"
    graph = args.graph.resolve(strict=True)
    libraries = {}
    for name in CRATES:
        matches = sorted(graph.glob("lib" + name + "-*.rlib"))
        if len(matches) != 1:
            raise RuntimeError(f"require exactly one immutable {name} archive in {graph}: {matches}")
        libraries[name] = identity(matches[0])
    dependencies = [inventory(path) for path in dict.fromkeys([graph, *[p.resolve() for p in args.dependency]])]
    source = Path(fixture["directory"]) / "fixture.rs"
    executable = output / "template-correctness"
    command = [rustc, str(source), "--edition=2021", "--crate-name", "template_correctness", "--target", TARGET,
               "--cfg", 'feature="retain-archive"', "-Copt-level=z", "-Clto=thin", "-Ccodegen-units=16",
               "-Cpanic=unwind", "-Cstrip=symbols", "-Clinker=" + linker,
               "-Clink-arg=-fuse-ld=lld", "-Clink-arg=-Wl,--threads=1"]
    command += ["-Ldependency=" + item["path"] for item in dependencies]
    for name, item in libraries.items():
        command += ["--extern", name + "=" + item["path"]]
    command += ["-o", str(executable)]
    report = {
        "schema_version": 1, "producer": identity(Path(__file__)), "fixture": identity(args.fixture),
        "fixture_contents": fixture, "source": identity(source), "libraries": libraries, "dependencies": dependencies,
        "library_graph_profile": identity(args.profile), "profile_used_on_companion": None,
        "rustc": {**identity(rustc), "invoked_as": rustc, "verbose_version": version},
        "linker": {**identity(linker), "invoked_as": linker,
                   "version": subprocess.check_output([linker, "--version"], env=env, text=True)},
        "lld": identity(lld), "lld_backend": identity(lld_backend) if lld_backend.is_file() else None,
        "environment": {**settings, "PATH": env["PATH"]}, "cpu": args.cpu,
        "scope": "Production-library correctness companion: opt-z/ThinLTO/CGU16/ordinary unwind, no additional profile-use or training; no shipping-size or timing claim.",
        "command": command, "complete": False,
    }
    save(output / "build.json", report)
    report["compiler"] = run(command, output, "compiler", env)
    if report["compiler"]["exit_code"] == 0:
        report["executable"] = {**identity(executable), "st_dev": executable.stat().st_dev}
    save(output / "build.json", report)
    verify_fixture(args.fixture)
    for item in libraries.values():
        if identity(item["path"]) != item:
            raise RuntimeError("a selected library changed during the companion build")
    if identity(args.profile) != report["library_graph_profile"]:
        raise RuntimeError("library profile changed during the companion build")
    if [inventory(item["path"]) for item in dependencies] != dependencies:
        raise RuntimeError("dependency inventory changed during the companion build")
    if identity(args.fixture) != report["fixture"]:
        raise RuntimeError("fixture manifest changed during the companion build")
    for key in ("producer", "rustc", "linker", "lld", "lld_backend"):
        recorded = report[key]
        if recorded is not None and identity(recorded["path"]) != {name: recorded[name] for name in ("path", "sha256", "bytes")}:
            raise RuntimeError(f"{key} changed during the companion build")
    report["inputs_unchanged_after_compiler"] = True
    report["complete"] = report["compiler"]["exit_code"] == 0
    save(output / "build.json", report)
    if not report["complete"]:
        raise RuntimeError(f"template companion build failed; inspect {output / 'compiler.stderr'}")
    print(output / "build.json")


def read_build(path):
    report = json.loads(Path(path).read_text())
    if not report.get("complete") or not report.get("inputs_unchanged_after_compiler"):
        raise RuntimeError("template companion build is incomplete or its inputs were not verified")
    if digest(report["fixture"]["path"]) != report["fixture"]["sha256"]:
        raise RuntimeError("template fixture manifest changed after compilation")
    verify_fixture(report["fixture"]["path"])
    executable = report["executable"]
    if identity(executable["path"]) != {key: executable[key] for key in ("path", "sha256", "bytes")}:
        raise RuntimeError("template companion executable changed")
    return report


def parse_case(stdout, label, fixture):
    activations = {}
    extra_markers = []
    for line in stdout.splitlines():
        if not line.strip():
            continue
        if line.startswith("EDITABILITY "):
            extra_markers.append(line)
            continue
        fields = line.split()
        if len(fields) != 3 or fields[0] in activations:
            raise RuntimeError(f"unexpected template activation output: {line!r}")
        name, nodes, count = fields
        activations[name] = {"nodes_including_control": int(nodes), "activations": int(count)}
    expected = {name: {"nodes_including_control": count,
                       "activations": fixture["activations_per_control_per_case"]}
                for name, count in fixture["nodes_including_control"].items()}
    if activations != expected:
        raise RuntimeError(f"template activation counts differ: {activations}")
    expected_markers = ["EDITABILITY AdditionalEditProbe 31"] if label == "structural" else []
    if extra_markers != expected_markers:
        raise RuntimeError("structural resource assertion marker is missing, duplicated, or present in another case")
    return {"controls": activations, "activations": sum(row["activations"] for row in activations.values()),
            "fixed_added_resource_assertion_passed": True if label == "structural" else None}


def check(args):
    build_report = read_build(args.build)
    fixture = build_report["fixture_contents"]
    output = fresh_directory(args.output)
    env, settings = environment(output)
    archives = {"embedded": None, "scalar": identity(args.scalar_archive), "structural": identity(args.structural_archive)}
    report = {"schema_version": 1, "build": identity(args.build), "environment": settings,
              "archives": archives, "cases": {}, "complete": False,
              "scope": "Original 156 template activations plus one fixed structural AdditionalEditProbe=31 assertion; fresh processes against the actual recorded library graph."}
    save(output / "check.json", report)
    for label, expectations in fixture["cases"].items():
        command = [build_report["executable"]["path"]]
        if archives[label]:
            command.append(archives[label]["path"])
        case_settings = {
            "FERROUI_TEMPLATE_ARCHIVE_CASE": label,
            "FERROUI_BENCH_EXPECT_NORMAL_FONT_SIZE": str(expectations["normal"]),
            "FERROUI_BENCH_EXPECT_COMPACT_FONT_SIZE": str(expectations["compact"]),
        }
        row = run(command, output, label, {**env, **case_settings})
        row["environment"] = case_settings
        report["cases"][label] = row
        save(output / "check.json", report)
        if row["exit_code"] != 0:
            raise RuntimeError(f"{label} template assertions failed; inspect {row['stderr']['path']}")
        row["verified"] = parse_case(Path(row["stdout"]["path"]).read_text(), label, fixture)
        save(output / "check.json", report)
    for item in archives.values():
        if item is not None and identity(item["path"]) != item:
            raise RuntimeError("archive input changed during template verification")
    read_build(args.build)
    report["total_activations"] = sum(row["verified"]["activations"] for row in report["cases"].values())
    if report["total_activations"] != fixture["expected_total_activations"]:
        raise RuntimeError("template activation total differs from the retained 156-case expectation")
    report["complete"] = True
    save(output / "check.json", report)
    print(output / "check.json")


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="action", required=True)
    prepare = commands.add_parser("materialize")
    prepare.add_argument("--source-root", type=Path, default=ROOT)
    prepare.add_argument("--output", type=Path, required=True)
    prepare.set_defaults(handler=materialize)
    compiler = commands.add_parser("build")
    compiler.add_argument("--fixture", type=Path, required=True)
    compiler.add_argument("--graph", type=Path, required=True)
    compiler.add_argument("--dependency", type=Path, action="append", default=[])
    compiler.add_argument("--profile", type=Path, required=True, help="matching library graph profile, recorded without applying it to the companion")
    compiler.add_argument("--rustc", default="rustc")
    compiler.add_argument("--linker", default="cc")
    compiler.add_argument("--cpu", type=int, required=True, help="one allowed CPU for the retained serialized correctness-link policy")
    compiler.add_argument("--output", type=Path, required=True)
    compiler.set_defaults(handler=build)
    checker = commands.add_parser("check")
    checker.add_argument("--build", type=Path, required=True)
    checker.add_argument("--scalar-archive", type=Path, required=True)
    checker.add_argument("--structural-archive", type=Path, required=True)
    checker.add_argument("--output", type=Path, required=True)
    checker.set_defaults(handler=check)
    args = parser.parse_args()
    args.handler(args)


if __name__ == "__main__":
    main()
