#!/usr/bin/env python3
"""Freeze generated artifacts and a section-verified symbol twin after a PGO replay.

The symbol twin reuses captured native inputs. It does not invoke rustc or alter
the shipping binary. Run before reusing the disposable checkout/target directory.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import time

from attribute_codegen import Elf
from rebuild_winui_pgo import digest, identity, save, verify_source_files


def allocated_sections(path: Path) -> dict:
    elf = Elf(path)
    try:
        return {
            elf.name(elf.section_names + section[0]).decode(): {
                "type": section[1], "flags": section[2], "address": section[3],
                "allocated_bytes": section[5], "file_bytes": 0 if section[1] == 8 else section[5],
                "sha256": None if section[1] == 8 else
                hashlib.sha256(elf.data[section[4]:section[4] + section[5]]).hexdigest(),
            }
            for section in elf.sections() if section[2] & 2
        }
    finally:
        elf.data.close()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build", type=Path, required=True)
    parser.add_argument("--attribute", action="store_true")
    args = parser.parse_args()
    build_path = args.build.resolve(strict=True)
    work = build_path.parent
    report_path = work / "artifacts.json"
    if report_path.exists():
        raise RuntimeError("Artifact collection requires a fresh report; existing evidence is preserved")
    build = json.loads(build_path.read_text())
    if not build.get("complete"):
        raise RuntimeError("The whole-graph PGO build has not completed")
    source_manifest = Path(build["source"]["path"])
    if identity(source_manifest) != build["source"]:
        raise RuntimeError("Frozen source manifest identity changed")
    source_record = json.loads(source_manifest.read_text())
    source = Path(source_record["directory"])
    verify_source_files(source, source_record["files"])
    compile_source = Path(source_record["compile_directory"])
    if subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=compile_source, text=True).strip() != source_record["head"]:
        raise RuntimeError("Collect generated outputs before switching the shared source checkout")
    verify_source_files(compile_source, source_record["files"])
    if identity(Path(build["profile"]["path"])) != build["profile"]:
        raise RuntimeError("Matched PGO profile changed")
    base_manifest = Path(build["base_manifest"]["path"])
    if identity(base_manifest) != build["base_manifest"]:
        raise RuntimeError("Base/graph manifest changed")
    base = json.loads(base_manifest.read_text())
    if base["profile"] != build["profile"] or base["graph"] != build["graph"]:
        raise RuntimeError("Base graph is not associated with this profile-use build")
    if identity(Path(base["base"]["path"])) != base["base"]:
        raise RuntimeError("Base archive identity changed")
    for directory, expected_inventory in [(Path(base["graph"]["path"]), base["graph"]["inventory_sha256"]),
                                          (Path(base["host_dependencies"]), base["host_inventory_sha256"])]:
        inventory = directory / "hashes.json"
        if digest(inventory) != expected_inventory:
            raise RuntimeError(f"Dependency inventory changed: {inventory}")
        for name, expected_hash in json.loads(inventory.read_text()).items():
            if digest(directory / name) != expected_hash:
                raise RuntimeError(f"Dependency artifact changed: {directory / name}")
    for item in build["tools"].values():
        if identity(Path(item["path"])) != item:
            raise RuntimeError(f"Selected compiler/linker tool changed: {item['path']}")
    for name in ("attribute_codegen.py", "attribute_runtime.py"):
        if digest(Path(__file__).with_name(name)) != build["helper_inputs"][name]["sha256"]:
            raise RuntimeError(f"Attribution helper differs from the frozen build: {name}")
    shipping = Path(build["binary"]["path"])
    if identity(shipping) != build["binary"]:
        raise RuntimeError("Shipping binary identity changed")
    report = {"scope": __doc__, "build": identity(build_path), "producer": identity(Path(__file__)),
              "shipping": identity(shipping), "input_identities_verified": True, "complete": False}
    save(report_path, report)
    records = []
    for path in (work / "compilers/optimized").glob("*.json"):
        row = json.loads(path.read_text())
        if row["crate"] == "ferroui_themes_winui" and row["target"] == "x86_64-unknown-linux-gnu" and row.get("returncode") == 0:
            records.append((path, row))
    if len(records) != 1:
        raise RuntimeError(f"Expected one completed optimized WinUI compiler, found {len(records)}")
    compiler_path, compiler = records[0]
    generated = Path(compiler["environment"]["OUT_DIR"])
    if not (generated / "xaml/compiled_xaml.rs").is_file():
        raise RuntimeError("WinUI compiler OUT_DIR has no compiled XAML source")
    snapshot = work / "generated"
    shutil.copytree(generated, snapshot)
    files = {str(path.relative_to(snapshot)): {"bytes": path.stat().st_size, "sha256": digest(path)}
             for path in sorted(snapshot.rglob("*")) if path.is_file()}
    save(snapshot / "hashes.json", files)
    documents = {str(path.relative_to(source)): identity(path) for path in
                 sorted((source / "src/FerroUI.Themes.WinUI").rglob("*.xaml")) if path.is_file() and not path.is_symlink()}
    save(work / "theme-documents.json", documents)
    report["generated"] = {
        "directory": str(snapshot), "original_directory": str(generated),
        "inventory": identity(snapshot / "hashes.json"), "compiler_record": identity(compiler_path),
        "source_documents": identity(work / "theme-documents.json"), "source_document_count": len(documents),
        "counts": {suffix: sum(name.endswith(suffix) for name in files) for suffix in (".rs", ".fxb", ".fxp")},
        "bytes": {suffix: sum(row["bytes"] for name, row in files.items() if name.endswith(suffix))
                  for suffix in (".rs", ".fxb", ".fxp")},
    }
    report["sections"] = allocated_sections(shipping)
    report["compact_unwind_applied"] = ".cfi_reserve" in report["sections"]
    save(report_path, report)
    if not report["compact_unwind_applied"]:
        raise RuntimeError("Shipping binary did not receive the required compact unwind transform")

    captured = [(path, json.loads(path.read_text())) for path in sorted((work / "native-link").glob("[0-9][0-9][0-9].json"))]
    matches = [(path, row) for path, row in captured
               if row.get("returncode") == 0 and row.get("output", {}).get("sha256") == build["binary"]["sha256"]]
    if not matches:
        raise RuntimeError("No captured native link output matches the shipping binary")
    path, native = matches[-1]
    command = native["command"].copy()
    for item in native["captured_inputs"]:
        saved = Path(item["captured"]["path"])
        if identity(saved) != item["captured"]:
            raise RuntimeError(f"Captured native input changed: {saved}")
        command[item["argument_index"] + 1] = str(saved)
    command = [argument for argument in command if argument not in ("-s", "-Wl,--strip-all", "-Wl,--strip-debug")]
    symbolized = work / "winui-theme-probe.symbolized"
    command[command.index("-o") + 1] = str(symbolized)
    environment = {**os.environ, **build["controlled_environment"], "LLVM_PROFILE_FILE": os.devnull}
    environment["TMPDIR"] = str(work / "symbol-tmp")
    Path(environment["TMPDIR"]).mkdir()
    os.sched_setaffinity(0, set(build["cpu_affinity"]))
    link = {"captured_record": identity(path), "argv": command,
            "cwd": str(work), "started_unix": time.time(), "log": str(work / "symbol-link.log")}
    report["symbol_link"] = link
    save(report_path, report)
    start = time.monotonic()
    with Path(link["log"]).open("w") as log:
        process = subprocess.run(command, cwd=work, env=environment, stdout=log, stderr=log)
    link.update(returncode=process.returncode, elapsed_seconds=time.monotonic() - start,
                log_identity=identity(Path(link["log"])))
    save(report_path, report)
    process.check_returncode()
    actual = allocated_sections(symbolized)
    expected = report["sections"]
    # LLD's build ID can include the intentionally retained symbol table.
    mismatches = [name for name in sorted(set(actual) | set(expected))
                  if name != ".note.gnu.build-id" and actual.get(name) != expected.get(name)]
    with shipping.open("rb") as stream:
        shipping_entry = struct.unpack_from("<Q", stream.read(32), 24)[0]
    with symbolized.open("rb") as stream:
        symbol_entry = struct.unpack_from("<Q", stream.read(32), 24)[0]
    report["symbolized"] = identity(symbolized)
    report["symbolized_allocated_sections"] = actual
    report["symbolized_section_mismatches"] = mismatches
    report["entry_points"] = {"shipping": shipping_entry, "symbolized": symbol_entry}
    save(report_path, report)
    if mismatches or shipping_entry != symbol_entry:
        raise RuntimeError(f"Symbol twin does not reproduce shipping allocated sections: {mismatches}")
    ordinary = [item for _, row in captured for item in row["captured_inputs"]
                if item["original"]["path"].endswith(".rcgu.o")]
    by_hash = {item["captured"]["sha256"]: item["captured"] for item in ordinary}
    if len(by_hash) != 1:
        raise RuntimeError("Expected exactly one ordinary FatLTO native object")
    report["ordinary_fatlto_object"] = next(iter(by_hash.values()))
    report["native_link_records"] = [identity(path) for path, _ in captured]
    if args.attribute:
        command = [sys.executable, str(Path(__file__).with_name("attribute_codegen.py")),
                   "--shipping", str(shipping), "--symbolized", str(symbolized),
                   "--object", report["ordinary_fatlto_object"]["path"],
                   "--profile", build["profile"]["path"],
                   "--llvm-profdata", str(Path(build["controlled_environment"]["RUSTC"]).parents[1] /
                                           "lib/rustlib/x86_64-unknown-linux-gnu/bin/llvm-profdata"),
                   "--output", str(work / "attribution.json")]
        archives = list(snapshot.rglob("*.fxp"))
        if len(archives) == 1:
            command += ["--embedded-archive", str(archives[0])]
        report["attribution_command"] = command
        save(report_path, report)
        with (work / "attribution.log").open("w") as log:
            process = subprocess.run(command, env=environment, stdout=log, stderr=log)
        report["attribution_exit"] = process.returncode
        save(report_path, report)
        process.check_returncode()
        report["attribution"] = identity(work / "attribution.json")
    if identity(shipping) != build["binary"]:
        raise RuntimeError("Artifact collection changed the shipping binary")
    report.update(complete=True, finished_unix=time.time())
    save(report_path, report)
    print(json.dumps({key: report[key] for key in ("shipping", "generated", "symbolized", "compact_unwind_applied")}, indent=2))


if __name__ == "__main__":
    main()
