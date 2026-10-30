#!/usr/bin/env python3
"""Pinned LiveCharts2 source, API and test-declaration inventory.

It reads a LiveCharts2 checkout (`--upstream`): the Git objects at the commit of
`docs/porting/livecharts2/sources.lock.json`, never working-tree files, and runs
FerroUI's Roslyn syntax extractor (`scripts/api-extract`, .NET 10) on a temporary
export of that commit. Committed (docs/porting/winui-controls.md, O8; the W10b
pattern): the pin, the reviewed `mapping.json`, and the generated `summary.json`
and `REPORT.md`. The inventories and maps (`inventory.json`, `upstream-api.json`,
`source-map.json`, `api-map.json`, `test-map.json`) are written to `--output-dir`,
an artifact of the workflow. `--check` fails when the committed report is stale.

This is an accountability check, not a claim of behavioral parity. Pending and
partial dispositions remain visible; --require-complete rejects either.
"""

from __future__ import annotations

import argparse
from collections import Counter
from copy import deepcopy
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile


REPO = Path(__file__).resolve().parents[2]
DATA = REPO / "docs/porting/livecharts2"
OUTPUT = REPO / "target/livecharts2-scan"
STATUSES = {"pending", "partial", "ported", "native-replacement", "not-applicable"}
TEST_ATTRIBUTES = {"TestMethod", "DataTestMethod", "AppTestMethod", "Fact", "Theory", "Test", "TestCase"}


def read_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def encoded(value) -> str:
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def extraction_input_hash(path: Path) -> str:
    # Git's Windows checkout may expand LF to CRLF. The extractor inputs are
    # UTF-8 text, so checkout line endings must not look like a source change.
    # Pinned upstream inventory hashes still cover the exact Git object bytes.
    return sha256(path.read_bytes().replace(b"\r\n", b"\n"))


def git_archive(upstream: Path, reference: dict) -> bytes:
    roots = reference["source_roots"] + reference["test_roots"] + reference["sample_roots"]
    roots += reference["extra_files"]
    return subprocess.check_output(
        ["git", "-C", str(upstream), "archive", reference["commit"], "--", *roots]
    )


def section(path: str, reference: dict) -> str:
    for key, value in (("source_roots", "source"), ("test_roots", "test"), ("sample_roots", "sample")):
        if any(path.startswith(root + "/") for root in reference[key]):
            return value
    return "reference"


def inventory(archive: bytes, reference: dict) -> dict:
    records = []
    with tarfile.open(fileobj=io.BytesIO(archive)) as source:
        for member in source.getmembers():
            if not member.isfile():
                continue
            data = source.extractfile(member).read()
            record = {
                "path": member.name,
                "section": section(member.name, reference),
                "sha256": sha256(data),
                "bytes": len(data),
            }
            try:
                record["lines"] = len(data.decode("utf-8-sig").splitlines())
            except UnicodeDecodeError:
                record["binary"] = True
            records.append(record)
    return {"schema": 1, "commit": reference["commit"], "files": sorted(records, key=lambda r: r["path"])}


