#!/usr/bin/env python3
"""Compare two complete editable WinUI releases in one disposable compile path.

Each revision trains its own original thirteen-workload PGO profile. Collection
and archive/template correctness finish before the checkout or target is reused.
This orchestrator measures shipping bytes, not throughput or build performance.
Compiler identities, profiles, graphs, commands and failures retain their existing
producer manifests under --work-dir/evidence. No runtime source is overlaid.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import tomllib


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
HOST = "x86_64-unknown-linux-gnu"
CANDIDATE = "d2d2f83d606f547b4ebda2323a145b14ca8379ad"
REFERENCE = "be91062df9bb018de78113c34a1967c60038f9b6"
# Retained production-codec outputs, independently reproduced by the isolated
# tool. These guard the original training inputs before any application build.
CANONICAL = {
    "scalar-v3": (296352, "3e209b649ca465fed259065111c90defe2823d282fe581ad89f20e140b8a528f"),
    "two-edits-v3": (296359, "ac7712e2a7611f71432a7aaf6dd2f61c5ffa771a3574ca6834722245933e5cf0"),
    "installed": (296347, "70e56b58375dfc7d31d4304b4ec0da69f515d41c937187ba9c035fde84d9a7fb"),
}
PRODUCERS = (
    "run_winui_release_comparison.py", "bootstrap_toolchain.py", "rebuild_winui_pgo.py",
    "collect_winui_pgo.py", "prepare_portable_packer.py", "validate_runtime_release.py",
    "prepare_template_validation.py", "template_structural_resource.rs", "profile_winui.py",
    "winui.rs", "pgo_templates.rs", "compact_unwind.py", "compact_unwind_link.py",
    "attribute_codegen.py", "attribute_runtime.py", "compare_registration.py", "measure.py",
)


def identity(path: Path) -> dict:
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path.resolve()), "bytes": path.stat().st_size, "sha256": digest}


def read(path: Path) -> dict:
    return json.loads(path.read_text())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def external_lock(path: Path) -> list:
    return sorted((item["name"], item["version"], item["source"], item.get("checksum", ""))
                  for item in tomllib.loads(path.read_text())["package"] if item.get("source"))


class Comparison:
    def __init__(self, args):
        self.args = args
        self.repository = args.repository.resolve(strict=True)
        self.work = args.work_dir.resolve()
        require(not self.work.exists(), "Use a new work directory; previous results are never overwritten")
        require(not self.work.is_relative_to(self.repository), "Place disposable work outside the tooling checkout")
        self.evidence = self.work / "evidence"
        (self.evidence / "logs").mkdir(parents=True)
        self.checkout = self.work / "checkout"
        self.shared = self.work / "shared"
        self.shared.mkdir()
        self.report = {
            "schema_version": 1, "scope": __doc__, "complete": False, "started_unix": time.time(),
            "commands": [], "arms": {}, "cpu_affinity": args.cpus,
            "compile_source_path": str(self.checkout), "consumer_path": str(self.shared / "consumer"),
            "target_path": str(self.shared / "target"), "shipping_budget_bytes": 10_000_000,
            "performance": "Not measured: no throughput, allocation, startup or build-time parity claim.",
            "release_acceptance": "Pending the full property, binding, metadata and animation correctness/performance gates; this size and targeted correctness comparison is not full release acceptance.",
            "requested_revisions": {"reference": args.reference, "candidate": args.candidate},
            "order": ["reference", "candidate"],
        }
        self.environment = os.environ.copy()
        removed = []
        for key in list(self.environment):
            if key in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "LLVM_PROFILE_FILE", "RUSTC",
                       "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTC_BOOTSTRAP"} or key.startswith(
                           ("CARGO_PROFILE_", "FERROUI_PGO_", "FERROUI_BENCH_EXPECT_")):
                removed.append(key)
                del self.environment[key]
        temporary = self.work / "temporary"
        temporary.mkdir()
        self.settings = {"TMPDIR": str(temporary), "LLVM_PROFILE_FILE": os.devnull,
                         "CARGO_HOME": str(self.work / "cargo-home"), "CARGO_INCREMENTAL": "0",
                         "CARGO_BUILD_JOBS": "1", "RAYON_NUM_THREADS": "1", "MALLOC_ARENA_MAX": "1",
                         "CARGO_TERM_COLOR": "never"}
        self.environment.update(self.settings)
        self.report["environment"] = {"removed_variable_names": sorted(removed), "controlled": self.settings}
        self.persist()

    def persist(self):
        destination = self.evidence / "comparison.json"
        temporary = destination.with_suffix(".tmp")
        temporary.write_text(json.dumps(self.report, indent=2, sort_keys=True) + "\n")
        temporary.replace(destination)

    def run(self, label, command, cwd=None, extra=None, stop_file=None):
        log = self.evidence / "logs" / (label + ".log")
        require(not log.exists(), f"Duplicate command label: {label}")
        row = {"label": label, "argv": list(map(str, command)), "cwd": str(cwd or self.work),
               "log": str(log), "started_unix": time.time(), "complete": False,
               "extra_environment": extra or {}}
        self.report["commands"].append(row)
        self.persist()
        print(f"START {label}", flush=True)
        process = None
        try:
            with log.open("w") as output:
                process = subprocess.Popen(row["argv"], cwd=row["cwd"], env={**self.environment, **(extra or {})},
                                           stdout=output, stderr=subprocess.STDOUT, start_new_session=True)
                row["exit"] = process.wait()
            require(row["exit"] == 0, f"{label} exited {row['exit']}; inspect {log}")
            row["complete"] = True
        except BaseException:
            # The replay supervises Cargo in a separate process group. Ask its
            # existing watchdog to stop that group before stopping the replay.
            if process is not None and process.poll() is None and stop_file is not None and stop_file.parent.is_dir():
                stop_file.write_text("Comparison supervisor interrupted.\n")
                try:
                    process.wait(timeout=30)
                except subprocess.TimeoutExpired:
                    pass
            if process is not None and process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
            if process is not None:
                row["exit"] = process.returncode
            raise
        finally:
            row["finished_unix"] = time.time()
            if log.exists():
                row["output"] = identity(log)
            self.persist()
            print(f"END {label}: exit={row.get('exit')} complete={row['complete']}", flush=True)

    def producer(self, name, *arguments):
        return [sys.executable, HERE / name, *arguments]

    def canonical(self, archives, labels):
        for label in labels:
            recorded = archives["cases"][label]["archive"]
            actual = identity(Path(recorded["path"]))
            require(actual == recorded and (actual["bytes"], actual["sha256"]) == CANONICAL[label],
                    f"{label} does not reproduce the retained production-codec archive")

    def packer(self, arm, directory):
        package = directory / "packer-source"
        self.run(arm + "-packer-source", self.producer("prepare_portable_packer.py", "materialize",
                 "--source", self.checkout, "--output", package))
        cargo = self.prefix / "bin/cargo"
        # Cargo retains the copied version seed while pruning to the small tool
        # graph. The verifier rejects any external tuple absent from that seed.
        policy = {"CARGO_TARGET_DIR": str(self.shared / "packer-target"),
                  "CARGO_PROFILE_RELEASE_OPT_LEVEL": "s", "CARGO_PROFILE_RELEASE_LTO": "thin",
                  "CARGO_PROFILE_RELEASE_CODEGEN_UNITS": "16", "CARGO_PROFILE_RELEASE_DEBUG": "0",
                  "CARGO_PROFILE_RELEASE_PANIC": "unwind"}
        self.run(arm + "-packer-fetch", [cargo, "fetch", "--target", HOST], package, policy)
        for stage in ("before", "after"):
            if stage == "after":
                self.run(arm + "-packer-build", [cargo, "build", "--locked", "--release", "--target", HOST,
                         "--bin", "ferroui-fxd"], package, {**policy, "CARGO_NET_OFFLINE": "true"})
            self.run(arm + "-packer-lock-" + stage, self.producer("prepare_portable_packer.py", "verify-lock",
                     "--manifest", package / "materialization.json", "--out", directory / ("packer-lock-" + stage + ".json")))
        original = self.shared / "packer-target" / HOST / "release/ferroui-fxd"
        destination = directory / "ferroui-fxd"
        shutil.copy2(original, destination)
        require(identity(original)["sha256"] == identity(destination)["sha256"], "Packer copy changed bytes")
        self.run(arm + "-packer-archives", self.producer("validate_runtime_release.py", "--source", self.checkout,
                 "archives", "--packer", destination, "--output", directory / "packer-check"))
        self.canonical(read(directory / "packer-check/archives.json"), ("scalar-v3", "two-edits-v3"))
        return destination

    def arm(self, name, revision):
        directory = self.evidence / name
        directory.mkdir()
        row = {"revision": revision, "complete": False}
        self.report["arms"][name] = row
        self.persist()
        self.run(name + "-checkout", ["git", "checkout", "--detach", revision], self.checkout)
        packer = self.packer(name, directory)
        row["packer"] = identity(packer)
        release = directory / "release"
        self.run(name + "-release", self.producer("rebuild_winui_pgo.py", "--source", self.checkout,
                 "--work-dir", release, "--shared-work", self.shared, "--toolchain-prefix", self.prefix,
                 "--cargo-home", self.work / "cargo-home", "--packer", packer,
                 "--cpus", ",".join(map(str, self.args.cpus)), "--min-free-mib", str(self.args.min_free_mib),
                 "--min-available-mib", str(self.args.min_available_mib)), stop_file=release / "STOP")
        build = read(release / "build.json")
        require(build.get("complete"), "Release producer did not complete")
        source = read(Path(build["source"]["path"]))
        require(source["head"] == revision and source["compile_directory"] == str(self.checkout), "Wrong source revision/path")
        require(build["controlled_environment"]["CARGO_TARGET_DIR"] == str(self.shared / "target"), "Target path changed")
        require(len(build["training_runs"]) == 13, "Original PGO training requires thirteen launches")
        for lane, label in (("scalar", "scalar-v3"), ("structure", "two-edits-v3")):
            archive = build["training_archives"][lane]
            require((archive["bytes"], archive["sha256"]) == CANONICAL[label], "Training archive differs from packer proof")
        self.run(name + "-collect", self.producer("collect_winui_pgo.py", "--build", release / "build.json", "--attribute"))
        artifacts = read(release / "artifacts.json")
        require(artifacts.get("complete"), "Generated/native collection did not complete")
        installed, = Path(artifacts["generated"]["directory"]).rglob("*.fxp")
        archives = directory / "archives"
        self.run(name + "-archives", self.producer("validate_runtime_release.py", "--source", source["directory"],
                 "archives", "--packer", packer, "--installed", installed, "--output", archives))
        archive_manifest = read(archives / "archives.json")
        self.canonical(archive_manifest, tuple(CANONICAL))
        self.run(name + "-compatibility", self.producer("validate_runtime_release.py", "compat", "--executable",
                 build["binary"]["path"], "--archives", archives / "archives.json", "--output", directory / "compatibility.json"))
        compatibility = read(directory / "compatibility.json")
        require(compatibility.get("complete") and len(compatibility["cases"]) == 9, "Nine archive cases are required")
        base = read(Path(build["base_manifest"]["path"]))
        require(base["profile"] == build["profile"] and base["graph"] == build["graph"], "Library graph/profile mismatch")
        for gate in ("registry", "unwind"):
            command = self.producer("validate_runtime_release.py", "--source", source["directory"], gate,
                "--base", base["base"]["path"], "--dependency", base["graph"]["path"],
                "--dependency", base["host_dependencies"], "--profile", base["profile"]["path"],
                "--rustc", self.prefix / "bin/rustc", "--output", directory / gate)
            if gate == "registry":
                # This assertion-enabled correctness companion consumes the
                # production bitcode graph; it is not a shipping/timing build.
                command += ["--dev-lto", "thin", "--dev-opt-level", "1"]
            self.run(name + "-" + gate, command)
            result = read(directory / gate / "result.json")
            require(result.get("complete") and result["run_exit"] == 0, f"{gate} gate did not complete")
            if gate == "registry":
                require(result["counts"] == {"passed": 47, "failed": 0, "ignored": 0}, "Registry coverage changed")
            else:
                require(".cfi_reserve" in result["sections"], "Unwind companion did not use compact unwind")
        self.run(name + "-templates-build", self.producer("prepare_template_validation.py", "build", "--fixture",
                 self.evidence / "template-fixture/fixture.json", "--graph", base["graph"]["path"],
                 "--dependency", base["host_dependencies"], "--profile", base["profile"]["path"],
                 "--rustc", self.prefix / "bin/rustc", "--cpu", str(self.args.cpus[0]), "--output", directory / "templates-build"))
        self.run(name + "-templates-check", self.producer("prepare_template_validation.py", "check", "--build",
                 directory / "templates-build/build.json", "--scalar-archive", archives / "scalar-v3.fxp",
                 "--structural-archive", archives / "struct-v3.fxp", "--output", directory / "templates-check"))
        templates = read(directory / "templates-check/check.json")
        require(templates.get("complete") and templates["total_activations"] == 156, "Template gate must pass all 156 activations")
        require(identity(Path(build["binary"]["path"])) == build["binary"], "Validation changed the shipping executable")
        row.update(binary=build["binary"], profile=build["profile"], graph=build["graph"], sections=artifacts["sections"],
                   archive_cases=9, registry_passed=47, compact_unwind_passed=True,
                   template_activations=156, additional_edit_probe=31,
                   manifests={label: identity(path) for label, path in {
                       "build": release / "build.json", "artifacts": release / "artifacts.json",
                       "base": Path(build["base_manifest"]["path"]), "archives": archives / "archives.json",
                       "compatibility": directory / "compatibility.json", "templates": directory / "templates-check/check.json",
                       "registry": directory / "registry/result.json", "unwind": directory / "unwind/result.json",
                   }.items()}, complete=True)
        self.persist()
        # Only the disposable compiler cache is removed. Both final graphs,
        # native link inputs, generated files, raw profiles and sources remain.
        shutil.rmtree(self.shared / "target")
        row["disposable_target_removed_after_collection_and_validation"] = True
        self.persist()
        return build

    def execute(self):
        os.sched_setaffinity(0, set(self.args.cpus))
        tooling = self.evidence / "tooling"
        tooling.mkdir()
        self.report["producer_inputs"] = {}
        for name in PRODUCERS:
            shutil.copy2(HERE / name, tooling / name)
            self.report["producer_inputs"][name] = identity(HERE / name)
        self.report["tooling_head"] = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        revisions = {label: subprocess.check_output(["git", "rev-parse", "--verify", "--end-of-options", ref + "^{commit}"],
                     cwd=self.repository, text=True).strip() for label, ref in self.report["requested_revisions"].items()}
        self.report["resolved_revisions"] = revisions
        self.persist()
        self.run("checkout-create", ["git", "clone", "--shared", "--no-checkout", self.repository, self.checkout])
        try:
            self.run("toolchain", self.producer("bootstrap_toolchain.py", "--work-dir", self.work / "toolchain", "--version", "1.99.0"))
        finally:
            # Keep distribution/installation diagnostics even if bootstrap
            # fails; downloaded official compiler payloads stay in the cache.
            diagnostics = self.evidence / "toolchain-diagnostics"
            diagnostics.mkdir()
            for pattern in ("*.json", "*.log", "*.sh", "dist/*.toml", "dist/*.sha256"):
                for path in (self.work / "toolchain").glob(pattern):
                    shutil.copy2(path, diagnostics / path.name)
        self.prefix = self.work / "toolchain/install"
        shutil.copy2(self.work / "toolchain/installation.json", self.evidence / "toolchain.json")
        self.report["toolchain"] = identity(self.evidence / "toolchain.json")
        self.environment.update(PATH=str(self.prefix / "bin") + os.pathsep + str(self.prefix / "lib/rustlib" / HOST / "bin/gcc-ld") +
                                os.pathsep + self.environment["PATH"], RUSTC=str(self.prefix / "bin/rustc"),
                                LD_LIBRARY_PATH=str(self.prefix / "lib") + (os.pathsep + self.environment["LD_LIBRARY_PATH"]
                                if self.environment.get("LD_LIBRARY_PATH") else ""))
        self.report["environment"]["toolchain"] = {key: self.environment[key] for key in ("PATH", "RUSTC", "LD_LIBRARY_PATH")}
        self.run("template-fixture", self.producer("prepare_template_validation.py", "materialize", "--source-root", ROOT,
                 "--output", self.evidence / "template-fixture"))
        builds = {label: self.arm(label, revisions[label]) for label in self.report["order"]}
        reference, candidate = builds["reference"], builds["candidate"]
        for key in ("tools", "rustc", "shipping_main_sha256", "profile_policy", "cpu_affinity"):
            require(reference[key] == candidate[key], f"Comparison policy differs: {key}")
        require({key: item["sha256"] for key, item in reference["training_sources"].items()} ==
                {key: item["sha256"] for key, item in candidate["training_sources"].items()}, "Training sources differ")
        require({key: item["sha256"] for key, item in reference["helper_inputs"].items()} ==
                {key: item["sha256"] for key, item in candidate["helper_inputs"].items()},
                "Frozen feature, linker, training or unwind helper policies differ")
        releases = [self.evidence / label / "release" for label in self.report["order"]]
        require((releases[0] / "consumer.Cargo.toml").read_bytes() == (releases[1] / "consumer.Cargo.toml").read_bytes(),
                "Consumer manifest/path differs between arms")
        require(external_lock(releases[0] / "consumer.Cargo.lock") == external_lock(releases[1] / "consumer.Cargo.lock"),
                "External dependency lock selections differ")
        for name, expected in self.report["producer_inputs"].items():
            require(identity(HERE / name) == expected, f"Producer changed during comparison: {name}")
        a, b = self.report["arms"]["reference"], self.report["arms"]["candidate"]
        size = b["binary"]["bytes"]
        self.report["comparison"] = {
            "reference_bytes": a["binary"]["bytes"], "candidate_bytes": size,
            "candidate_minus_reference_bytes": size - a["binary"]["bytes"],
            "remaining_to_budget_bytes": max(0, size - 10_000_000), "meets_10mb_target": size <= 10_000_000,
            "section_file_byte_deltas": {name: b["sections"].get(name, {}).get("file_bytes", 0) -
                a["sections"].get(name, {}).get("file_bytes", 0) for name in sorted(a["sections"].keys() | b["sections"].keys())},
        }
        self.report.update(complete=True, finished_unix=time.time())
        self.persist()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=ROOT, help="local tooling checkout with both revision objects")
    parser.add_argument("--work-dir", type=Path, required=True, help="new disposable directory outside the checkout")
    parser.add_argument("--candidate", default=CANDIDATE)
    parser.add_argument("--reference", default=REFERENCE)
    parser.add_argument("--cpus", default=str(min(os.sched_getaffinity(0))), help="allowed CPU numbers, comma separated")
    parser.add_argument("--min-free-mib", type=int, default=4096)
    parser.add_argument("--min-available-mib", type=int, default=512)
    args = parser.parse_args()
    args.cpus = sorted(set(map(int, args.cpus.split(","))))
    require(bool(args.cpus) and set(args.cpus) <= os.sched_getaffinity(0), "Choose allowed CPU numbers")
    require(args.min_free_mib > 0 and args.min_available_mib > 0, "Resource floors must be positive")
    comparison = Comparison(args)

    def terminate(signum, _frame):
        raise InterruptedError(f"Received signal {signum}")

    signal.signal(signal.SIGTERM, terminate)
    try:
        comparison.execute()
    except BaseException as error:
        comparison.report.update(error={"type": type(error).__name__, "message": str(error)}, finished_unix=time.time())
        comparison.persist()
        raise
    finally:
        report = comparison.report
        print(json.dumps({"complete": report["complete"], "evidence": str(comparison.evidence / "comparison.json"),
                          "arms": {name: {key: row[key] for key in ("revision", "complete", "binary", "archive_cases", "template_activations")
                                          if key in row} for name, row in report["arms"].items()},
                          "comparison": report.get("comparison"), "error": report.get("error")}, sort_keys=True), flush=True)


if __name__ == "__main__":
    main()
