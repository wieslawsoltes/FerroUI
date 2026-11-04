#!/usr/bin/env python3
"""Import, check and measure one authenticated CI-linked companion bundle.

Import never starts a compiler, ELF image or other subprocess. Original bytes and
manifests remain immutable. Separate runtime views relocate file locators only;
actual compiler commands, native-link identities and Rust fixture bytes remain
unchanged. Check is an explicit later execution step. Measure uses the original
individual producers and newly CI-linked binaries on both sides of every pair.
No command trains PGO or promotes a release automatically.
"""
from __future__ import annotations

import argparse
import copy
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import signal
import struct
import subprocess
import sys
import time

import run_winui_runtime_comparison as runtime
from link_winui_runtime_artifacts import CALLABLES, HOST, MAX_BUNDLE, PROPERTIES, ROLES, content, identity, read, require, safe_extract, save, validate_configuration, verify_download_record

HERE = Path(__file__).resolve().parent


def relative_file(root, name):
    relative = PurePosixPath(name)
    require(name == str(relative) and not relative.is_absolute() and ".." not in relative.parts, "Invalid bundle-relative path")
    result = (Path(root) / name).resolve(strict=True)
    require(result.is_relative_to(Path(root).resolve()) and result.is_file() and not (Path(root) / name).is_symlink(), "Bundle file escapes its root")
    return result


def verify_bundle(directory):
    directory = Path(directory).resolve(strict=True)
    manifest = read(directory / "manifest.json")
    require(manifest.get("schema") == 1 and manifest.get("kind") == "ci-linked-runtime-companions" and manifest.get("complete"), "Incomplete or unsupported companion bundle")
    validate_configuration(manifest["configuration"])
    actual = {str(p.relative_to(directory)) for p in directory.rglob("*") if p.is_file()}
    require(actual == set(manifest["files"]) | {"manifest.json"}, "Bundle contains missing or unrecorded files")
    require(not any(p.is_symlink() for p in directory.rglob("*")), "Bundle links are unsupported")
    for name, expected in manifest["files"].items():
        require(content(identity(relative_file(directory, name))) == content(expected), f"Bundle bytes changed: {name}")
    helpers = manifest["tooling_files"]
    require(helpers and len({Path(name).name for name in helpers}) == len(helpers)
            and len(set(helpers.values())) == len(helpers), "Ambiguous tooling identity mapping")
    for original, relative in helpers.items():
        require(Path(original).is_absolute() and relative == "tooling/" + Path(original).name
                and relative in manifest["files"], "Invalid exact tooling mapping")
    require(set(manifest["builds"]) == set(ROLES) and set(manifest["selected_origins"]) == set(ROLES), "Incomplete companion roles")
    require(manifest.get("no_new_training") and manifest.get("no_remote_measurements") and manifest.get("old_local_reference_reused") is False,
            "Bundle does not represent the reviewed compiler-only cohort")
    for name, expected in manifest["configuration"]["origins"].items():
        verify_download_record(directory / "origins" / name, expected)
    executed = read(relative_file(directory, manifest["execution_snapshot"]))
    require(executed.get("shared_native_inputs_unchanged") and executed["native_before"] == executed["native_after"] == manifest["native_link_environment"]
            and executed.get("no_new_training") and executed.get("no_framework_build") and executed.get("no_remote_timing"), "Native execution policy association differs")
    require(executed["commands"] and all(row.get("complete") and row.get("exit") == 0 for row in executed["commands"]), "Remote command sequence did not complete")
    checked = read(relative_file(directory, manifest["runtime_result"]))
    require(checked.get("all_production_links_completed") and checked.get("production_correctness_passed")
            and not checked.get("paired_companion_measurements_completed"), "Remote checks-only result is incomplete")
    return manifest


def translated_input(path, prefixes, bundle, directory, original_work):
    """Resolve only a selected origin prefix or an exact transported helper."""
    path = Path(path)
    helper = bundle["tooling_files"].get(str(path))
    if helper is not None:
        return relative_file(directory, helper)
    candidates = []
    for before, after in prefixes:
        if path.is_relative_to(before):
            candidates.append((len(before.parts), after / path.relative_to(before)))
    if candidates:
        length = max(size for size, _ in candidates)
        matches = {p.resolve(strict=True) for size, p in candidates if size == length}
        require(len(matches) == 1, "Ambiguous external origin path")
        return matches.pop()
    require(path.is_relative_to(original_work), "Input has no selected origin or exact tooling mapping")
    relative = str(path.relative_to(original_work))
    require(relative in bundle["files"], "Referenced source input was not transported")
    return relative_file(directory, relative)


