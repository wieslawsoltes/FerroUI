#!/usr/bin/env python3
"""Import completed CI release evidence, then run the existing production gates.

The import is a separate, hash-verified path map; original CI manifests are never
rewritten. Companions use the imported production libraries and each arm's own
profile, with the existing producer policies. All links and correctness checks
finish before any paired workload. No command rebuilds Base or trains PGO.

Downloaded shipping executables form a separate CI-link/local-execution cohort.
Startup remains pending unless their host-loader checks and editable-archive
smoke pass. A completed measurement is evidence, not automatic release approval.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys


HERE = Path(__file__).resolve().parent
HOST = "x86_64-unknown-linux-gnu"
ARMS = ("reference", "candidate")
CALLABLE = ("receivers", "getters", "animation", "animation-allocations", "keyframes")
PROPERTY = ("qa", "allocations", "mixed")


def identity(path):
    path = Path(path).resolve(strict=True)
    with path.open("rb") as stream:
        sha = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path), "sha256": sha, "bytes": path.stat().st_size}


def read(path):
    return json.loads(Path(path).read_text())


def save(path, data):
    path = Path(path)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def fresh(path):
    path = Path(path).resolve()
    path.mkdir(parents=True, exist_ok=False)
    return path


def verify_source(directory, files):
    """Also prove the bytes of file symlinks flattened by artifact transport."""
    directory = Path(directory).resolve()
    flattened = []
    for name, expected in files.items():
        path = directory / name
        require(path.resolve().is_relative_to(directory), f"Source escapes its snapshot: {name}")
        if "symlink" in expected:
            if path.is_symlink():
                require(str(path.readlink()) == expected["symlink"], f"Source link changed: {name}")
                continue
            target = (path.parent / expected["symlink"]).resolve()
            require(target.is_relative_to(directory), f"Unsupported external source link: {name}")
            target_name = str(target.relative_to(directory))
            target_expected = files.get(target_name, {})
            require("sha256" in target_expected, f"Cannot prove a flattened source link: {name}")
            actual = identity(path)
            require(all(actual[k] == target_expected[k] for k in ("sha256", "bytes")), f"Flattened link bytes differ: {name}")
            flattened.append({"path": name, "original_target": expected["symlink"], "content_target": target_name})
        else:
            actual = identity(path)
            require(all(actual[k] == expected[k] for k in ("sha256", "bytes")), f"Source bytes changed: {name}")
    return flattened


def verify_graph_names(directory, files):
    require({path.name for path in Path(directory).iterdir()} == set(files) | {"hashes.json"},
            "Dependency directory differs from its exact preserved inventory")


def import_evidence(args):
    evidence = args.evidence.resolve(strict=True)
    output = fresh(args.output)
    require(not output.is_relative_to(evidence), "Keep derived imports outside the original evidence")
    comparison = read(evidence / "comparison.json")
    require(comparison.get("complete") and all(comparison["arms"][a].get("complete") for a in ARMS),
            "Require both completed CI release arms")
    old_root = Path(comparison["arms"]["reference"]["manifests"]["build"]["path"]).parents[2]
    report = {"schema_version": 1, "complete": False, "producer": identity(__file__),
              "evidence_root": str(evidence), "original_evidence_root": str(old_root),
              "observed_comparison": identity(evidence / "comparison.json"),
              "relocations": {}, "arms": {}, "original_manifests_modified": False}
    destination = output / "import.json"
    save(destination, report)

    def located(old):
        relative = Path(old).relative_to(old_root)
        result = (evidence / relative).resolve(strict=True)
        require(result.is_relative_to(evidence), f"Artifact escapes evidence: {old}")
        return result

    def checked(expected):
        old = expected["path"]
        actual = identity(located(old))
        require(all(actual[k] == expected[k] for k in ("sha256", "bytes") if k in expected), f"Artifact changed: {old}")
        record = {"original": expected, "local": actual}
        previous = report["relocations"].get(old)
        if previous is not None:
            require(previous["local"] == actual and all(previous["original"].get(k, v) == v for k, v in expected.items()),
                    f"Conflicting artifact identity: {old}")
            record["original"] = {**previous["original"], **expected}
        report["relocations"][old] = record
        return actual

    def graph(old_directory, expected_inventory):
        directory = located(old_directory)
        inventory = checked({"path": str(Path(old_directory) / "hashes.json"), "sha256": expected_inventory})
        files = read(inventory["path"])
        verify_graph_names(directory, files)
        for name, sha in files.items():
            require(Path(name).name == name, "Dependency inventory must use basenames")
            checked({"path": str(Path(old_directory) / name), "sha256": sha})
        return {"path": str(directory), "inventory": inventory, "files": files}

    try:
        report["toolchain"] = checked(comparison["toolchain"])
        for arm in ARMS:
            ci_arm = comparison["arms"][arm]
            original = {"build": ci_arm["manifests"]["build"]}
            current = {"build": checked(original["build"])}
            build = read(current["build"]["path"])
            require(build.get("complete") and len(build["training_runs"]) == 13, "Incomplete production PGO build")
            require(ci_arm["binary"] == build["binary"] and ci_arm["profile"] == build["profile"]
                    and ci_arm["graph"] == build["graph"] and ci_arm["manifests"]["base"] == build["base_manifest"],
                    "CI comparison refers to another production build")
            for key, value in (("base_manifest", build["base_manifest"]), ("source_manifest", build["source"]),
                               ("profile", build["profile"]), ("shipping", build["binary"]),
                               ("source_archive", build["source_archive"]), ("artifacts", ci_arm["manifests"]["artifacts"]),
                               ("archives", ci_arm["manifests"]["archives"])):
                original[key] = value
                current[key] = checked(value)
            base = read(current["base_manifest"]["path"])
            source = read(current["source_manifest"]["path"])
            artifacts = read(current["artifacts"]["path"])
            require(artifacts.get("complete") and artifacts["shipping"] == build["binary"], "Wrong or incomplete native collection")
            require(base["source"] == build["source"] and base["profile"] == build["profile"] and base["graph"] == build["graph"],
                    "Source, production graph and fresh profile are not associated")
            require(base["phases"]["build"]["exit"] == 0 and base["compiler_record"] == base["compiler_record_identity"]["path"],
                    "Base manifest lacks its completed compiler association")
            require(source["head"] == ci_arm["revision"] == comparison["resolved_revisions"][arm], "Revision association changed")
            require(Path(build["profile"]["path"]).is_relative_to(Path(original["build"]["path"]).parent),
                    "Each arm must own its fresh profile")
            for key, value in (("base", base["base"]), ("compiler_record", base["compiler_record_identity"])):
                original[key] = value
                current[key] = checked(value)
            compiler = read(current["compiler_record"]["path"])
            require(compiler.get("returncode") == 0 and compiler["target"] == HOST and compiler["crate"] == "ferroui_base",
                    "Require actual successful target Base compiler record")
            flags = [compiler["argv"][i + 1] for i, token in enumerate(compiler["argv"][:-1]) if token == "-C"]
            flags += [token[2:] for token in compiler["argv"] if token.startswith("-C") and len(token) > 2]
            profiles = [flag.split("=", 1)[1] for flag in flags if flag.startswith("profile-use=")]
            require(profiles and set(profiles) == {original["profile"]["path"]}, "Base compiler did not use the recorded profile")
            current["graph"] = graph(base["graph"]["path"], base["graph"]["inventory_sha256"])
            current["host_dependencies"] = graph(base["host_dependencies"], base["host_inventory_sha256"])
            require(Path(base["base"]["path"]).parent == Path(base["graph"]["path"])
                    and current["graph"]["files"].get(Path(base["base"]["path"]).name) == base["base"]["sha256"],
                    "Selected Base is not part of its preserved production graph")
            externs = [compiler["argv"][i + 1] for i, token in enumerate(compiler["argv"][:-1]) if token == "--extern"]
            selected = [x.split("=", 1)[1] for x in externs if x.startswith("bitflags=")]
            require(len(selected) == 1, "Select bitflags from the actual Base compiler, not a glob")
            bitflags = Path(selected[0]).with_suffix(".rlib").name
            require(bitflags in current["graph"]["files"], "Base's exact bitflags dependency is missing")
            current["bitflags"] = identity(Path(current["graph"]["path"]) / bitflags)
            current["bitflags_original_extern"] = selected[0]
            current["source_directory"] = str(located(source["directory"]))
            current["flattened_source_links"] = verify_source(current["source_directory"], source["files"])
            require(Path(base["source_directory"]) == Path(source["directory"]) / "src/FerroUI.Base", "Wrong frozen Base source")
            archive_view = {"schema_version": 1, "derived_view_only": True, "original_manifest": current["archives"], "cases": {}}
            for label, case in read(current["archives"]["path"])["cases"].items():
                archive_view["cases"][label] = {"font": case["font"], "archive": checked(case["archive"]) if case["archive"] else None}
            require(len(archive_view["cases"]) == 9, "Require the complete nine-case archive matrix")
            view = output / (arm + "-relocated-archives.json")
            save(view, archive_view)
            current.update(original=original, revision=source["head"], archives_view=identity(view))
            report["arms"][arm] = current
            save(destination, report)
        report["complete"] = True
    except BaseException as error:
        report["error"] = {"type": type(error).__name__, "message": str(error)}
        raise
    finally:
        save(destination, report)
    print(destination)


def verify_import(path):
    imported = read(path)
    require(imported.get("complete"), "Import did not complete")
    require(identity(imported["observed_comparison"]["path"]) == imported["observed_comparison"], "CI comparison changed")
    for record in imported["relocations"].values():
        require(identity(record["local"]["path"]) == record["local"], "Imported artifact changed")
    for arm in imported["arms"].values():
        for name in ("graph", "host_dependencies"):
            verify_graph_names(arm[name]["path"], arm[name]["files"])
        verify_source(arm["source_directory"], read(arm["source_manifest"]["path"])["files"])
        require(identity(arm["archives_view"]["path"]) == arm["archives_view"], "Relocated archive view changed")
    return imported


class RuntimeComparison:
    def __init__(self, args):
        self.args = args
        self.work = fresh(args.output)
        (self.work / "logs").mkdir()
        (self.work / "tmp").mkdir()
        self.report = {"schema_version": 1, "complete": False, "scope": __doc__, "commands": [], "pending": [],
                       "import": identity(args.import_manifest), "cpu": args.cpu, "pairs": 31,
                       "full_release_accepted": False, "producer": identity(__file__),
                       "companion_scope": "Imported CI production graph; fresh local links under the existing producer policies.",
                       "startup_scope": "Unchanged CI-linked executables executed locally, separately qualified for host compatibility."}
        self.env = {k: v for k, v in os.environ.items() if k not in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_BOOTSTRAP", "LLVM_PROFILE_FILE", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "FERROUI_CC", "LD_PRELOAD", "LD_AUDIT"}
                    and not k.startswith(("FERROUI_PGO_", "FERROUI_BENCH_EXPECT_", "FERROUI_TEMPLATE_", "CARGO_PROFILE_"))}
        self.report["removed_environment_names"] = sorted(set(os.environ) - set(self.env))
        prefix = args.toolchain_prefix.resolve(strict=True)
        self.rustc = prefix / "bin/rustc"
        self.env.update(PATH=str(prefix / "bin") + os.pathsep + str(prefix / "lib/rustlib" / HOST / "bin/gcc-ld") + os.pathsep + self.env.get("PATH", os.defpath),
                        LD_LIBRARY_PATH=os.pathsep.join([str(prefix / "lib"), str(prefix / "lib/rustlib" / HOST / "lib")]),
                        TMPDIR=str(self.work / "tmp"), LLVM_PROFILE_FILE=os.devnull, RAYON_NUM_THREADS="1", MALLOC_ARENA_MAX="1")
        self.report["environment"] = {k: self.env[k] for k in ("PATH", "LD_LIBRARY_PATH", "TMPDIR", "LLVM_PROFILE_FILE", "RAYON_NUM_THREADS", "MALLOC_ARENA_MAX")}
        self.persist()

    def persist(self):
        save(self.work / "runtime-comparison.json", self.report)

    def run(self, label, command):
        row = {"label": label, "command": list(map(str, command)), "complete": False}
        self.report["commands"].append(row)
        self.persist()
        print("START", label, flush=True)
        with (self.work / "logs" / (label + ".stdout")).open("w") as stdout, (self.work / "logs" / (label + ".stderr")).open("w") as stderr:
            process = subprocess.Popen(row["command"], env=self.env, stdout=stdout, stderr=stderr, start_new_session=True)
            try:
                row["exit"] = process.wait()
            except BaseException:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
                raise
            finally:
                row.update(stdout=identity(stdout.name), stderr=identity(stderr.name))
                self.persist()
        require(row["exit"] == 0, f"{label} failed; retained raw logs")
        row["complete"] = True
        self.persist()
        return row

    def producer(self, name, *arguments):
        return [sys.executable, HERE / name, *arguments]

    def verify_integrity(self):
        require(identity(self.args.import_manifest) == self.report["import"], "Import manifest changed during the run")
        verify_import(self.args.import_manifest)
        for expected in self.report["producer_inputs"].values():
            require(identity(expected["path"]) == expected, "Validation producer changed during the run")

    def execute(self):
        from runtime_artifact_host import verify_native, verify_toolchain
        imported = verify_import(self.args.import_manifest)
        self.report["producer_inputs"] = {name: identity(HERE / name) for name in (
            "run_winui_runtime_comparison.py", "runtime_artifact_host.py", "prepare_property_validation.py",
            "property_validation_cold.rs", "property_allocation_counter.rs", "prepare_callable_validation.py",
            "shared_receiver_workloads.rs", "animation_attachment_allocations.rs", "actual_getters.rs", "keyframe_probe.rs",
            "compare_registration.py", "compact_unwind_link.py", "compact_unwind.py", "attribute_codegen.py",
            "attribute_runtime.py", "validate_runtime_release.py", "measure.py")}
        if self.args.pgo_audit_script.is_file():
            self.report["pgo_audit_producer"] = identity(self.args.pgo_audit_script)
            self.report["producer_inputs"]["pgo_acceptance_audit"] = self.report["pgo_audit_producer"]
        self.arms = imported["arms"]
        builds = {arm: read(row["build"]["path"]) for arm, row in self.arms.items()}
        proof = verify_toolchain(read(imported["toolchain"]["path"]), builds, self.args.toolchain_prefix,
                                 self.args.toolchain_manifest, self.work / "toolchain-proof", self.env)
        self.report["toolchain_proof"] = proof
        macros = [Path(row["host_dependencies"]["path"]) / name for row in self.arms.values()
                  for name in row["host_dependencies"]["files"] if name.endswith(".so")]
        native = verify_native(macros, self.work / "macro-host-proof", self.env)
        self.report["macro_host_proof"] = native
        if not proof.get("complete") or not native.get("complete"):
            self.report["pending"].append("Compiler/sysroot or imported proc-macro host compatibility is not proven; no companion was linked.")
            self.persist()
            return
        if not self.args.checks_only and not self.args.pgo_audit_script.is_file():
            self.report["pending"].append("The required bounded PGO audit script is unavailable; no companion link or timing was started.")
            self.persist()
            return
        os.sched_setaffinity(0, {self.args.cpu})
        candidate_source = self.arms["candidate"]["source_directory"]
        fixtures = self.work / "fixtures"
        fixtures.mkdir()
        self.run("property-materialize", self.producer("prepare_property_validation.py", "materialize", "--source-root", candidate_source, "--work-dir", fixtures / "property"))
        for arm, policy in (("reference", "cloned"), ("candidate", "borrowed")):
            self.run(arm + "-callable-materialize", self.producer("prepare_callable_validation.py", "materialize", "--source-root", candidate_source,
                     "--receiver-policy", policy, "--output", fixtures / arm))
        # All links finish before checks, counters, host smoke, or paired timings.
        for arm, row in self.arms.items():
            common = ["--mode", "production", "--base", row["base"]["path"], "--profile", row["profile"]["path"],
                      "--dependency", row["graph"]["path"], "--dependency", row["host_dependencies"]["path"], "--rustc", self.rustc]
            for kind in PROPERTY:
                self.run(arm + "-property-build-" + kind, self.producer("prepare_property_validation.py", "build", *common, "--fixture", fixtures / "property/fixture.json",
                         "--work-dir", self.work / arm / "property", "--kind", kind, "--bitflags", row["bitflags"]["path"], "--cpu", self.args.cpu))
            for kind in CALLABLE:
                self.run(arm + "-callable-build-" + kind, self.producer("prepare_callable_validation.py", "build", *common, "--fixture", fixtures / arm / "fixture.json",
                         "--kind", kind, "--source-root", candidate_source, "--output", self.work / arm / kind))
        self.report["all_production_links_completed"] = True
        for kind in CALLABLE:
            pair = [read(self.work / arm / kind / "build.json") for arm in ARMS]
            require(all(row["mode"] == "production" and row["compact_unwind_applied"] for row in pair),
                    "Callable production link did not retain its compact-unwind policy")
            require(pair[0]["native_linker"] == pair[1]["native_linker"], "Callable arms used different local native linkers")
        self.persist()
        allocation = {}
        for arm in ARMS:
            directory = self.work / arm
            for suite, kind in (("correctness", "qa"), ("regressions", "qa"), ("allocations", "allocations")):
                self.run(arm + "-property-check-" + suite, self.producer("prepare_property_validation.py", "check", "--build", directory / "property" / kind / "build.json",
                         "--suite", suite, "--work-dir", directory / ("property-" + suite)))
                checked = read(directory / ("property-" + suite) / "check.json")
                require(checked.get("complete"), "Incomplete property gate")
                if suite == "correctness":
                    require(checked["runs"][0]["summary"] == {"passed": 347, "failed": 0, "ignored": 3}, "Production property coverage changed")
                elif suite == "regressions":
                    require(len(checked["runs"]) == 9, "Nine focused regression checks are required")
            for kind in CALLABLE:
                self.run(arm + "-callable-check-" + kind, self.producer("prepare_callable_validation.py", "check", "--build", directory / kind / "build.json", "--out", directory / kind / "check.json"))
                require(read(directory / kind / "check.json").get("complete"), "Incomplete callable gate")
            mixed = read(directory / "property/mixed/build.json")
            ran = self.run(arm + "-mixed-check", [mixed["executable"]["path"], "0"])
            spec = importlib.util.spec_from_file_location("existing_property_gate", HERE / "prepare_property_validation.py")
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            module.timing_rows("mixed", Path(ran["stdout"]["path"]).read_text(), read(fixtures / "property/fixture.json")["expectations"]["mixed"])
            properties = read(directory / "property-allocations/check.json")
            allocation[arm] = {"property": {lane: data for run in properties["runs"] for lane, data in run["allocation_counts"].items()},
                               "animation": read(directory / "animation-allocations/check.json")["results"][0]["samples"]}
        self.report["production_correctness_passed"] = True
        self.report["allocation_comparison"] = {"arms": allocation, "equal": allocation["reference"] == allocation["candidate"],
                                                "scope": "Existing separate counters; requested allocations/bytes, not live memory or timing."}
        self.verify_integrity()
        self.persist()
        if self.args.checks_only:
            self.report["pending"].append("Paired measurements were intentionally deferred by --checks-only.")
            return
        self.run("pgo-acceptance", [sys.executable, self.args.pgo_audit_script, "--import-manifest", self.args.import_manifest,
                 "--toolchain-prefix", self.args.toolchain_prefix, "--cpu", self.args.cpu, "--output", self.work / "pgo-acceptance"])
        audit = read(self.work / "pgo-acceptance/result.json")
        require(audit.get("complete") and audit.get("accepted"), "PGO acceptance is unresolved; do not time")
        self.report["pgo_acceptance"] = identity(self.work / "pgo-acceptance/result.json")
        startup = self.work / "startup"
        startup.mkdir()
        shipping = {}
        for arm, row in self.arms.items():
            path = startup / arm
            shutil.copy2(row["shipping"]["path"], path)
            path.chmod(0o755)  # Artifact transport may remove executable permission; originals stay untouched.
            require(identity(path)["sha256"] == row["shipping"]["sha256"], "Shipping copy changed bytes")
            shipping[arm] = path
        host = verify_native(list(shipping.values()), startup / "host-proof", self.env)
        self.report["shipping_host_proof"] = host
        if host.get("complete"):
            reference_archives = read(self.arms["reference"]["archives_view"]["path"])
            candidate_archives = read(self.arms["candidate"]["archives_view"]["path"])
            for name, case in candidate_archives["cases"].items():
                other = reference_archives["cases"][name]
                require(case["font"] == other["font"] and (case["archive"] or {}).get("sha256") == (other["archive"] or {}).get("sha256"), "Startup archive cohorts differ")
            for arm, binary in shipping.items():
                self.run(arm + "-shipping-compatibility", self.producer("validate_runtime_release.py", "compat", "--executable", binary,
                         "--archives", self.arms[arm]["archives_view"]["path"], "--output", startup / (arm + "-compatibility.json")))
        else:
            self.report["pending"].append("Unchanged CI shipping binaries lack local host compatibility proof; startup remains pending.")
        # No compiler command is reachable after this point.
        measurements = self.work / "measurements"
        measurements.mkdir()
        self.report["measurement_results"] = {}
        for suite in ("bindings", "cold", "setters", "mixed"):
            kind = "mixed" if suite == "mixed" else "qa"
            self.run("property-measure-" + suite, self.producer("prepare_property_validation.py", "measure", "--reference-build", self.work / "reference/property" / kind / "build.json",
                     "--candidate-build", self.work / "candidate/property" / kind / "build.json", "--suite", suite, "--cpu", self.args.cpu,
                     "--pairs", 31, "--warmups", 3, "--work-dir", measurements / suite))
            result = measurements / suite / "measurement.json"
            require(read(result).get("complete"), "Incomplete property timing cohort")
            self.report["measurement_results"]["property-" + suite] = identity(result)
            self.persist()
        for kind in ("receivers", "getters", "keyframes"):
            self.run("callable-measure-" + kind, self.producer("prepare_callable_validation.py", "measure", "--reference", self.work / "reference" / kind / "build.json",
                     "--candidate", self.work / "candidate" / kind / "build.json", "--cpu", self.args.cpu, "--pairs", 31, "--out", measurements / (kind + ".json")))
            result = measurements / (kind + ".json")
            require(read(result).get("complete"), "Incomplete callable timing cohort")
            self.report["measurement_results"][kind] = identity(result)
            self.persist()
        if host.get("complete"):
            self.run("startup-measure", self.producer("validate_runtime_release.py", "startup", "--reference", shipping["reference"], "--candidate", shipping["candidate"],
                     "--archives", self.arms["candidate"]["archives_view"]["path"], "--cpu", self.args.cpu, "--output", measurements / "startup.json"))
            require(read(measurements / "startup.json").get("complete"), "Incomplete startup cohort")
            self.report["measurement_results"]["startup"] = identity(measurements / "startup.json")
        self.verify_integrity()
        self.report["paired_companion_measurements_completed"] = True
        self.report["complete"] = not self.report["pending"]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    commands = parser.add_subparsers(dest="command", required=True)
    prepare = commands.add_parser("import", help="verify and map completed downloaded evidence without compiling")
    prepare.add_argument("--evidence", type=Path, required=True)
    prepare.add_argument("--output", type=Path, required=True)
    run = commands.add_parser("run", help="link/check both real production graphs, then execute existing paired cohorts")
    run.add_argument("--import-manifest", type=Path, required=True)
    run.add_argument("--toolchain-prefix", type=Path, required=True)
    run.add_argument("--toolchain-manifest", type=Path, required=True)
    run.add_argument("--output", type=Path, required=True)
    run.add_argument("--cpu", type=int, required=True)
    run.add_argument("--checks-only", action="store_true")
    run.add_argument("--pgo-audit-script", type=Path, default=HERE / "verify_winui_profile_acceptance.py")
    args = parser.parse_args()
    if args.command == "import":
        import_evidence(args)
        return
    require(sys.platform == "linux" and args.cpu in os.sched_getaffinity(0), "Require an allowed Linux CPU")
    original_affinity = os.sched_getaffinity(0)
    comparison = RuntimeComparison(args)
    def terminated(signum, _frame):
        raise InterruptedError(f"Received signal {signum}")
    signal.signal(signal.SIGTERM, terminated)
    try:
        comparison.execute()
    except BaseException as error:
        comparison.report["error"] = {"type": type(error).__name__, "message": str(error)}
        raise
    finally:
        os.sched_setaffinity(0, original_affinity)
        comparison.persist()
        print(json.dumps({k: comparison.report.get(k) for k in ("complete", "production_correctness_passed", "paired_companion_measurements_completed", "pending", "error", "full_release_accepted")}), flush=True)
    if not comparison.report["complete"] and not (args.checks_only and comparison.report.get("production_correctness_passed")):
        raise SystemExit(2)


if __name__ == "__main__":
    main()