def extract_api(archive: bytes, reference: dict, dotnet: str) -> dict:
    with tempfile.TemporaryDirectory(prefix="ferroui-livecharts2-") as temporary:
        root = Path(temporary)
        export, output = root / "reference", root / "output"
        export.mkdir()
        with tarfile.open(fileobj=io.BytesIO(archive)) as source:
            # Git supplies all paths; data filtering also prevents traversal and symlinks.
            source.extractall(export, filter="data")
        project = REPO / "scripts/livecharts2/ApiExtract.csproj"
        subprocess.run([dotnet, "build", str(project), "-c", "Release", "--nologo", "-v:q"], check=True)
        assembly = project.parent / "bin/Release/net10.0/ApiExtract.dll"
        config = read_json(REPO / "scripts/livecharts2/projects.json")
        profiles = read_json(REPO / "scripts/livecharts2/profiles.json")
        combined = None
        profile_records = []
        for index, (name, symbols) in enumerate(profiles["profiles"].items()):
            bit = 1 << index
            common = [] if "NET462" in symbols else profiles["common"]
            config["preprocessorSymbols"] = sorted(set(common + symbols))
            profile_config = root / "projects.json"
            profile_config.write_text(encoded(config), encoding="utf-8")
            subprocess.run(
                [dotnet, str(assembly), "--upstream", str(export),
                 "--config", str(profile_config), "--out", str(output),
                 "--commit", reference["commit"]], check=True,
            )
            current = read_json(output / "upstream-api.json")
            profile_records.append({"name": name, "mask": bit, "symbols": config["preprocessorSymbols"]})
            if combined is None:
                combined = current
                for p in combined["projects"]:
                    for file in p["files"]:
                        for ty in all_types(file.get("types", [])):
                            ty["profile_mask"] = bit
                            for member in ty.get("members", []):
                                member["profile_mask"] = bit
            else:
                merge_api(combined, current, bit)
        combined["profiles"] = profile_records
        combined["extraction"] = {
            "kind": "Roslyn syntax union; no semantic compilation or generated sources",
            "extractor_sha256": extraction_input_hash(REPO / "scripts/api-extract/Program.cs"),
            "projects_sha256": extraction_input_hash(REPO / "scripts/livecharts2/projects.json"),
            "profiles_sha256": extraction_input_hash(REPO / "scripts/livecharts2/profiles.json"),
        }
        return combined


def merge_api(combined: dict, current: dict, bit: int) -> None:
    projects = {p["path"]: p for p in combined["projects"]}
    for project in current["projects"]:
        target_project = projects[project["path"]]
        files = {f["path"]: f for f in target_project["files"]}
        for file in project["files"]:
            if file["path"] not in files:
                target_project["files"].append(deepcopy(file))
                files[file["path"]] = target_project["files"][-1]
                for ty in all_types(files[file["path"]].get("types", [])):
                    ty["profile_mask"] = bit
                    for member in ty.get("members", []):
                        member["profile_mask"] = bit
                continue
            merge_types(files[file["path"]].setdefault("types", []), file.get("types", []), bit)
        target_project["files"].sort(key=lambda f: f["path"])


def merge_types(target: list, current: list, bit: int) -> None:
    types = {(t["fullName"], t["kind"], t["line"]): t for t in target}
    for ty in current:
        key = (ty["fullName"], ty["kind"], ty["line"])
        if key not in types:
            clone = deepcopy(ty)
            target.append(clone)
            for nested in all_types([clone]):
                nested["profile_mask"] = bit
                for member in nested.get("members", []):
                    member["profile_mask"] = bit
            continue
        existing = types[key]
        merge_conditional_declaration(existing, ty, bit, {"members", "nestedTypes"})
        existing["profile_mask"] |= bit
        members = {(m["kind"], m.get("signature", m["name"]), m.get("line")): m
                   for m in existing.get("members", [])}
        for member in ty.get("members", []):
            member_key = (member["kind"], member.get("signature", member["name"]), member.get("line"))
            if member_key in members:
                merge_conditional_declaration(members[member_key], member, bit, set())
                members[member_key]["profile_mask"] |= bit
            else:
                clone = deepcopy(member)
                clone["profile_mask"] = bit
                existing.setdefault("members", []).append(clone)
        merge_types(existing.setdefault("nestedTypes", []), ty.get("nestedTypes", []), bit)
        existing.get("members", []).sort(key=lambda m: (m.get("line", 0), m["kind"], m["name"]))
    target.sort(key=lambda t: (t["line"], t["fullName"]))


