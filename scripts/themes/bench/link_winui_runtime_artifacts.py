#!/usr/bin/env python3
"""Link production runtime companions from authenticated completed CI graphs.

This job never rebuilds a framework crate, trains PGO, or measures throughput.
The acceptance reference and candidate are newly linked together on one native
host. An additional correction-reference QA image uses the same property fixture.
The original individual producers, profiles, flags and test bodies are retained.
Only a bounded companion bundle is exported; original release archives stay separate.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import signal
import stat
import subprocess
import sys
import time
from types import SimpleNamespace
import urllib.error
import zipfile

import run_winui_runtime_comparison as runtime
import split_release_artifact as transfer

HERE = Path(__file__).resolve().parent
HOST = "x86_64-unknown-linux-gnu"
ROLES = ("reference", "candidate", "correction_reference")
MAX_BUNDLE = 448 * 1024 * 1024
MAX_EXPANDED_RELEASE = 16 * 1024**3
PROPERTIES = ("qa", "allocations", "mixed")
CALLABLES = ("receivers", "getters", "animation", "animation-allocations", "keyframes")


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def identity(path):
    path = Path(path).resolve(strict=True)
    with path.open("rb") as source:
        sha = hashlib.file_digest(source, "sha256").hexdigest()
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": sha}


def content(row):
    return row["bytes"], row["sha256"]


def read(path):
    return json.loads(Path(path).read_text())


def save(path, value):
    path = Path(path)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def safe_extract(archive, destination, limit):
    """Extract only bounded regular-file/directory ZIP entries, never links."""
    destination = Path(destination).resolve()
    destination.mkdir(parents=True, exist_ok=False)
    with zipfile.ZipFile(archive) as source:
        names, total = set(), 0
        for entry in source.infolist():
            path = PurePosixPath(entry.filename)
            require(entry.filename and "\\" not in entry.filename and not path.is_absolute()
                    and ".." not in path.parts and str(path) == entry.filename.rstrip("/"), "Unsafe ZIP member path")
            require(str(path) not in names, "Duplicate ZIP member")
            names.add(str(path))
            kind = stat.S_IFMT(entry.external_attr >> 16)
            require(kind in (0, stat.S_IFREG, stat.S_IFDIR) and not entry.flag_bits & 1,
                    "Encrypted or special ZIP member is unsupported")
            total += entry.file_size
            require(total <= limit and entry.file_size <= 2 * 1024**3, "ZIP exceeds its extraction budget")
        for entry in source.infolist():
            target = destination / entry.filename
            if entry.is_dir():
                target.mkdir(parents=True, exist_ok=True)
                continue
            target.parent.mkdir(parents=True, exist_ok=True)
            with source.open(entry) as incoming, target.open("xb") as output:
                shutil.copyfileobj(incoming, output, 1024 * 1024)
            require(target.stat().st_size == entry.file_size, "Extracted member size changed")
    return {"members": len(names), "expanded_bytes": total}


def validate_configuration(value):
    require(value.get("schema") == 1 and value.get("enabled") is True,
            "Remote link configuration is not explicitly enabled with frozen artifact metadata")
    require(set(value["origins"]) == {"acceptance", "correction"}, "Require the two explicitly named origins")
    require(set(value["selection"]) == set(ROLES), "Require reference, candidate and correction-reference selections")
    for name, origin in value["origins"].items():
        require(isinstance(origin, dict) and set(origin) == set(transfer.DEFAULTS), f"Incomplete frozen origin: {name}")
        require(origin["repository"] == "wieslawsoltes/FerroUI" and origin["repository_id"] == 1405576478,
                "Only the reviewed source repository is supported")
        require(re.fullmatch(r"[0-9a-f]{40}", origin["head_sha"]) and re.fullmatch(r"[0-9a-f]{64}", origin["sha256"]),
                "Origin requires full source and archive digests")
        require(all(isinstance(origin[key], int) and origin[key] > 0 for key in
                    ("repository_id", "run_id", "run_attempt", "artifact_id", "size")), "Invalid origin IDs/size")
        require(origin["size"] <= 4 * 1024**3, "Release ZIP exceeds the reviewed download bound")
    require(value["origins"]["acceptance"]["artifact_id"] != value["origins"]["correction"]["artifact_id"],
            "Acceptance and correction must be separate authenticated artifacts")
    for role, selected in value["selection"].items():
        require(set(selected) == {"origin", "arm", "revision", "receiver_policy"}, "Unknown selection field")
        require(selected["origin"] in value["origins"] and selected["arm"] in runtime.ARMS
                and re.fullmatch(r"[0-9a-f]{40}", selected["revision"]), "Invalid selected origin/arm/revision")
        require(selected["receiver_policy"] == ("cloned" if role == "reference" else "borrowed"),
                "This cohort requires cloned acceptance and borrowed corrected receiver policies")
    require(value["selection"]["reference"]["origin"] == "acceptance"
            and value["selection"]["candidate"]["origin"] == value["selection"]["correction_reference"]["origin"] == "correction"
            and value["selection"]["candidate"]["arm"] != value["selection"]["correction_reference"]["arm"],
            "Correction pair must select the two distinct arms of one origin")


def download_release(expected, output):
    """Reuse the reviewed no-Authorization-redirect transport, without splitting."""
    task = transfer.Transfer(SimpleNamespace(**expected, output=Path(output)))
    require(os.environ.get("GITHUB_REPOSITORY", "").lower() == expected["repository"].lower(), "Wrong workflow repository")
    task.token = os.environ.get("GITHUB_TOKEN", "")
    require(bool(task.token), "Read-only Actions token is required")
    try:
        prefix = "/repos/" + expected["repository"]
        repository = task.metadata(prefix, "repository")
        run = task.metadata(prefix + f'/actions/runs/{expected["run_id"]}', "run")
        artifact = task.metadata(prefix + f'/actions/artifacts/{expected["artifact_id"]}', "artifact")
        require(repository["id"] == expected["repository_id"] and repository["full_name"].lower() == expected["repository"].lower(),
                "Repository identity differs")
        require(run["id"] == expected["run_id"] and run["run_attempt"] == expected["run_attempt"]
                and run["status"] == "completed" and run["conclusion"] == "success", "Require the exact successful completed run")
        require(run["head_sha"] == expected["head_sha"] and run["head_branch"] == expected["head_branch"]
                and run["repository"]["id"] == run["head_repository"]["id"] == expected["repository_id"], "Run provenance differs")
        require(artifact["id"] == expected["artifact_id"] and artifact["name"] == expected["artifact_name"]
                and not artifact["expired"] and artifact["size_in_bytes"] == expected["size"]
                and artifact["digest"] == "sha256:" + expected["sha256"], "Artifact identity differs")
        origin = artifact["workflow_run"]
        require(origin["id"] == expected["run_id"] and origin["repository_id"] == origin["head_repository_id"] == expected["repository_id"]
                and origin["head_sha"] == expected["head_sha"] and origin["head_branch"] == expected["head_branch"], "Artifact origin differs")
        route = prefix + f'/actions/artifacts/{expected["artifact_id"]}/zip'
        require(artifact["archive_download_url"] == "https://api.github.com" + route, "Unexpected artifact endpoint")
        task.report["metadata_verified"] = True
        task.persist()
        archive = task.download(route)
        task.report.update(complete=True, stage="download-only-complete", splitting_requested=False)
        task.persist()
        return archive
    except BaseException as error:
        failure = {"type": type(error).__name__}
        if isinstance(error, (RuntimeError, transfer.TransferError)):
            failure["message"] = str(error)
        if isinstance(error, urllib.error.HTTPError):
            failure["http_status"] = error.code
        task.report.update(complete=False, error=failure)
        task.persist()
        raise RuntimeError("Authenticated release download failed; see sanitized transfer evidence") from None
    finally:
        task.token = ""


def verify_download_record(directory, expected):
    """Verify the retained transport association without network or execution."""
    directory = Path(directory)
    proof = read(directory / "transfer.json")
    require(proof.get("complete") and proof.get("stage") == "download-only-complete"
            and proof.get("metadata_verified") and proof.get("download_verified") and proof.get("expected") == expected,
            "Authenticated origin transport did not complete")
    require(proof["download"] == {"bytes": expected["size"], "sha256": expected["sha256"]}, "Original ZIP association differs")
    for response in (row["response"] for row in proof["network"] if "response" in row):
        require(Path(response["file"]).name == response["file"] and content(identity(directory / response["file"])) == content(response), "Retained API metadata bytes changed")
    repository, run, artifact = (read(directory / (name + ".json")) for name in ("repository", "run", "artifact"))
    require(repository["id"] == expected["repository_id"] and repository["full_name"].lower() == expected["repository"].lower(), "Retained repository differs")
    require(run["id"] == expected["run_id"] and run["run_attempt"] == expected["run_attempt"] and run["status"] == "completed"
            and run["conclusion"] == "success" and run["head_sha"] == expected["head_sha"] and run["head_branch"] == expected["head_branch"]
            and run["repository"]["id"] == run["head_repository"]["id"] == expected["repository_id"], "Retained run differs")
    origin = artifact["workflow_run"]
    require(artifact["id"] == expected["artifact_id"] and artifact["name"] == expected["artifact_name"] and not artifact["expired"]
            and artifact["size_in_bytes"] == expected["size"] and artifact["digest"] == "sha256:" + expected["sha256"]
            and origin["id"] == expected["run_id"] and origin["head_sha"] == expected["head_sha"] and origin["head_branch"] == expected["head_branch"]
            and origin["repository_id"] == origin["head_repository_id"] == expected["repository_id"], "Retained artifact differs")
    require(artifact["archive_download_url"] == f'https://api.github.com/repos/{expected["repository"]}/actions/artifacts/{expected["artifact_id"]}/zip', "Retained artifact endpoint differs")
    return proof


def native_snapshot(prefix, env):
    cc = Path(shutil.which("cc", path=env["PATH"])).resolve(strict=True)
    def query(*args):
        return subprocess.check_output([str(cc), *args], env=env, text=True, timeout=30).strip()
    files = {"cc": identity(cc), "lld": identity(prefix / "lib/rustlib" / HOST / "bin/gcc-ld/ld.lld"),
             "rust-lld": identity(prefix / "lib/rustlib" / HOST / "bin/rust-lld"), "rustc": identity(prefix / "bin/rustc")}
    for path in sorted((prefix / "lib").glob("*.so")):
        if path.name.startswith(("libLLVM", "librustc_driver")): files[path.name] = identity(path)
    for name in ("Scrt1.o", "crti.o", "crtn.o", "crtbeginS.o", "crtendS.o", "libgcc.a", "libgcc_s.so.1", "libc.so", "libc.so.6", "libm.so", "libm.so.6"):
        located = Path(query("-print-file-name=" + name))
        require(located.is_absolute() and located.is_file(), f"Native input is not concretely resolved: {name}")
        files[name] = identity(located)
    return {"scope": "Shared recorded native compiler/CRT/link inputs sampled before and after the complete serialized companion sequence; distinct from the old local-linked cohort.",
            "cc_version": query("--version"), "target": query("-dumpmachine"), "compiler_version": query("-dumpfullversion"),
            "sysroot_query": query("-print-sysroot"), "files": files,
            "os_release": identity(Path("/etc/os-release"))}


class LinkJob:
    def __init__(self, args):
        self.args = args
        self.work = args.output.resolve()
        self.work.mkdir(parents=True, exist_ok=False)
        (self.work / "evidence").mkdir()
        (self.work / "temporary").mkdir()
        self.report = {"schema": 1, "scope": __doc__, "complete": False, "commands": [], "producer": identity(__file__),
                       "configuration": identity(args.configuration), "configuration_contents": read(args.configuration),
                       "no_new_training": True, "no_framework_build": True, "no_remote_timing": True}
        validate_configuration(self.report["configuration_contents"])
        self.env = {k: v for k, v in os.environ.items() if k not in {"GITHUB_TOKEN", "GH_TOKEN", "LD_PRELOAD", "LD_AUDIT", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTC_BOOTSTRAP", "FERROUI_CC"}
                    and "TOKEN" not in k and not k.startswith(("FERROUI_PGO_", "FERROUI_BENCH_EXPECT_", "FERROUI_TEMPLATE_", "CARGO_PROFILE_"))}
        self.env.update(TMPDIR=str(self.work / "temporary"), LLVM_PROFILE_FILE=os.devnull, RAYON_NUM_THREADS="1", MALLOC_ARENA_MAX="1",
                        MALLOC_CONF="narenas:1,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,background_thread:false,retain:false")
        self.persist()

    def persist(self):
        save(self.work / "evidence/link-job.json", self.report)

    def run(self, label, command):
        row = {"label": label, "argv": list(map(str, command)), "complete": False, "started_unix": time.time()}
        self.report["commands"].append(row)
        self.persist()
        print("START", label, flush=True)
        with (self.work / "evidence" / (label + ".stdout")).open("wb") as out, (self.work / "evidence" / (label + ".stderr")).open("wb") as err:
            child = subprocess.Popen(row["argv"], env=self.env, stdout=out, stderr=err, start_new_session=True)
            try:
                row["exit"] = child.wait()
            except BaseException:
                os.killpg(child.pid, signal.SIGTERM)
                try: child.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                raise
            finally:
                row.update(finished_unix=time.time(), stdout=identity(out.name), stderr=identity(err.name))
                self.persist()
        require(row["exit"] == 0, f"{label} failed; raw evidence is retained")
        row["complete"] = True
        self.persist()
        print("END", label, "exit=0", flush=True)

    def execute(self):
        config = self.report["configuration_contents"]
        require(self.args.cpu in os.sched_getaffinity(0), "Select an allowed CPU")
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=HERE, text=True).strip()
        require(head == os.environ.get("GITHUB_SHA"), "Tooling checkout does not match the workflow head")
        require(not subprocess.check_output(["git", "status", "--porcelain"], cwd=HERE, text=True), "Tooling checkout is dirty")
        self.report["tooling_revision"] = head
        self.report["tooling_helpers"] = {p.name: identity(p) for p in [Path(__file__), HERE / "split_release_artifact.py", HERE / "bootstrap_toolchain.py", HERE / "run_winui_runtime_comparison.py", HERE / "verify_winui_profile_acceptance.py"]}
        self.report["imports"] = {}
        for name, expected in config["origins"].items():
            archive = download_release(expected, self.work / (name + "-download"))
            evidence = self.work / (name + "-release")
            extracted = safe_extract(archive, evidence, MAX_EXPANDED_RELEASE)
            self.run(name + "-import", [sys.executable, HERE / "run_winui_runtime_comparison.py", "import", "--evidence", evidence, "--output", self.work / (name + "-import")])
            self.report["imports"][name] = {"manifest": identity(self.work / (name + "-import/import.json")), "archive": identity(archive), "extraction": extracted}
            self.persist()
        self.selected = {}
        for role, selected in config["selection"].items():
            imported = runtime.verify_import(self.report["imports"][selected["origin"]]["manifest"]["path"])
            row = imported["arms"][selected["arm"]]
            require(row["revision"] == selected["revision"], "Selected source revision differs")
            self.selected[role] = row
        ref, cand = config["selection"]["reference"], config["selection"]["candidate"]
        self.run("select-acceptance-pair", [sys.executable, HERE / "run_winui_runtime_comparison.py", "compose",
                 "--reference-import", self.report["imports"][ref["origin"]]["manifest"]["path"], "--reference-arm", ref["arm"], "--reference-receiver-policy", ref["receiver_policy"],
                 "--candidate-import", self.report["imports"][cand["origin"]]["manifest"]["path"], "--candidate-arm", cand["arm"], "--candidate-receiver-policy", cand["receiver_policy"], "--output", self.work / "selection"])
        self.run("bootstrap", [sys.executable, HERE / "bootstrap_toolchain.py", "--work-dir", self.work / "toolchain"])
        self.prefix = self.work / "toolchain/install"
        self.env.update(PATH=str(self.prefix / "bin") + os.pathsep + str(self.prefix / "lib/rustlib" / HOST / "bin/gcc-ld") + os.pathsep + self.env.get("PATH", os.defpath),
                        LD_LIBRARY_PATH=os.pathsep.join([str(self.prefix / "lib"), str(self.prefix / "lib/rustlib" / HOST / "lib")]))
        self.report["native_before"] = native_snapshot(self.prefix, self.env)
        self.persist()
        self.run("acceptance-companions", [sys.executable, HERE / "run_winui_runtime_comparison.py", "run", "--import-manifest", self.work / "selection/selection.json",
                 "--toolchain-prefix", self.prefix, "--toolchain-manifest", self.work / "toolchain/installation.json", "--cpu", self.args.cpu, "--checks-only", "--output", self.work / "runtime"])
        compared = read(self.work / "runtime/runtime-comparison.json")
        require(compared.get("all_production_links_completed") and compared.get("production_correctness_passed")
                and not compared.get("paired_companion_measurements_completed"), "The sixteen checks-only companions did not complete")
        self.run("pgo-acceptance", [sys.executable, HERE / "verify_winui_profile_acceptance.py", "--import-manifest", self.work / "selection/selection.json", "--toolchain-prefix", self.prefix,
                 "--cpu", self.args.cpu, "--output", self.work / "pgo-acceptance"])
        require(read(self.work / "pgo-acceptance/result.json").get("accepted") is True, "PGO acceptance failed")
        extra = self.selected["correction_reference"]
        self.run("correction-reference-qa", [sys.executable, HERE / "prepare_property_validation.py", "build", "--mode", "production", "--kind", "qa",
                 "--fixture", self.work / "runtime/fixtures/property/fixture.json", "--base", extra["base"]["path"], "--bitflags", extra["bitflags"]["path"], "--profile", extra["profile"]["path"],
                 "--dependency", extra["graph"]["path"], "--dependency", extra["host_dependencies"]["path"], "--rustc", self.prefix / "bin/rustc", "--cpu", self.args.cpu, "--work-dir", self.work / "correction-reference/property"])
        for suite in ("correctness", "regressions"):
            self.run("correction-reference-" + suite, [sys.executable, HERE / "prepare_property_validation.py", "check", "--build", self.work / "correction-reference/property/qa/build.json", "--suite", suite, "--work-dir", self.work / ("correction-reference/" + suite)])
        self.report["native_after"] = native_snapshot(self.prefix, self.env)
        require(self.report["native_before"] == self.report["native_after"], "Native tool/CRT/sysroot inputs changed during links")
        self.report["shared_native_inputs_unchanged"] = True
        for expected in self.report["tooling_helpers"].values():
            require(identity(expected["path"]) == expected, "Tooling helper changed")
        require(identity(self.args.configuration) == self.report["configuration"], "Configuration changed")
        self.export_bundle(compared)
        self.report["complete"] = True
        self.persist()

    def export_bundle(self, compared):
        bundle = self.work / "bundle"
        bundle.mkdir()
        manifest = {"schema": 1, "kind": "ci-linked-runtime-companions", "complete": False, "scope": __doc__,
                    "original_work_directory": str(self.work),
                    "runtime_result": "runtime/runtime-comparison.json", "execution_snapshot": "evidence/link-job.json",
                    "tooling_revision": self.report["tooling_revision"], "configuration": self.report["configuration_contents"],
                    "native_link_environment": self.report["native_after"], "selected_origins": {}, "builds": {}, "files": {},
                    "no_new_training": True, "no_remote_measurements": True, "old_local_reference_reused": False}
        executable_paths = set()
        for role in ROLES:
            base = self.work / ("correction-reference" if role == "correction_reference" else "runtime/" + role)
            paths = {"property-" + kind: base / "property" / kind / "build.json" for kind in (PROPERTIES if role != "correction_reference" else ("qa",))}
            if role != "correction_reference": paths.update({kind: base / kind / "build.json" for kind in CALLABLES})
            manifest["builds"][role] = {kind: str(path.relative_to(self.work)) for kind, path in paths.items()}
            for path in paths.values():
                build = read(path)
                require(build["compiler"]["exit_code"] == 0 and build.get("inputs_unchanged_after_compiler"), "Incomplete companion build")
                require(content(build["base"]) == content(self.selected[role]["base"]) and content(build["profile"]) == content(self.selected[role]["profile"]), "Companion selected another Base/profile")
                executable_paths.add(Path(build["executable"]["path"]))
            manifest["selected_origins"][role] = {**self.report["configuration_contents"]["selection"][role], "base": self.selected[role]["base"], "profile": self.selected[role]["profile"], "source_manifest": self.selected[role]["source_manifest"]}
        def copy(path, relative):
            target = bundle / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, target)
            require(content(identity(target)) == content(identity(path)), "Bundle copy changed bytes")
        for directory in ("runtime", "correction-reference", "pgo-acceptance", "evidence"):
            for path in sorted((self.work / directory).rglob("*")):
                if not path.is_file() or path.is_symlink() or "tmp" in path.relative_to(self.work / directory).parts or "temporary" in path.relative_to(self.work / directory).parts:
                    continue
                if path in executable_paths or path.suffix in (".json", ".rs", ".log", ".stderr", ".txt") or (path.suffix == ".stdout" and path.stat().st_size < 4 * 1024 * 1024):
                    copy(path, path.relative_to(self.work))
        for name, record in self.report["imports"].items():
            copy(Path(record["manifest"]["path"]), Path("origins") / name / "import.json")
            for path in (self.work / (name + "-download/evidence")).glob("*.json"):
                copy(path, Path("origins") / name / path.name)
        copy(self.work / "selection/selection.json", Path("selection.json"))
        copy(self.work / "toolchain/installation.json", Path("toolchain-installation.json"))
        helper_paths = {Path(row["path"]) for row in compared["producer_inputs"].values()}
        helper_paths.update({HERE / "verify_winui_profile_acceptance.py", Path(__file__), HERE / "import_winui_runtime_companions.py", HERE / "split_release_artifact.py", HERE / "bootstrap_toolchain.py"})
        manifest["tooling_files"] = {}
        for path in sorted(helper_paths):
            copy(path, Path("tooling") / path.name)
            manifest["tooling_files"][str(path)] = str(Path("tooling") / path.name)
        for path in sorted(bundle.rglob("*")):
            if path.is_file(): manifest["files"][str(path.relative_to(bundle))] = {k: identity(path)[k] for k in ("bytes", "sha256")}
        require(sum(row["bytes"] for row in manifest["files"].values()) < MAX_BUNDLE, "Companion bundle exceeds its conservative uncompressed limit")
        manifest["complete"] = True
        save(bundle / "manifest.json", manifest)
        export = self.work / "export"
        export.mkdir()
        archive = export / "runtime-companions.zip"
        with zipfile.ZipFile(archive, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=6) as zipped:
            for path in sorted(bundle.rglob("*")):
                if path.is_file(): zipped.write(path, str(path.relative_to(bundle)))
        require(archive.stat().st_size + 1024 * 1024 < MAX_BUNDLE, "ZIP does not leave framing headroom below the connector limit")
        descriptor = {"archive": identity(archive), "manifest": identity(bundle / "manifest.json"), "maximum_export_bytes": MAX_BUNDLE,
                      "scope": "Companions and evidence only; all release graphs/profiles are authenticated external origins, not duplicated."}
        save(export / "bundle-identity.json", descriptor)
        self.report["export"] = descriptor


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--configuration", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cpu", type=int, required=True)
    args = parser.parse_args()
    job = LinkJob(args)
    signal.signal(signal.SIGTERM, lambda number, frame: (_ for _ in ()).throw(InterruptedError(f"Signal {number}")))
    try:
        job.execute()
    except BaseException as error:
        job.report["error"] = {"type": type(error).__name__}
        if isinstance(error, (RuntimeError, InterruptedError)):
            job.report["error"]["message"] = str(error)
        raise
    finally:
        job.persist()


if __name__ == "__main__":
    main()
