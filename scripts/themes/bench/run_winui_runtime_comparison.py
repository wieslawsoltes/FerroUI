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
from pathlib import Path, PurePosixPath
import shutil
import signal
import subprocess
import sys


HERE = Path(__file__).resolve().parent
HOST = "x86_64-unknown-linux-gnu"
ARMS = ("reference", "candidate")
SELECTION_KIND = "runtime-arm-selection"
RECEIVER_POLICIES = ("cloned", "borrowed")
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
    """Authenticate transported links against the recorded source graph."""
    directory = Path(directory).resolve()
    flattened = []
    children = {"": set()}
    for name in files:
        parts = PurePosixPath(name).parts
        require(name == str(PurePosixPath(name)) and not PurePosixPath(name).is_absolute()
                and parts and ".." not in parts, f"Invalid recorded source path: {name}")
        for index, part in enumerate(parts):
            children.setdefault("/".join(parts[:index]), set()).add(part)
    require(not set(files).intersection(children), "Recorded source file/directory collision")

    def recorded_target(name):
        pending, resolved, chain, seen = list(PurePosixPath(name).parts), [], [], set()
        while pending:
            state = (tuple(resolved), tuple(pending))
            require(state not in seen, f"Recorded source link cycle: {name}")
            seen.add(state)
            part = pending.pop(0)
            if part == "..":
                require(resolved, f"Recorded source link escapes its snapshot: {name}")
                resolved.pop()
                continue
            if part in ("", "."):
                continue
            prefix = "/".join((*resolved, part))
            entry = files.get(prefix, {})
            if "symlink" in entry:
                target = PurePosixPath(entry["symlink"])
                require(entry["symlink"] and not target.is_absolute(), f"Invalid/external recorded source link: {prefix}")
                require(len(chain) < 40, "Recorded source link chain exceeds the supported 40-link bound")
                chain.append({"path": prefix, "target": entry["symlink"]})
                pending = list(target.parts) + pending
            else:
                require(prefix in files or prefix in children, f"Unrecorded source link target: {prefix}")
                require(not pending or prefix in children, f"Recorded source link traverses a file: {prefix}")
                resolved.append(part)
        name = "/".join(resolved)
        require(name in files or name in children, f"Unrecorded source link target: {name}")
        return name, chain

    def verify_node(path, name, ancestors=()):
        resolved = path.resolve(strict=True)
        require(resolved.is_relative_to(directory), f"Source escapes its snapshot: {path}")
        expected = files.get(name, {})
        if "symlink" in expected:
            target_name, chain = recorded_target(name)
            if path.is_symlink():
                require(str(path.readlink()) == expected["symlink"], f"Source link changed: {name}")
                return verify_node(resolved, target_name, ancestors)
            count = verify_node(path, target_name, ancestors)
            flattened.append({"path": str(path.relative_to(directory)), "source_entry": name,
                              "original_target": expected["symlink"], "content_target": target_name,
                              "kind": "directory" if target_name in children else "file",
                              "recorded_link_chain": chain, "verified_files": count})
            return count
        require(not path.is_symlink(), f"Unexpected source symlink: {path}")
        if name in children:
            require(name not in ancestors, f"Recursive source directory link: {name}")
            require(path.is_dir(), f"Source directory missing: {path}")
            require({child.name for child in path.iterdir()} == children[name],
                    f"Source directory contains extra or missing entries: {path}")
            return sum(verify_node(path / child, "/".join(filter(None, (name, child))), (*ancestors, name))
                       for child in sorted(children[name]))
        require(path.is_file(), f"Source file missing: {path}")
        actual = identity(path)
        require(all(actual[k] == expected[k] for k in ("sha256", "bytes")), f"Source bytes changed: {name}")
        return 1

    for name in sorted(files):
        verify_node(directory / name, name)
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


def toolchain_content(record):
    """Compare distributions/tools, while retaining each origin's actual paths."""
    return {
        "host": record["host"], "compiler": record["compiler"],
        "components": {name: {key: value[key] for key in ("version", "expected_sha256")}
                       for name, value in record["components"].items()},
        "executables": {name: {key: value[key] for key in ("sha256", "bytes")}
                        for name, value in record["executables"].items()},
    }


