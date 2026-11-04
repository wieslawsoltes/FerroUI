#!/usr/bin/env python3
"""Inspect selected PGO acceptance in the two imported production Base libraries.

This is a bounded read-only code-generation check, not training or a throughput
result. At most two archive members per arm are disassembled. Current profile
keys and genuine optimized entry metadata are required; historical entry counts
are not acceptance thresholds because optimization can adjust those counts.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time


HOST = "x86_64-unknown-linux-gnu"
# These closure ordinals name the unchanged root_vtable declaration's four
# slots: constructed, core change, effective change, data validation. Resolve
# current symbols from the archive; never copy a historical crate/CGU key.
ROOT_TYPE = r"11root_vtableNtB[0-9A-Za-z]+_11FerroObject"
SELECTORS = {
    "notify_property_changed": r"23notify_property_changed$",
    "root_constructed": ROOT_TYPE + r"E0INt.*9call_once",
    "root_on_property_changed_core": ROOT_TYPE + r"Es_0INt.*9call_once",
    "root_on_property_changed": ROOT_TYPE + r"Es0_0INt.*9call_once",
    "root_update_data_validation": ROOT_TYPE + r"Es1_0INt.*9call_once",
    "styled_constructed": r"11root_vtable.*13StyledElementE0INt.*9call_once",
}


def identity(path: Path) -> dict:
    path = path.resolve(strict=True)
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    return {"path": str(path), "bytes": path.stat().st_size, "sha256": digest}


def read(path: Path) -> dict:
    return json.loads(path.read_text())


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def save(path: Path, value: dict) -> None:
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")
    temporary.replace(path)


def codegen_options(argv: list[str]) -> dict[str, str]:
    options = {}
    iterator = iter(argv)
    for argument in iterator:
        if argument == "-C":
            argument = next(iterator)
        elif argument.startswith("-C"):
            argument = argument[2:]
        else:
            continue
        key, _, value = argument.partition("=")
        options[key] = value
    return options


def definitions(path: Path, archive: Path) -> list[dict]:
    prefix = str(archive) + "["
    rows = []
    with path.open() as stream:
        for line in stream:
            require(line.startswith(prefix), f"Unexpected llvm-nm record: {line[:160]}")
            member, separator, tail = line[len(prefix):].partition("]: ")
            require(bool(separator), "Missing archive member in llvm-nm output")
            fields = tail.rsplit(None, 3)
            require(len(fields) == 4, "Unexpected llvm-nm POSIX fields")
            name, kind, address, size = fields
            rows.append({"member": member, "symbol": name, "kind": kind, "address": address, "size": size})
    return rows


def selected_ir(path: Path, symbols: set[str]) -> dict:
    """Stream a member, retaining selected bodies, root headers and needed metadata."""
    source_filename = None
    functions, metadata, wanted = {}, {}, set()
    active = None
    with path.open() as stream:
        for line in stream:
            if line.startswith("source_filename = "):
                source_filename = json.loads(line.partition(" = ")[2])
            if line.startswith("define "):
                match = re.search(r"@([A-Za-z0-9_.$]+)\(", line)
                if match and (match[1] in symbols or "11root_vtable" in match[1]):
                    name = match[1]
                    require(name not in functions, f"Duplicate LLVM definition {name}")
                    functions[name] = {"definition": line.rstrip(), "body": []}
                    wanted.update(re.findall(r"!(?:prof|PGOFuncName) !(\d+)", line))
                    active = name if name in symbols else None
                else:
                    active = None
            elif active is not None:
                functions[active]["body"].append(line.rstrip())
                wanted.update(re.findall(r"!prof !(\d+)", line))
                if line.strip() == "}":
                    active = None
            elif line.startswith("!"):
                match = re.match(r"!(\d+) = (.*)", line)
                if match and match[1] in wanted:
                    metadata[match[1]] = match[2]
    require(source_filename is not None, "LLVM member omitted source_filename")
    for symbol, row in functions.items():
        header = row["definition"]
        profile_id = re.search(r"!prof !(\d+)", header)
        name_id = re.search(r"!PGOFuncName !(\d+)", header)
        row["entry_metadata"] = metadata.get(profile_id[1]) if profile_id else None
        row["pgo_name_metadata"] = metadata.get(name_id[1]) if name_id else None
        entry = re.match(r'!\{!"function_entry_count", i64 (\d+)(?:,|\})', row["entry_metadata"] or "")
        row["accepted_entry_count"] = int(entry[1]) if entry else None
        explicit = re.fullmatch(r'!\{!"([^"\\]+)"\}', row["pgo_name_metadata"] or "")
        if name_id:
            require(explicit is not None, f"Unrecognized PGOFuncName metadata for {symbol}")
        internal = re.search(r"\b(?:internal|private)\b", header.partition("@")[0]) is not None
        row["key"] = explicit[1] if explicit else (source_filename + ";" + symbol if internal else symbol)
        row["explicit_pgo_name"] = explicit is not None
        row["branch_metadata"] = {key: metadata[key] for key in sorted({key for line in row["body"]
                                  for key in re.findall(r"!prof !(\d+)", line)}) if key in metadata}
    return {"source_filename": source_filename, "functions": functions}


def exact_profile(text: str, key: str) -> dict:
    """llvm-profdata's --function argument is a substring, so check the full key."""
    records = []
    current = None
    for line in text.splitlines():
        match = re.fullmatch(r"  (\S.*):", line)
        if match:
            current = {"key": match[1], "lines": []}
            records.append(current)
        elif current is not None and (line.startswith("    ") or not line):
            current["lines"].append(line)
        else:
            current = None
    matches = [record for record in records if record["key"] == key]
    require(len(matches) == 1, f"Expected one exact current profile key, found {len(matches)}: {key}")
    row = matches[0]
    body = "\n".join(row["lines"])
    cfg = re.search(r"^    Hash: (0x[0-9a-fA-F]+)$", body, re.M)
    counters = re.search(r"^    Counters: (\d+)$", body, re.M)
    counts = re.search(r"^    Block counts: (\[[0-9, ]*\])$", body, re.M)
    require(cfg is not None and counters is not None and counts is not None, "Incomplete exact profile record")
    row.update(cfg_hash=cfg[1], counter_count=int(counters[1]), block_counts=json.loads(counts[1]))
    require(len(row["block_counts"]) == row["counter_count"], "Profile counter vector length differs")
    return row