def elf_sections(path):
    data = Path(path).read_bytes()
    require(data[:6] == b"\x7fELF\x02\x01" and len(data) >= 64, "Require ELF64 little endian")
    header = struct.unpack_from("<16sHHIQQQIHHHHHH", data)
    require(header[2] == 62 and header[1] in (2, 3), "Require native x86_64 executable ELF")
    offset, entry_size, count, names_index = header[6], header[11], header[12], header[13]
    require(entry_size == 64 and count and names_index < count and offset + count * entry_size <= len(data), "Unsupported ELF section table")
    sections = [struct.unpack_from("<IIQQQQIIQQ", data, offset + i * entry_size) for i in range(count)]
    strings = sections[names_index]
    require(strings[4] + strings[5] <= len(data), "Invalid ELF section names")
    names = data[strings[4]:strings[4] + strings[5]]
    return {names[s[0]:].split(b"\0", 1)[0].decode("ascii") for s in sections}


def compiler_options(command):
    result, externs, dependencies = {}, {}, []
    for i, token in enumerate(command):
        value = command[i + 1] if token == "-C" and i + 1 < len(command) else token[2:] if token.startswith("-C") else None
        if value is not None:
            key, _, setting = value.partition("=")
            result.setdefault(key, []).append(setting)
        if token == "--extern":
            name, path = command[i + 1].split("=", 1)
            require(name not in externs, "Duplicate extern")
            externs[name] = path
        if token == "-L":
            dependencies.append(command[i + 1])
        elif token.startswith("-L"):
            dependencies.append(token[2:])
    return result, externs, dependencies


def verify_build_contract(build, kind, origin, native):
    require(build.get("mode") == "production" and build["compiler"].get("exit_code") == 0
            and build.get("inputs_unchanged_after_compiler") is True, "Require a completed production companion")
    require(build["kind"] == kind and content(build["base"]) == content(origin["base"])
            and build["base"]["path"] == origin["base"]["path"] and build["profile"] == origin["profile"], "Companion Base/profile origin differs")
    command = build["compiler"]["command"]
    if "command" in build: require(build["command"] == command, "Compiler command records disagree")
    options, externs, dependencies = compiler_options(command)
    require(set(options) == {"opt-level", "panic", "debuginfo", "debug-assertions", "codegen-units", "lto", "strip", "llvm-args", "profile-use", "link-arg", "linker"},
            "Unrecorded compiler codegen policy option")
    for key, value in {"opt-level": "z", "panic": "unwind", "debuginfo": "0", "debug-assertions": "no", "codegen-units": "1", "lto": "fat", "strip": "symbols", "llvm-args": "--addrsig", "profile-use": origin["profile"]["path"]}.items():
        require(options.get(key) == [value], f"Production compiler policy differs: {key}")
    link_args = ["-fuse-ld=lld", "-Wl,--icf=safe", "-Wl,-z,pack-relative-relocs"]
    if kind in PROPERTIES: link_args.append("-Wl,--threads=1")
    require(options["link-arg"] == link_args, "Native link policy differs")
    require(externs.get("ferroui_base") == origin["base"]["path"], "Compiler extern did not use recorded Base")
    require(set(dependencies) == {"dependency=" + origin["graph"]["path"], "dependency=" + origin["host_dependencies"]["path"]}, "Compiler dependency search graph differs")
    require(command.count("--target") == 1 and command[command.index("--target") + 1] == HOST, "Compiler target differs")
    require(command.count("-o") == 1 and command[command.index("-o") + 1] == build["executable"]["path"], "Compiler output differs")
    require(command.count("--test") == int(kind in ("qa", "allocations", "animation")), "Test-harness compilation policy differs")
    cfg = [command[i + 1] for i, token in enumerate(command[:-1]) if token == "--cfg"]
    require(cfg == (["ferro_count_allocations"] if kind == "allocations" else []), "Additional compiler cfg changed fixture semantics")
    recorded = {r["path"]: r["files"] for r in build["dependencies"]}
    require(recorded == {origin["graph"]["path"]: origin["graph"]["files"], origin["host_dependencies"]["path"]: origin["host_dependencies"]["files"]}, "Recorded dependency inventory differs")
    if kind in PROPERTIES:
        require(set(externs) == {"ferroui_base", "bitflags"} and build["source"]["path"] in command,
                "Property compiler source or extern list changed")
        require(externs.get("bitflags") == origin["bitflags"]["path"] and build["bitflags"] == origin["bitflags"], "Compiler bitflags extern differs")
        require(build.get("debug_assertions") is False and build.get("hot_reload_cfg") is False and build.get("development_policy") is None,
                "Development cfg entered the production fixture")
        require(build["link_policy"] == "ordinary unwind; no compact-unwind relink", "Property link policy differs")
        cc, lld = build["linker"], build["lld"]
    else:
        require(set(externs) == {"ferroui_base"}, "Callable extern list changed")
        require(build.get("compact_unwind_applied") is True and build.get("development_options") is None, "Callable compact-unwind production policy differs")
        cc, lld = build["native_linker"]["cc"], build["native_linker"]["lld"]
    require(cc["sha256"] == native["files"]["cc"]["sha256"], "Companion did not use the common native CC")
    if "bytes" in cc: require(cc["bytes"] == native["files"]["cc"]["bytes"], "Native CC size differs")
    # The callable helper records the Rust lld wrapper and, where applicable,
    # its backend separately. The property helper records the selected wrapper.
    require(lld["sha256"] == native["files"]["lld"]["sha256"], "Companion did not use the common selected LLD")
    if "bytes" in lld: require(lld["bytes"] == native["files"]["lld"]["bytes"], "Selected LLD size differs")
    if "backend" in lld:
        require(lld["backend"]["sha256"] == native["files"]["rust-lld"]["sha256"], "LLD backend differs")