def relocation_for(imported, arm, original_path):
    """Original CI paths are namespaced by origin, never merged across runs."""
    require(arm in ARMS, "Unknown runtime comparison arm")
    if imported.get("kind") == SELECTION_KIND:
        origin = imported["arm_selection"][arm]["origin"]
        return imported["origin_relocations"][origin].get(original_path)
    return imported["relocations"].get(original_path)


def verify_selection(imported):
    require(imported.get("schema_version") == 2 and imported.get("original_manifests_modified") is False,
            "Unsupported or mutable runtime selection")
    require("relocations" not in imported and "observed_comparison" not in imported,
            "A selection must not pretend to be one original CI run")
    require(set(imported["arm_selection"]) == set(imported["arms"]) == set(imported["receiver_policies"]) == set(ARMS),
            "A runtime selection must define exactly two explicit arms and receiver policies")
    origins = imported["origin_imports"]
    require(origins and set(origins) == set(imported["origin_relocations"]) == set(imported["observed_comparisons"])
            == set(imported["origin_toolchains"]), "Origin namespaces differ or are incomplete")
    require({choice["origin"] for choice in imported["arm_selection"].values()} == set(origins),
            "Unselected or missing origin namespace")
    require(len({record["path"] for record in origins.values()}) == len(origins),
            "The same import path must not have ambiguous origin namespaces")
    original_imports = {}
    for name, record in origins.items():
        require(identity(record["path"]) == record, "An origin import changed")
        source = read(record["path"])
        require(source.get("schema_version") == 1 and "kind" not in source and source.get("complete"),
                "Select from completed original imports, not nested or fabricated selections")
        require(imported["origin_relocations"][name] == source["relocations"]
                and imported["observed_comparisons"][name] == source["observed_comparison"]
                and imported["origin_toolchains"][name] == source["toolchain"],
                "Origin evidence differs from the original import")
        original_imports[name] = source
    for arm, choice in imported["arm_selection"].items():
        require(set(choice) == {"origin", "arm"} and choice["arm"] in ARMS,
                "Arm selection must name one exact origin and original arm")
        require(imported["receiver_policies"][arm] in RECEIVER_POLICIES, "Invalid receiver policy")
        require(imported["arms"][arm] == original_imports[choice["origin"]]["arms"][choice["arm"]],
                "Selected arm record differs from its original import")
    reference_origin = imported["arm_selection"]["reference"]["origin"]
    require(imported["toolchain"] == original_imports[reference_origin]["toolchain"],
            "Canonical toolchain must be the selected reference's original record")
    contents = []
    for name, source in original_imports.items():
        require(identity(source["toolchain"]["path"]) == source["toolchain"], "Origin toolchain manifest changed")
        contents.append(toolchain_content(read(source["toolchain"]["path"])))
    require(all(content == contents[0] for content in contents), "Selected origins use different Rust/LLVM distributions or tools")
    # Verify every original import in its own namespace. This preserves every
    # old/new path identity even when both CI runs used the same absolute root.
    for record in origins.values():
        verify_import(record["path"])
    return imported