def merge_conditional_declaration(existing: dict, incoming: dict, bit: int, exclude: set[str]) -> None:
    ignored = exclude | {"profile_mask", "conditional_declarations"}
    first = {k: deepcopy(v) for k, v in existing.items() if k not in ignored}
    declaration = {k: deepcopy(v) for k, v in incoming.items() if k not in ignored}
    if first == declaration and "conditional_declarations" not in existing:
        return
    variants = existing.setdefault("conditional_declarations", [
        {"profile_mask": existing["profile_mask"], "declaration": first}
    ])
    for variant in variants:
        if variant["declaration"] == declaration:
            variant["profile_mask"] |= bit
            break
    else:
        variants.append({"profile_mask": bit, "declaration": declaration})


def all_types(types):
    for item in types:
        yield item
        yield from all_types(item.get("nestedTypes", []))


def family(path: str) -> str:
    if path.startswith("generators/"):
        return "Source generation and build metadata"
    if path.startswith("build/") or not path.endswith(".cs"):
        return "Resources and build configuration"
    if path.startswith("src/_Shared.Native/"):
        return "Native host services"
    if path.startswith("src/skiasharp/"):
        if path.startswith("src/skiasharp/LiveChartsCore.SkiaSharp/"):
            return "Drawing backend and concrete series"
        return "UI controls and platform adapters"
    sub = path.removeprefix("src/LiveChartsCore/").split("/")[0]
    if sub.endswith(".cs"):
        return "Chart engines, axes and series"
    return {
        "Kernel": "Kernel, data, interaction and observers",
        "Motion": "Motion and frame scheduling",
        "Drawing": "Drawing abstractions and geometry",
        "Geo": "Geography and map data",
        "Measure": "Measurement, scales and policies",
        "Defaults": "Observable data models",
        "Easing": "Easing functions",
        "Themes": "Themes and palettes",
        "Generators": "Source generation and build metadata",
        "VisualElements": "Visual annotations and layouts",
        "VisualStates": "Visual states and conditional styling",
        "ConditionalDraw": "Visual states and conditional styling",
    }.get(sub, sub)


def disposition(override: dict | None, *, test: bool = False) -> dict:
    if not override:
        return {
            "status": "pending",
            "rust": [],
            "remaining": ["No reviewed Rust equivalent is recorded." if not test else
                          "No independently verified Rust test counterpart is recorded."],
        }
    result = dict(override)
    if result.get("status") not in STATUSES:
        raise ValueError(f"Invalid disposition: {result!r}")
    result.setdefault("rust", [])
    result.setdefault("remaining", [])
    if not result.get("reason"):
        raise ValueError(f"A reviewed disposition requires a reason: {result!r}")
    if result["status"] in {"ported", "native-replacement"} and not result["rust"]:
        raise ValueError(f"Implemented disposition has no Rust evidence: {result!r}")
    if result["status"] in {"pending", "partial"} and not result["remaining"]:
        raise ValueError(f"Incomplete disposition must name remaining work: {result!r}")
    for target in result["rust"]:
        path = REPO / target.split("::", 1)[0]
        if not path.is_file():
            raise ValueError(f"Rust mapping does not exist: {target}")
    return result