class Importer:
    def __init__(self, args):
        self.args = args
        self.work = args.output.resolve()
        self.work.mkdir(parents=True, exist_ok=False)
        self.result = {"schema": 1, "kind": "imported-ci-runtime-companions", "complete": False, "scope": __doc__, "producer": identity(__file__),
                       "archive": identity(args.archive), "executed_on_import": False, "original_manifests_modified": False,
                       "origin_imports": {}, "roles": {}, "derived_files": {}}
        require(content(self.result["archive"]) == (args.archive_bytes, args.archive_sha256), "Companion ZIP identity differs")
        require(args.archive_bytes < MAX_BUNDLE, "Companion ZIP exceeds the transport bound")
        self.persist()

    def persist(self):
        save(self.work / "import.json", self.result)

    def original(self, path):
        relative = Path(path).relative_to(self.original_work)
        require(str(relative) in self.bundle["files"], "Referenced original file was not transported")
        return relative_file(self.directory, str(relative))

    def translate_data(self, path):
        return translated_input(path, self.prefixes, self.bundle, self.directory, self.original_work)

    def fixture(self, original_path, property_fixture):
        if original_path in self.fixtures: return self.fixtures[original_path]
        raw_path = self.original(original_path)
        raw = read(raw_path)
        derived = copy.deepcopy(raw)
        directory_key = "fixture_directory" if property_fixture else "directory"
        before = Path(raw[directory_key])
        after = self.original(before / next(iter(raw["files"]))).parent
        derived[directory_key] = str(after)
        for name, expected in raw["files"].items():
            require(Path(name).name == name, "Fixture inventory must use basenames")
            actual = identity(self.original(before / name))
            if property_fixture:
                require(expected["path"] == str(before / name) and content(actual) == content(expected), "Property fixture bytes changed")
                derived["files"][name] = actual
            else:
                require(actual["sha256"] == expected, "Callable fixture bytes changed")
        # Keep original provenance verbatim. Verify selected frozen sources and
        # the exact transported tooling inputs, including helpers outside work.
        for row in raw.get("source_inputs", []):
            require(content(identity(self.translate_data(row["path"]))) == content(row), "Fixture source origin changed")
        derived["transport_derivation"] = {"original_manifest": identity(raw_path), "rust_source_bytes_unchanged": True, "locator_changes_only": True}
        target = self.work / "views/fixtures" / f"{len(self.fixtures):02d}.json"
        target.parent.mkdir(parents=True, exist_ok=True)
        save(target, derived)
        self.fixtures[original_path] = (target, derived)
        self.result["derived_files"][str(target.relative_to(self.work))] = identity(target)
        return target, derived

    def execute(self):
        self.directory = self.work / "original-bundle"
        self.result["extraction"] = safe_extract(self.args.archive, self.directory, MAX_BUNDLE)
        self.bundle = verify_bundle(self.directory)
        self.result["bundle_manifest"] = identity(self.directory / "manifest.json")
        self.original_work = Path(self.bundle["original_work_directory"])
        require(self.original_work.is_absolute(), "Original work root must be absolute")
        self.prefixes, self.fixtures, self.local_origins, self.remote_origins = [], {}, {}, {}
        for name in ("acceptance", "correction"):
            local_path = getattr(self.args, name + "_import")
            local = runtime.verify_import(local_path)
            require(local.get("schema_version") == 1 and local.get("kind") != runtime.SELECTION_KIND, "Supply the original release import for each origin")
            remote_path = relative_file(self.directory, f"origins/{name}/import.json")
            remote = read(remote_path)
            require(remote.get("complete") and remote.get("original_manifests_modified") is False, "Incomplete remote origin import")
            require(content(remote["observed_comparison"]) == content(local["observed_comparison"]), "Local and remote artifacts are different cohorts")
            self.local_origins[name], self.remote_origins[name] = local, remote
            self.result["origin_imports"][name] = {"local": identity(local_path), "remote": identity(remote_path)}
            for arm in runtime.ARMS:
                old, new = remote["arms"][arm], local["arms"][arm]
                require(old["original"] == new["original"] and old["revision"] == new["revision"], "Origin manifest associations differ")
                for key in ("base", "profile", "build", "source_manifest", "compiler_record", "shipping", "bitflags"):
                    require(content(old[key]) == content(new[key]), f"Origin identity differs: {key}")
                for key in ("graph", "host_dependencies"):
                    require(old[key]["files"] == new[key]["files"], "Origin graph inventory differs")
                    self.prefixes.append((Path(old[key]["path"]), Path(new[key]["path"])))
                self.prefixes.append((Path(old["source_directory"]), Path(new["source_directory"])))
        native = self.bundle["native_link_environment"]
        for role in ROLES:
            selected = self.bundle["configuration"]["selection"][role]
            remote = self.remote_origins[selected["origin"]]["arms"][selected["arm"]]
            local = self.local_origins[selected["origin"]]["arms"][selected["arm"]]
            require(selected["revision"] == remote["revision"] == self.bundle["selected_origins"][role]["revision"], "Selected revision differs")
            require(self.bundle["selected_origins"][role]["base"] == remote["base"] and self.bundle["selected_origins"][role]["profile"] == remote["profile"], "Selected Base/profile differs")
            required = {"property-qa"} if role == "correction_reference" else {"property-" + kind for kind in PROPERTIES} | set(CALLABLES)
            require(set(self.bundle["builds"][role]) == required, "Companion build matrix changed")
            self.result["roles"][role] = {"selection": selected, "source_import": identity(getattr(self.args, selected["origin"] + "_import")), "builds": {}}
            for label, relative in self.bundle["builds"][role].items():
                original_path = relative_file(self.directory, relative)
                raw = read(original_path)
                kind = label.removeprefix("property-")
                verify_build_contract(raw, kind, remote, native)
                fixture_path, fixture = self.fixture(raw["fixture"]["path"], label.startswith("property-"))
                require(content(identity(self.original(raw["fixture"]["path"]))) == content(raw["fixture"]), "Original fixture manifest changed")
                executable = self.original(raw["executable"]["path"])
                require(content(identity(executable)) == content(raw["executable"]), "Companion executable bytes differ")
                sections = elf_sections(executable)
                require((".cfi_reserve" in sections) == (not label.startswith("property-")), "Actual unwind representation differs from the producer policy")
                view = copy.deepcopy(raw)
                image = self.work / "views/images" / (role + "-" + label)
                image.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(executable, image)
                image.chmod(0o755)
                require(content(identity(image)) == content(raw["executable"]), "Executable staging changed bytes")
                view["executable"] = {**identity(image), "storage": {"resolved": str(image.resolve()), "st_dev": image.stat().st_dev}}
                view["fixture"] = identity(fixture_path)
                if "fixture_contents" in view: view["fixture_contents"] = fixture
                for key in ("base", "profile", "bitflags"):
                    if key in view: view[key] = local[key]
                if "source" in view:
                    actual = identity(self.original(raw["source"]["path"]))
                    require(content(actual) == content(raw["source"]), "Compiler source bytes changed")
                    view["source"] = actual
                for dependency in view["dependencies"]:
                    if dependency["path"] == remote["graph"]["path"]: dependency["path"] = local["graph"]["path"]
                    elif dependency["path"] == remote["host_dependencies"]["path"]: dependency["path"] = local["host_dependencies"]["path"]
                    else: raise RuntimeError("Unassociated dependency directory")
                view["transport_derivation"] = {"original_build": identity(original_path), "original_executable": raw["executable"], "origin_import": self.result["origin_imports"][selected["origin"]], "compiler_command_and_native_records_unchanged": True, "scope": "Runtime view only; this manifest did not execute a local compiler."}
                destination = self.work / "views" / role / label / "build.json"
                destination.parent.mkdir(parents=True, exist_ok=True)
                save(destination, view)
                self.result["roles"][role]["builds"][label] = {"original": identity(original_path), "view": identity(destination), "executable": identity(image), "fixture": identity(fixture_path)}
                self.result["derived_files"][str(destination.relative_to(self.work))] = identity(destination)
                self.result["derived_files"][str(image.relative_to(self.work))] = identity(image)
                self.persist()
        audit = read(relative_file(self.directory, "pgo-acceptance/result.json"))
        require(audit.get("complete") and audit.get("accepted") and audit.get("input_identities_unchanged"), "Mandatory remote PGO acceptance is incomplete")
        for role in ("reference", "candidate"):
            selected = self.bundle["configuration"]["selection"][role]
            origin = self.remote_origins[selected["origin"]]["arms"][selected["arm"]]
            arm = audit["arms"][role]
            require(arm.get("complete") and arm.get("accepted") and arm["revision"] == selected["revision"]
                    and arm["inputs"]["base"] == origin["base"] and arm["inputs"]["profile"] == origin["profile"], "PGO acceptance belongs to another graph")
        self.result["pgo_acceptance"] = identity(self.directory / "pgo-acceptance/result.json")
        self.result["native_link_environment"] = native
        self.result["tooling"] = {Path(old).name: identity(relative_file(self.directory, relative)) for old, relative in self.bundle["tooling_files"].items()}
        for role in ROLES:
            for row in self.result["roles"][role]["builds"].values():
                original = read(row["original"]["path"])
                require(content(original["producer"]) == content(self.result["tooling"][Path(original["producer"]["path"]).name]), "Companion producer bytes are not preserved")
        self.result["complete"] = True
        self.persist()