def verify_import(path):
    imported = read(path)
    require(imported.get("complete"), "Import did not complete")
    if imported.get("kind") == SELECTION_KIND:
        return verify_selection(imported)
    require(imported.get("schema_version") == 1 and "kind" not in imported, "Unsupported artifact import")
    require(identity(imported["observed_comparison"]["path"]) == imported["observed_comparison"], "CI comparison changed")
    comparison = read(imported["observed_comparison"]["path"])
    require(comparison.get("complete") and set(imported["arms"]) == set(ARMS), "Original CI comparison or import is incomplete")
    old_root = Path(comparison["arms"]["reference"]["manifests"]["build"]["path"]).parents[2]
    require(str(old_root) == imported["original_evidence_root"], "Original evidence root changed")
    evidence = Path(imported["evidence_root"]).resolve(strict=True)

    def require_association(local, original):
        record = imported["relocations"].get(original["path"])
        require(record is not None and record["local"] == local
                and all(record["original"].get(key) == value for key, value in original.items()),
                "Arm identity has no exact original relocation")

    def source_location(old):
        result = (evidence / Path(old).relative_to(old_root)).resolve(strict=True)
        require(result.is_relative_to(evidence), "Original source path escapes the evidence root")
        return str(result)

    require_association(imported["toolchain"], comparison["toolchain"])
    for name, arm in imported["arms"].items():
        original, ci = arm["original"], comparison["arms"][name]
        require(ci.get("complete") and original["build"] == ci["manifests"]["build"], "Wrong original build association")
        for key, expected in original.items():
            require_association(arm[key], expected)
        build, base, source = (read(arm[key]["path"]) for key in ("build", "base_manifest", "source_manifest"))
        require(build.get("complete") and len(build["training_runs"]) == 13 and base["phases"]["build"]["exit"] == 0,
                "Original production build did not complete its existing PGO policy")
        required = {"base_manifest": build["base_manifest"], "source_manifest": build["source"],
                    "profile": build["profile"], "shipping": build["binary"], "source_archive": build["source_archive"],
                    "base": base["base"], "compiler_record": base["compiler_record_identity"],
                    "artifacts": ci["manifests"]["artifacts"], "archives": ci["manifests"]["archives"]}
        require(all(original[key] == value for key, value in required.items()), "Original Base/build/source association changed")
        require(base["source"] == build["source"] and base["profile"] == build["profile"] and base["graph"] == build["graph"]
                and ci["binary"] == build["binary"] and ci["profile"] == build["profile"] and ci["graph"] == build["graph"]
                and ci["manifests"]["base"] == build["base_manifest"], "Original production graph association changed")
        require(arm["revision"] == source["head"] == ci["revision"] == comparison["resolved_revisions"][name],
                "Selected source revision differs from its original CI arm")
        require(arm["source_directory"] == source_location(source["directory"]), "Frozen source directory changed")
        for graph_name, old_directory, expected_hash in (
            ("graph", base["graph"]["path"], base["graph"]["inventory_sha256"]),
            ("host_dependencies", base["host_dependencies"], base["host_inventory_sha256"]),
        ):
            graph = arm[graph_name]
            require(graph["path"] == source_location(old_directory) and graph["inventory"]["sha256"] == expected_hash,
                    "Preserved dependency graph association changed")
            require_association(graph["inventory"], {"path": str(Path(old_directory) / "hashes.json"), "sha256": expected_hash})
            require(graph["files"] == read(graph["inventory"]["path"]), "Dependency file inventory changed")
        require(arm["graph"]["files"].get(Path(arm["base"]["path"]).name) == arm["base"]["sha256"],
                "Selected Base differs from its graph")
        compiler = read(arm["compiler_record"]["path"])
        externs = [compiler["argv"][i + 1] for i, token in enumerate(compiler["argv"][:-1]) if token == "--extern"]
        bitflags = [value.split("=", 1)[1] for value in externs if value.startswith("bitflags=")]
        require(len(bitflags) == 1 and arm["bitflags_original_extern"] == bitflags[0]
                and Path(arm["bitflags"]["path"]).name == Path(bitflags[0]).with_suffix(".rlib").name
                and Path(arm["bitflags"]["path"]).parent == Path(arm["graph"]["path"])
                and arm["graph"]["files"].get(Path(arm["bitflags"]["path"]).name) == arm["bitflags"]["sha256"]
                and identity(arm["bitflags"]["path"]) == arm["bitflags"],
                "Bitflags selection differs from the original Base compiler")
        archives, view = read(arm["archives"]["path"]), read(arm["archives_view"]["path"])
        require(view.get("derived_view_only") is True and view["original_manifest"] == arm["archives"]
                and set(view["cases"]) == set(archives["cases"]) and len(view["cases"]) == 9,
                "Archive view lost its original nine-case association")
        for label, original_case in archives["cases"].items():
            actual = view["cases"][label]
            require(actual["font"] == original_case["font"], "Archive view changed an expected value")
            if original_case["archive"] is None:
                require(actual["archive"] is None, "Embedded archive case changed")
            else:
                require_association(actual["archive"], original_case["archive"])
    for record in imported["relocations"].values():
        require(identity(record["local"]["path"]) == record["local"], "Imported artifact changed")
    for arm in imported["arms"].values():
        for name in ("graph", "host_dependencies"):
            verify_graph_names(arm[name]["path"], arm[name]["files"])
        verify_source(arm["source_directory"], read(arm["source_manifest"]["path"])["files"])
        require(identity(arm["archives_view"]["path"]) == arm["archives_view"], "Relocated archive view changed")
    return imported