class Audit:
    def __init__(self, args):
        self.args = args
        self.output = args.output.resolve()
        require(not self.output.exists(), "Use a fresh audit directory; previous evidence is never overwritten")
        self.output.mkdir(parents=True)
        self.import_path = args.import_manifest.resolve(strict=True)
        import run_winui_runtime_comparison
        self.import_helper = run_winui_runtime_comparison
        self.import_record = self.import_helper.verify_import(self.import_path)
        require(self.import_record.get("complete"), "Artifact import must complete before profile inspection")
        self.prefix = args.toolchain_prefix.resolve(strict=True)
        self.llvm = self.prefix / "lib/rustlib" / HOST / "bin"
        clean_environment = {key: value for key, value in os.environ.items() if key not in ("LD_PRELOAD", "LD_AUDIT")}
        self.environment = {**clean_environment, "LD_LIBRARY_PATH": str(self.prefix / "lib"), "LLVM_PROFILE_FILE": os.devnull,
                            "TMPDIR": str(self.output / "temporary")}
        Path(self.environment["TMPDIR"]).mkdir()
        self.report = {"schema_version": 1, "scope": __doc__, "complete": False, "accepted": False,
                       "import_manifest": identity(self.import_path), "producer": identity(Path(__file__)),
                       "import_verifier": identity(Path(self.import_helper.__file__)),
                       "cpu": args.cpu, "started_unix": time.time(), "arms": {}, "commands": [],
                       "removed_environment_names": sorted(set(os.environ) - set(clean_environment)),
                       "limits": {"maximum_disassembled_members_per_arm": 2,
                                  "historical_entry_counts_are_not_thresholds": True,
                                  "throughput_and_final_native_layout": "Not established by this check."}}
        self.tracked = [self.report["import_manifest"], self.report["producer"], self.report["import_verifier"]]
        self.persist()

    def persist(self):
        save(self.output / "result.json", self.report)

    def checked(self, local, original, arm):
        actual = identity(Path(local["path"]))
        require(actual == local, f"Imported file changed: {local['path']}")
        for key in ("sha256", "bytes"):
            if key in original:
                require(actual[key] == original[key], f"Imported bytes differ from original: {original['path']}")
        relocation = self.import_helper.relocation_for(self.import_record, arm, original["path"])
        require(relocation is not None and relocation["local"] == local, "Missing exact import relocation")
        for key, value in original.items():
            require(relocation["original"].get(key) == value, "Original relocation identity differs")
        self.tracked.append(actual)
        return Path(actual["path"])

    def run(self, label, command):
        stdout, stderr = self.output / (label + ".stdout"), self.output / (label + ".stderr")
        require(not stdout.exists() and not stderr.exists(), "Duplicate audit command label")
        row = {"label": label, "argv": list(map(str, command)), "started_unix": time.time()}
        self.report["commands"].append(row)
        self.persist()
        with stdout.open("wb") as output, stderr.open("wb") as error:
            process = subprocess.run(row["argv"], stdout=output, stderr=error, env=self.environment)
        row.update(exit=process.returncode, finished_unix=time.time(), stdout=identity(stdout), stderr=identity(stderr))
        self.persist()
        require(process.returncode == 0, f"Profile audit command failed: {label}; inspect {stderr}")
        return stdout

    def arm(self, name):
        imported = self.import_record["arms"][name]
        original = imported["original"]
        paths = {key: self.checked(imported[key], original[key], name) for key in
                 ("build", "base_manifest", "source_manifest", "compiler_record", "profile", "base", "shipping")}
        build, base, source, compiler = (read(paths[key]) for key in ("build", "base_manifest", "source_manifest", "compiler_record"))
        require(build.get("complete") and build.get("source_files_unchanged") and build.get("tool_identities_unchanged"),
                "Original production build did not complete with stable inputs")
        require(build["base_manifest"] == original["base_manifest"] and build["source"] == original["source_manifest"] and
                build["profile"] == original["profile"] and build["binary"] == original["shipping"], "Original build association differs")
        require(base["base"] == original["base"] and base["profile"] == original["profile"] and
                base["source"] == original["source_manifest"] and base["compiler_record_identity"] == original["compiler_record"] and
                base["graph"] == build["graph"], "Original Base/profile/compiler association differs")
        require(source["head"] == imported["revision"], "Imported source revision differs")
        require(compiler["crate"] == "ferroui_base" and compiler["target"] == HOST and compiler["returncode"] == 0,
                "This is not a completed production Base compiler record")
        options = codegen_options(compiler["argv"])
        require(options.get("profile-use") == original["profile"]["path"] and options.get("opt-level") == "z" and
                "profile-generate" not in options and options.get("linker-plugin-lto") in ("", "yes"),
                "Production Base did not use the matched profile/optimization policy")
        tool_rows = {}
        selected_tools = {tool: self.llvm / tool for tool in ("llvm-nm", "llvm-ar", "llvm-dis", "llvm-profdata")}
        selected_tools.update({tool: self.prefix / "lib" / tool for tool in build["tools"]
                               if tool.startswith(("libLLVM", "librustc_driver"))})
        require(any(tool.startswith("libLLVM") for tool in selected_tools), "Original LLVM shared-library identity is absent")
        for tool, path in selected_tools.items():
            actual = identity(path)
            require(all(actual[key] == build["tools"][tool][key] for key in ("sha256", "bytes")),
                    f"LLVM tool differs from production build: {tool}")
            self.tracked.append(actual)
            tool_rows[tool] = actual
        row = {"revision": imported["revision"], "inputs": {key: identity(path) for key, path in paths.items()},
               "tools": tool_rows, "compiler_codegen_options": options, "complete": False, "accepted": False}
        self.report["arms"][name] = row
        self.persist()
        inventory_path = self.run(name + "-definitions", [self.llvm / "llvm-nm", "--format=posix", "--print-file-name",
                                  "--defined-only", "--no-sort", paths["base"]])
        inventory = definitions(inventory_path, paths["base"])
        selected = {}
        for label, pattern in SELECTORS.items():
            matches = [entry for entry in inventory if re.search(pattern, entry["symbol"])]
            require(len(matches) == 1, f"Resolve {label} before inspection: found {len(matches)} current definitions")
            selected[label] = matches[0]
        members = sorted({entry["member"] for entry in selected.values()})
        row.update(selected_definitions=selected, selected_members=members)
        self.persist()
        require(len(members) <= 2, "Selected functions exceed the two-member audit bound; review selection before broadening")
        functions = {}
        row["members"] = []
        for index, member in enumerate(members):
            bitcode = self.run(f"{name}-member-{index}", [self.llvm / "llvm-ar", "p", paths["base"], member])
            ir = self.output / f"{name}-member-{index}.ll"
            self.run(f"{name}-disassemble-{index}", [self.llvm / "llvm-dis", bitcode, "-o", ir])
            parsed = selected_ir(ir, {entry["symbol"] for entry in selected.values()})
            row["members"].append({"member": member, "bitcode": identity(bitcode), "ir": identity(ir),
                                   "source_filename": parsed["source_filename"]})
            for symbol, data in parsed["functions"].items():
                require(symbol not in functions, f"Duplicate selected/root LLVM symbol across members: {symbol}")
                functions[symbol] = {"member": member, **data}
        roots = {key: value for key, value in functions.items() if "11root_vtable" in key}
        row["root_definition_summary_in_selected_members"] = {
            "definitions": len(roots), "accepted_entry_metadata": sum(x["accepted_entry_count"] is not None for x in roots.values()),
            "accepted_zero_entries": sum(x["accepted_entry_count"] == 0 for x in roots.values()),
            "explicit_pgo_names": sum(x["explicit_pgo_name"] for x in roots.values()),
            "entries": {key: {field: value[field] for field in ("member", "key", "accepted_entry_count", "explicit_pgo_name")}
                        for key, value in roots.items()},
        }
        row["selected_functions"] = {}
        for index, (label, entry) in enumerate(selected.items()):
            require(entry["symbol"] in functions, f"Selected symbol is not an LLVM definition: {label}")
            function = functions[entry["symbol"]]
            query = self.run(f"{name}-profile-{index}", [self.llvm / "llvm-profdata", "show", "--counts", "--ic-targets",
                             "--function=" + function["key"], paths["profile"]])
            raw = exact_profile(query.read_text(), function["key"])
            accepted = function["accepted_entry_count"] is not None
            if label != "root_update_data_validation":
                accepted = accepted and function["accepted_entry_count"] > 0 and any(raw["block_counts"])
            row["selected_functions"][label] = {"symbol": entry["symbol"], **function, "raw_profile": raw,
                                                "accepted": accepted, "query": identity(query)}
            self.persist()
        notify = row["selected_functions"]["notify_property_changed"]
        row["notify_root_target_comparisons"] = [line for line in notify["body"]
                                                  if "icmp eq ptr" in line and "11root_vtable" in line]
        row.update(complete=True, accepted=all(value["accepted"] for value in row["selected_functions"].values()))
        self.persist()

    def execute(self):
        for name in ("reference", "candidate"):
            self.arm(name)
        for expected in self.tracked:
            require(identity(Path(expected["path"])) == expected, f"Input changed during profile inspection: {expected['path']}")
        self.report.update(complete=True, accepted=all(row["accepted"] for row in self.report["arms"].values()),
                           input_identities_unchanged=True, finished_unix=time.time())
        self.persist()
        require(self.report["accepted"], "One or more selected current profile keys lack accepted optimized entry metadata")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--import-manifest", type=Path, required=True)
    parser.add_argument("--toolchain-prefix", type=Path, required=True)
    parser.add_argument("--cpu", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    affinity = os.sched_getaffinity(0)
    require(args.cpu in affinity, "Choose an allowed CPU")
    audit = Audit(args)
    try:
        os.sched_setaffinity(0, {args.cpu})
        audit.execute()
    except BaseException as error:
        audit.report.update(error={"type": type(error).__name__, "message": str(error)}, finished_unix=time.time())
        audit.persist()
        raise
    finally:
        os.sched_setaffinity(0, affinity)
    print(json.dumps({"complete": audit.report["complete"], "accepted": audit.report["accepted"],
                      "result": str(audit.output / "result.json")}, sort_keys=True))


if __name__ == "__main__":
    main()