def verify_imported(path):
    record = read(path)
    require(record.get("complete") and record.get("kind") == "imported-ci-runtime-companions"
            and record.get("executed_on_import") is False and record.get("original_manifests_modified") is False,
            "Require a completed data-only import")
    require(identity(record["archive"]["path"]) == record["archive"], "Original companion ZIP changed")
    bundle = Path(record["bundle_manifest"]["path"]).parent
    require(identity(record["bundle_manifest"]["path"]) == record["bundle_manifest"], "Original bundle manifest changed")
    contract = verify_bundle(bundle)
    work = Path(path).resolve().parent
    original_work = Path(contract["original_work_directory"])
    def original_file(old):
        relative = str(Path(old).relative_to(original_work))
        require(relative in contract["files"], "Runtime view refers to an omitted original input")
        return relative_file(bundle, relative)
    local_imports, remote_imports, prefixes = {}, {}, []
    require(set(record["origin_imports"]) == set(contract["configuration"]["origins"]), "Origin inventory changed")
    for name, origin in record["origin_imports"].items():
        for key in ("local", "remote"): require(identity(origin[key]["path"]) == origin[key], "Origin import changed")
        require(Path(origin["remote"]["path"]) == bundle / "origins" / name / "import.json", "Remote origin locator changed")
        local_imports[name] = runtime.verify_import(origin["local"]["path"])
        remote_imports[name] = read(origin["remote"]["path"])
        require(content(local_imports[name]["observed_comparison"]) == content(remote_imports[name]["observed_comparison"]), "Origin comparison association changed")
        for arm in runtime.ARMS:
            old, new = remote_imports[name]["arms"][arm], local_imports[name]["arms"][arm]
            require(old["original"] == new["original"] and old["revision"] == new["revision"], "Origin source association changed")
            for key in ("base", "profile", "build", "source_manifest", "compiler_record", "shipping", "bitflags"):
                require(content(old[key]) == content(new[key]), "Origin artifact association changed")
            for key in ("graph", "host_dependencies"):
                require(old[key]["files"] == new[key]["files"], "Origin graph association changed")
                prefixes.append((Path(old[key]["path"]), Path(new[key]["path"])))
            prefixes.append((Path(old["source_directory"]), Path(new["source_directory"])))
    for row in record["derived_files"].values(): require(identity(row["path"]) == row, "Derived runtime file changed")
    expected_tooling = {Path(old).name: identity(relative_file(bundle, relative)) for old, relative in contract["tooling_files"].items()}
    require(record["tooling"] == expected_tooling, "Frozen producer origin or bytes changed")
    expected_files = set()
    for role, selected in contract["configuration"]["selection"].items():
        require(record["roles"][role]["selection"] == selected and record["roles"][role]["source_import"] == record["origin_imports"][selected["origin"]]["local"], "Runtime role points to another origin")
        remote = remote_imports[selected["origin"]]["arms"][selected["arm"]]
        local = local_imports[selected["origin"]]["arms"][selected["arm"]]
        require(remote["original"] == local["original"] and local["revision"] == selected["revision"], "Source origin relationship changed")
        require(set(record["roles"][role]["builds"]) == set(contract["builds"][role]), "Runtime build role matrix changed")
        for label, row in record["roles"][role]["builds"].items():
            raw_file = relative_file(bundle, contract["builds"][role][label])
            require(row["original"] == identity(raw_file), "Runtime view selected another original build")
            raw = read(raw_file)
            verify_build_contract(raw, label.removeprefix("property-"), remote, contract["native_link_environment"])
            view_path = work / "views" / role / label / "build.json"
            require(row["view"] == identity(view_path), "Runtime view identity or placement changed")
            current = read(view_path)
            expected = copy.deepcopy(raw)
            image = work / "views/images" / (role + "-" + label)
            require(row["executable"] == identity(image) and content(row["executable"]) == content(raw["executable"]), "Runtime executable differs from the actual linked image")
            expected["executable"] = {**identity(image), "storage": {"resolved": str(image.resolve()), "st_dev": image.stat().st_dev}}
            fixture_file = original_file(raw["fixture"]["path"])
            require(content(identity(fixture_file)) == content(raw["fixture"]), "Original fixture manifest identity differs")
            original_fixture = read(fixture_file)
            derived_fixture = copy.deepcopy(original_fixture)
            directory_key = "fixture_directory" if label.startswith("property-") else "directory"
            old_directory = Path(original_fixture[directory_key])
            new_directory = original_file(old_directory / next(iter(original_fixture["files"]))).parent
            derived_fixture[directory_key] = str(new_directory)
            for name, source in original_fixture["files"].items():
                actual = identity(original_file(old_directory / name))
                if label.startswith("property-"):
                    require(content(actual) == content(source), "Runtime property source bytes changed")
                    derived_fixture["files"][name] = actual
                else: require(actual["sha256"] == source, "Runtime callable source bytes changed")
            for source in original_fixture.get("source_inputs", []):
                local_source = translated_input(source["path"], prefixes, contract, bundle, original_work)
                require(content(identity(local_source)) == content(source), "Runtime fixture source origin changed")
            derived_fixture["transport_derivation"] = {"original_manifest": identity(fixture_file), "rust_source_bytes_unchanged": True, "locator_changes_only": True}
            local_fixture = Path(row["fixture"]["path"])
            require(local_fixture.parent == work / "views/fixtures" and identity(local_fixture) == row["fixture"]
                    and read(local_fixture) == derived_fixture, "Runtime fixture view changed more than locators")
            expected["fixture"] = identity(local_fixture)
            if "fixture_contents" in expected: expected["fixture_contents"] = derived_fixture
            for key in ("base", "profile", "bitflags"):
                if key in expected: expected[key] = local[key]
            if "source" in expected: expected["source"] = identity(original_file(raw["source"]["path"]))
            for dependency in expected["dependencies"]:
                key = "graph" if dependency["path"] == remote["graph"]["path"] else "host_dependencies"
                require(dependency["path"] == remote[key]["path"], "Runtime dependency origin is ambiguous")
                dependency["path"] = local[key]["path"]
            expected["transport_derivation"] = {"original_build": identity(raw_file), "original_executable": raw["executable"], "origin_import": record["origin_imports"][selected["origin"]], "compiler_command_and_native_records_unchanged": True, "scope": "Runtime view only; this manifest did not execute a local compiler."}
            require(current == expected, "Runtime build view changed more than approved locators")
            for item in (view_path, image, local_fixture): expected_files.add(str(item.relative_to(work)))
    require(set(record["derived_files"]) == expected_files, "Derived-file inventory contains missing or extra inputs")
    return record