def compose_imports(args):
    """Write a derived view; neither an original manifest nor a graph is edited."""
    output = fresh(args.output)
    report = {"schema_version": 2, "kind": SELECTION_KIND, "complete": False,
              "producer": identity(__file__), "original_manifests_modified": False,
              "scope": "Explicit selection from separately verified CI imports; not a new matched CI build.",
              "origin_imports": {}, "origin_relocations": {}, "observed_comparisons": {},
              "origin_toolchains": {}, "arm_selection": {}, "arms": {}, "receiver_policies": {}}
    destination = output / "selection.json"
    save(destination, report)
    origin_paths = {}
    try:
        for arm in ARMS:
            path = getattr(args, arm + "_import").resolve(strict=True)
            require(not output.is_relative_to(path.parent), "Keep selections outside the original import directory")
            source = read(path)
            require(source.get("schema_version") == 1 and "kind" not in source,
                    "Compose original imports only; nested selections are unsupported")
            source = verify_import(path)
            origin = origin_paths.get(str(path))
            if origin is None:
                origin = "origin-" + str(len(origin_paths))
                origin_paths[str(path)] = origin
                report["origin_imports"][origin] = identity(path)
                report["origin_relocations"][origin] = source["relocations"]
                report["observed_comparisons"][origin] = source["observed_comparison"]
                report["origin_toolchains"][origin] = source["toolchain"]
            selected = getattr(args, arm + "_arm")
            report["arm_selection"][arm] = {"origin": origin, "arm": selected}
            report["arms"][arm] = source["arms"][selected]
            report["receiver_policies"][arm] = getattr(args, arm + "_receiver_policy")
            if arm == "reference":
                report["toolchain"] = source["toolchain"]
            save(destination, report)
        # Completion is published only after all identity/association checks.
        verify_selection(report)
        report["complete"] = True
    except BaseException as error:
        report["error"] = {"type": type(error).__name__, "message": str(error)}
        raise
    finally:
        save(destination, report)
    print(destination)


def receiver_policies(args, imported):
    recorded = imported.get("receiver_policies", {})
    result = {}
    for arm, default in (("reference", "cloned"), ("candidate", "borrowed")):
        explicit = getattr(args, arm + "_receiver_policy", None)
        require(explicit is None or arm not in recorded or explicit == recorded[arm],
                "Explicit receiver policy differs from the derived selection")
        result[arm] = explicit or recorded.get(arm) or default
        require(result[arm] in RECEIVER_POLICIES, "Invalid runtime receiver policy")
    require(result["candidate"] == "borrowed", "The existing paired callable producer requires a borrowed candidate")
    return result


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
        self.report["receiver_policies"] = receiver_policies(self.args, imported)
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
        for arm, policy in self.report["receiver_policies"].items():
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
    compose = commands.add_parser("compose", help="select arms from completed imports without merging their original path namespaces")
    for arm in ARMS:
        compose.add_argument("--" + arm + "-import", type=Path, required=True)
        compose.add_argument("--" + arm + "-arm", choices=ARMS, required=True)
        compose.add_argument("--" + arm + "-receiver-policy", choices=RECEIVER_POLICIES, required=True)
    compose.add_argument("--output", type=Path, required=True)
    run = commands.add_parser("run", help="link/check both real production graphs, then execute existing paired cohorts")
    run.add_argument("--import-manifest", type=Path, required=True)
    run.add_argument("--toolchain-prefix", type=Path, required=True)
    run.add_argument("--toolchain-manifest", type=Path, required=True)
    run.add_argument("--output", type=Path, required=True)
    run.add_argument("--cpu", type=int, required=True)
    run.add_argument("--checks-only", action="store_true")
    run.add_argument("--reference-receiver-policy", choices=RECEIVER_POLICIES)
    run.add_argument("--candidate-receiver-policy", choices=RECEIVER_POLICIES)
    run.add_argument("--pgo-audit-script", type=Path, default=HERE / "verify_winui_profile_acceptance.py")
    args = parser.parse_args()
    if args.command == "import":
        import_evidence(args)
        return
    if args.command == "compose":
        compose_imports(args)
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