def generated_records(source_inventory: dict, api: dict, mapping: dict) -> dict[str, dict]:
    source_files = [f for f in source_inventory["files"] if f["section"] == "source"]
    source_paths = {f["path"] for f in source_files}
    unknown = set(mapping["files"]) - source_paths
    if unknown:
        raise ValueError(f"Mappings name unknown upstream files: {sorted(unknown)}")
    source_map = []
    for source in source_files:
        source_map.append({
            "source": source["path"], "family": family(source["path"]),
            **disposition(mapping["files"].get(source["path"])),
        })
    tests, members, type_count = [], [], 0
    extracted_files = set()
    for project in api["projects"]:
        for file in project["files"]:
            path = project["path"] + "/" + file["path"]
            extracted_files.add(path)
            for ty in all_types(file.get("types", [])):
                if path in source_paths:
                    type_count += 1
                for member in ty.get("members", []):
                    identifier = f'{path}::{ty["fullName"]}::{member.get("signature", member["name"])}'
                    if member.get("line") is not None:
                        identifier += f'#L{member["line"]}'
                    attributes = member.get("attributes", [])
                    attr_names = {re.split(r"[.(]", a)[0] for a in attributes}
                    if path.startswith("tests/") and attr_names & TEST_ATTRIBUTES:
                        tests.append({
                            "id": identifier, "source": path, "type": ty["fullName"],
                            "method": member["name"], "line": member.get("line"),
                            "attributes": attributes, "profile_mask": member.get("profile_mask"),
                            **disposition(mapping["tests"].get(identifier), test=True),
                        })
                    elif path in source_paths:
                        # File-level adaptation must not silently certify all its members.
                        members.append({
                            "id": identifier, "source": path, "type": ty["fullName"],
                            "kind": member["kind"], "name": member["name"],
                            "signature": member.get("signature", member["name"]),
                            "line": member.get("line"), "profile_mask": member.get("profile_mask"),
                            **disposition(mapping.get("members", {}).get(identifier)),
                        })
    expected_cs = {f["path"] for f in source_inventory["files"]
                   if f["section"] in {"source", "test"} and f["path"].endswith(".cs")}
    missing = expected_cs - extracted_files
    if missing:
        raise ValueError(f"Roslyn project config omits tracked C# files: {sorted(missing)}")
    unknown_tests = set(mapping["tests"]) - {t["id"] for t in tests}
    unknown_members = set(mapping.get("members", {})) - {m["id"] for m in members}
    if unknown_tests or unknown_members:
        raise ValueError(f"Mappings name unknown declarations: {sorted(unknown_tests | unknown_members)}")
    for records in (members, tests):
        if len(records) != len({record["id"] for record in records}):
            raise ValueError("Duplicate API or test declaration identities")
    prefix = {"schema": 1, "commit": source_inventory["commit"]}
    summary = {
        **prefix,
        "source_files": len(source_files),
        "source_csharp_files": sum(f["path"].endswith(".cs") for f in source_files),
        "source_csharp_lines": sum(f.get("lines", 0) for f in source_files if f["path"].endswith(".cs")),
        "source_types": type_count,
        "source_members": len(members),
        "source_status": dict(sorted(Counter(s["status"] for s in source_map).items())),
        "member_status": dict(sorted(Counter(m["status"] for m in members).items())),
        "test_declarations": len(tests),
        "test_status": dict(sorted(Counter(t["status"] for t in tests).items())),
        "test_declarations_by_suite": dict(sorted(Counter(t["source"].split("/")[1] for t in tests).items())),
        "sample_files": sum(f["section"] == "sample" for f in source_inventory["files"]),
        "sample_csharp_files": sum(f["section"] == "sample" and f["path"].endswith(".cs") for f in source_inventory["files"]),
        "interpretation": "Counts are source/API/test declarations, not executed tests or behavioral equivalence. Parameterized methods count once; their attributes are retained. A file adaptation never certifies every member.",
    }
    return {
        "inventory.json": source_inventory,
        "source-map.json": {**prefix, "files": source_map},
        "api-map.json": {**prefix, "members": sorted(members, key=lambda m: m["id"])},
        "test-map.json": {**prefix, "tests": sorted(tests, key=lambda t: t["id"])},
        "summary.json": summary,
    }