class Execution:
    def __init__(self, args):
        self.args = args
        self.imported = verify_imported(args.import_manifest)
        require(args.cpu in os.sched_getaffinity(0), "Select an allowed CPU")
        self.work = args.output.resolve()
        self.work.mkdir(parents=True, exist_ok=False)
        (self.work / "logs").mkdir()
        (self.work / "tmp").mkdir()
        self.env = {k: v for k, v in os.environ.items() if k not in {"LD_PRELOAD", "LD_AUDIT", "LLVM_PROFILE_FILE", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"} and not k.startswith(("FERROUI_PGO_", "FERROUI_BENCH_EXPECT_"))}
        self.env.update(TMPDIR=str(self.work / "tmp"), LLVM_PROFILE_FILE=os.devnull, RAYON_NUM_THREADS="1", MALLOC_ARENA_MAX="1")
        self.report = {"schema": 1, "scope": __doc__, "complete": False, "full_release_accepted": False, "import": identity(args.import_manifest), "producer": identity(__file__), "commands": [], "cpu": args.cpu,
                       "cohort": "Newly CI-linked companions on both arms, executed on this local host; original local-link results remain separate.", "shipping_startup": "Separate validation remains required; this companion producer does not measure the shipping application."}
        self.persist()

    def persist(self): save(self.work / "result.json", self.report)

    def run(self, label, command):
        row = {"label": label, "argv": list(map(str, command)), "started_unix": time.time()}
        self.report["commands"].append(row); self.persist()
        with (self.work / "logs" / (label + ".stdout")).open("wb") as out, (self.work / "logs" / (label + ".stderr")).open("wb") as err:
            child = subprocess.Popen(row["argv"], env=self.env, stdout=out, stderr=err, start_new_session=True)
            try: row["exit"] = child.wait()
            except BaseException:
                os.killpg(child.pid, signal.SIGTERM)
                try: child.wait(timeout=15)
                except subprocess.TimeoutExpired: os.killpg(child.pid, signal.SIGKILL); child.wait()
                raise
            finally:
                row.update(stdout=identity(out.name), stderr=identity(err.name), finished_unix=time.time()); self.persist()
        require(row["exit"] == 0, f"{label} failed; preserve this output")
        return row

    def producer(self, name, *args): return [sys.executable, self.imported["tooling"][name]["path"], *args]
    def build(self, role, label): return self.imported["roles"][role]["builds"][label]["view"]["path"]

    def check(self):
        from runtime_artifact_host import verify_native
        images = [Path(row["executable"]["path"]) for role in self.imported["roles"].values() for row in role["builds"].values()]
        self.report["host_proof"] = verify_native(images, self.work / "host-proof", self.env)
        require(self.report["host_proof"].get("complete"), "Local ELF/loader/native ABI proof is unresolved; no application check started")
        for role in ROLES:
            for suite in ("correctness", "regressions"):
                self.run(role + "-" + suite, self.producer("prepare_property_validation.py", "check", "--build", self.build(role, "property-qa"), "--suite", suite, "--work-dir", self.work / (role + "-" + suite)))
            if role == "correction_reference": continue
            self.run(role + "-allocations", self.producer("prepare_property_validation.py", "check", "--build", self.build(role, "property-allocations"), "--suite", "allocations", "--work-dir", self.work / (role + "-allocations")))
            for kind in CALLABLES:
                self.run(role + "-" + kind, self.producer("prepare_callable_validation.py", "check", "--build", self.build(role, kind), "--out", self.work / (role + "-" + kind + ".json")))
            mixed = read(self.build(role, "property-mixed"))
            ran = self.run(role + "-mixed-check", [mixed["executable"]["path"], "0"])
            spec = importlib.util.spec_from_file_location("frozen_property_gate", self.imported["tooling"]["prepare_property_validation.py"]["path"])
            helper = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(helper)
            rows = helper.timing_rows("mixed", Path(ran["stdout"]["path"]).read_text(), read(mixed["fixture"]["path"])["expectations"]["mixed"])
            self.report.setdefault("mixed_correctness", {})[role] = {"lanes": {name: {key: row[key] for key in ("operations", "checksum")} for name, row in rows.items()}, "scope": "One checksum launch; its raw clock fields are not a paired performance cohort."}
        self.report["complete"] = True

    def measure(self):
        checked = read(self.args.check_result)
        require(checked.get("complete") and checked.get("host_proof", {}).get("complete") and checked["import"] == identity(self.args.import_manifest), "Matching local correctness and ABI checks are required")
        self.report["local_correctness"] = identity(self.args.check_result)
        reference = "reference" if self.args.pair == "acceptance" else "correction_reference"
        suites = self.args.suite or (["bindings", "cold", "setters", "mixed", "receivers", "getters", "keyframes"] if reference == "reference" else ["bindings", "cold"])
        require(reference == "reference" or set(suites) <= {"bindings", "cold"}, "Correction reference provides only the focused QA gates")
        self.report.update(pair=self.args.pair, suites=suites, pairs=31, property_warmups=3, measurements={})
        for suite in suites:
            if suite in ("bindings", "cold", "setters", "mixed"):
                label = "property-mixed" if suite == "mixed" else "property-qa"
                out = self.work / suite
                self.run(suite, self.producer("prepare_property_validation.py", "measure", "--reference-build", self.build(reference, label), "--candidate-build", self.build("candidate", label), "--suite", suite, "--cpu", self.args.cpu, "--pairs", 31, "--warmups", 3, "--work-dir", out))
                result = out / "measurement.json"
            else:
                result = self.work / (suite + ".json")
                self.run(suite, self.producer("prepare_callable_validation.py", "measure", "--reference", self.build(reference, suite), "--candidate", self.build("candidate", suite), "--cpu", self.args.cpu, "--pairs", 31, "--out", result))
            require(read(result).get("complete"), "Incomplete paired measurement")
            self.report["measurements"][suite] = identity(result); self.persist()
        self.report["complete"] = True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    add = commands.add_parser("import", help="verify and relocate data only; never execute")
    add.add_argument("--archive", type=Path, required=True)
    add.add_argument("--archive-sha256", required=True)
    add.add_argument("--archive-bytes", type=int, required=True)
    add.add_argument("--acceptance-import", type=Path, required=True)
    add.add_argument("--correction-import", type=Path, required=True)
    add.add_argument("--output", type=Path, required=True)
    for name in ("check", "measure"):
        item = commands.add_parser(name)
        item.add_argument("--import-manifest", type=Path, required=True)
        item.add_argument("--cpu", type=int, required=True)
        item.add_argument("--output", type=Path, required=True)
        if name == "measure":
            item.add_argument("--check-result", type=Path, required=True)
            item.add_argument("--pair", choices=("acceptance", "correction"), required=True)
            item.add_argument("--suite", choices=("bindings", "cold", "setters", "mixed", "receivers", "getters", "keyframes"), action="append")
    args = parser.parse_args()
    signal.signal(signal.SIGTERM, lambda number, frame: (_ for _ in ()).throw(InterruptedError(f"Signal {number}")))
    owner = Importer(args) if args.command == "import" else Execution(args)
    try:
        if args.command == "import": owner.execute()
        elif args.command == "check": owner.check()
        else: owner.measure()
        if args.command != "import":
            require(identity(args.import_manifest) == owner.report["import"], "Import changed during execution")
            verify_imported(args.import_manifest)
    except BaseException as error:
        report = owner.result if args.command == "import" else owner.report
        report.update(complete=False, error={"type": type(error).__name__, "message": str(error)})
        raise
    finally:
        owner.persist()


if __name__ == "__main__":
    main()