def report(summary: dict, reference: dict) -> str:
    """The committed human summary of a scan."""
    def table(title: str, counts: dict) -> list[str]:
        rows = [f"### {title}", "", "| Disposition | Count |", "|---|---:|"]
        rows += [f"| {key} | {value} |" for key, value in counts.items()]
        return rows + [""]
    lines = [
        "# LiveCharts2 port: source, API and test accounting", "",
        "Generated by `python3 scripts/livecharts2/inventory.py` from the pinned checkout. Do not edit.", "",
        f"Source: [{reference['repository']}]({reference['repository']}/tree/{reference['commit']}) "
        f"at `{reference['commit'][:12]}` ({reference['license']}, {reference['copyright']}).", "",
        summary["interpretation"], "",
        "## Totals", "", "| Measure | Count |", "|---|---:|",
        f"| Source files | {summary['source_files']} |",
        f"| C# source files | {summary['source_csharp_files']} |",
        f"| C# source lines | {summary['source_csharp_lines']} |",
        f"| Source types | {summary['source_types']} |",
        f"| Source API members | {summary['source_members']} |",
        f"| Test declarations | {summary['test_declarations']} |",
        f"| Sample files | {summary['sample_files']} |", "",
        "## Dispositions", "",
        *table("Source files", summary["source_status"]),
        *table("API members", summary["member_status"]),
        *table("Test declarations", summary["test_status"]),
        "## Test declarations by suite", "", "| Suite | Count |", "|---|---:|",
        *[f"| {key} | {value} |" for key, value in summary["test_declarations_by_suite"].items()], "",
        "The dispositions come from the reviewed `mapping.json`: a `ported` or `native-replacement` entry",
        "names the Rust files that implement it, which must exist; everything else is `pending`.",
    ]
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", type=Path, required=True,
                        help="a LiveCharts2 checkout that has the pinned commit")
    parser.add_argument("--dotnet", default="dotnet")
    parser.add_argument("--output-dir", type=Path, default=OUTPUT,
                        help="where the inventories and maps are written (not committed)")
    parser.add_argument("--check", action="store_true",
                        help="fail when the committed summary.json or REPORT.md differs from this scan")
    parser.add_argument("--require-complete", action="store_true")
    args = parser.parse_args()
    reference, mapping = read_json(DATA / "sources.lock.json"), read_json(DATA / "mapping.json")
    archive = git_archive(args.upstream.resolve(), reference)
    source_inventory = inventory(archive, reference)
    api = extract_api(archive, reference, args.dotnet)
    if source_inventory["commit"] != reference["commit"] or api["upstreamCommit"] != reference["commit"]:
        raise ValueError("Inventory and API must refer to the exact pinned commit")
    paths = [f["path"] for f in source_inventory["files"]]
    if len(paths) != len(set(paths)):
        raise ValueError("Duplicate source inventory paths")
    generated = generated_records(source_inventory, api, mapping)
    generated["upstream-api.json"] = api
    summary = generated.pop("summary.json")
    output = args.output_dir
    output.mkdir(parents=True, exist_ok=True)
    for filename, value in generated.items():
        (output / filename).write_text(encoded(value), encoding="utf-8")
    committed = {"summary.json": encoded(summary), "REPORT.md": report(summary, reference)}
    stale = []
    for filename, content in committed.items():
        target = DATA / filename
        if args.check:
            if not target.exists() or target.read_text(encoding="utf-8") != content:
                stale.append(filename)
                (output / filename).write_text(content, encoding="utf-8")
        else:
            target.write_text(content, encoding="utf-8")
    print(f'LiveCharts2 {reference["commit"]}: {summary["source_files"]} source files, '
          f'{summary["source_members"]} API members, {summary["test_declarations"]} test declarations.')
    print(f'Files: {summary["source_status"]}; API: {summary["member_status"]}; tests: {summary["test_status"]}.')
    if stale:
        print("Stale committed reports (the fresh ones are in the output directory): " + ", ".join(stale))
        return 1
    if args.require_complete and any(row["status"] in {"pending", "partial"}
                                     for key, field in (("source-map.json", "files"), ("api-map.json", "members"), ("test-map.json", "tests"))
                                     for row in generated[key][field]):
        print("Full port gate failed: incomplete dispositions remain.")
        return 1
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"LiveCharts2 inventory error: {error}") from error
