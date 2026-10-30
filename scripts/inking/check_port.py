#!/usr/bin/env python3
"""Check the pinned inking source ledger, API coverage and local evidence paths.

This checks bookkeeping, not behavior or visual parity. An `adapted` record
describes a deliberate implementation difference; it is not a Windows runtime
conformance result. `--require-complete` rejects every recorded gap, including
the separately labeled optional Windows dependency APIs. The narrower
`--require-control-complete` checks the pinned control scope and direct services.
"""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import sys

STATUSES = {"implemented", "adapted", "partial", "not_ported", "not_applicable", "external"}
CODE_STATUSES = {"implemented", "adapted", "partial"}
GAPS = {"partial", "not_ported", "external"}
COMMIT = "7e4b0c2096be402f51cc8f7d203ebf67cff183d0"
WINDOWS_COMMIT = "32c3144490c016fe496a0aed769bce60987a2e9d"
WINDOWS_CONTRACT_SHA256 = "358cd1f51721ad240af28f80d4a3a825c757eb1fd3d87c8f1dc716f3184e974c"
SERVICE_ROLES = {"required_by_controls", "returned_service_api", "portable_engine_support", "optional_extension"}


def idl_contract(text: str) -> dict[str, dict[str, tuple[str, int | None]]]:
    """Read the small checked-in IDL surface, retaining enum values and members."""
    text = re.sub(r"//[^\n]*", "", text)
    text = re.sub(r"\[[^\]]*\]", "", text)
    index = 0
    result = {}
    pattern = re.compile(r"\b(runtimeclass|enum)\s+(\w+)\s*[^\{]*\{")
    while match := pattern.search(text, index):
        kind, type_name = match.group(1), match.group(2)
        depth, end = 1, match.end()
        while depth and end < len(text):
            if text[end] == "{":
                depth += 1
            elif text[end] == "}":
                depth -= 1
            end += 1
        if depth:
            raise ValueError(f"Unclosed IDL declaration: {type_name}")
        body = text[match.end():end - 1]
        members = {}
        if kind == "enum":
            value = 0
            for raw in body.split(","):
                pieces = raw.strip().split("=")
                if not pieces[0]:
                    continue
                if len(pieces) == 2:
                    value = int(pieces[1].strip())
                members[pieces[0].strip()] = ("enum_variant", value)
                value += 1
        else:
            start = depth = 0
            for offset, char in enumerate(body):
                if char == "{":
                    depth += 1
                elif char == "}":
                    depth -= 1
                elif char == ";" and depth == 0:
                    raw = re.sub(r"\s+", " ", body[start:offset + 1].strip())
                    start = offset + 1
                    if not raw:
                        continue
                    if "(" in raw:
                        name = raw[:raw.index("(")].split()[-1]
                        member_kind = "constructor" if name == type_name else "method"
                    else:
                        name = raw.split("{", 1)[0].rstrip(";").split()[-1]
                        member_kind = ("event" if raw.startswith("event ") else
                                       "dependency_property" if "DependencyProperty" in raw else "property")
                    if name in members:
                        raise ValueError(f"Duplicate IDL member requires overload handling: {type_name}.{name}")
                    members[name] = (member_kind, None)
        if type_name in result:
            raise ValueError(f"Duplicate IDL type: {type_name}")
        result[type_name] = members
        index = end
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--json", action="store_true", help="Print a machine-readable summary")
    parser.add_argument("--require-complete", action="store_true", help="Fail if any recorded gap remains, including optional service APIs")
    parser.add_argument("--require-control-complete", action="store_true", help="Fail if a pinned control/direct dependency gap remains")
    args = parser.parse_args()
    root = args.repository_root.resolve()
    data = root / "docs/porting/data/winui-inking"
    errors: list[str] = []

    def read(name: str) -> dict:
        value = json.loads((data / name).read_text(encoding="utf-8"))
        if value.get("schema_version") != 1:
            errors.append(f"{name}: unsupported schema version")
        if value.get("commit") != COMMIT:
            errors.append(f"{name}: upstream revision differs from the pinned baseline")
        return value

    def local_file(path: str) -> Path | None:
        file = (root / path).resolve()
        if not file.is_relative_to(root) or not file.is_file():
            errors.append(f"Missing or invalid repository file: {path}")
            return None
        return file

    def status(record: dict, identifier: str, *, code=False) -> None:
        if record.get("status") not in STATUSES:
            errors.append(f"{identifier}: invalid status {record.get('status')!r}")
        if not record.get("notes", "").strip():
            errors.append(f"{identifier}: missing explanation")
        paths = record.get("rust_files", [])
        if code and record.get("status") in CODE_STATUSES and not paths:
            errors.append(f"{identifier}: implementation status has no evidence path")
        contents = "\n".join(file.read_text(encoding="utf-8") for path in paths if (file := local_file(path)))
        for symbol in record.get("symbols", []):
            if not re.search(r"\b" + re.escape(symbol) + r"\b", contents):
                errors.append(f"{identifier}: missing mapped symbol {symbol}")

    sources = read("source-map.json")
    api = read("api-map.json")
    tests = read("tests-map.json")
    dependencies = read("dependencies.json")
    services = read("service-api-map.json")
    files = sources["files"]
    file_map = {item["path"]: item for item in files}
    if len(files) != 164 or len(file_map) != 164 or sources.get("upstream_files") != 164:
        errors.append("Source ledger must retain all 164 uniquely named files from the pinned baseline")
    for item in files:
        path = item["path"]
        if not path.startswith(("controls/dev/InkCanvas/", "controls/dev/InkToolbar/")):
            errors.append(f"Unexpected upstream source path: {path}")
        if not re.fullmatch(r"[0-9a-f]{40}", item["sha"]) or item["size"] < 0:
            errors.append(f"Invalid upstream object identity: {path}")
        status(item, path, code=True)

    contract = {}
    snapshots = api["snapshots"]
    if {item["source"] for item in snapshots} != {path for path in file_map if path.endswith(".idl")}:
        errors.append("IDL snapshots do not cover exactly the six source IDL files")
    for snapshot in snapshots:
        file = local_file(snapshot["file"])
        if file is None:
            continue
        contents = file.read_bytes()
        actual = hashlib.sha1(b"blob " + str(len(contents)).encode() + b"\0" + contents).hexdigest()
        upstream = file_map.get(snapshot["source"], {})
        if actual != snapshot["sha"] or actual != upstream.get("sha"):
            errors.append(f"Pinned IDL contents have changed: {snapshot['file']}")
        for name, members in idl_contract(contents.decode("utf-8-sig")).items():
            if name in contract:
                errors.append(f"Repeated contract type {name}")
            contract[name] = members

    type_map = {item["name"]: item for item in api["types"]}
    if len(type_map) != len(api["types"]) or set(type_map) != set(contract):
        errors.append("API type ledger differs from the pinned IDL surface")
    all_members = []
    for item in api["types"]:
        name = item["name"]
        status(item, name, code=True)
        actual_members = {}
        for member in item["members"]:
            identifier = name + "." + member["name"]
            status(member, identifier, code=True)
            if member["id"] != identifier or member["name"] in actual_members:
                errors.append(f"Invalid or duplicate API member identity: {identifier}")
            actual_members[member["name"]] = (member["kind"], member.get("value"))
            all_members.append(member)
        if actual_members != contract.get(name):
            errors.append(f"Member ledger differs from pinned IDL: {name}")

    # Keep API and interaction scenarios exhaustive against their original C#
    # suites. Tests can be consolidated in Rust, but a source method cannot
    # disappear merely by being removed from the mutable JSON ledger.
    expected_test_sources = {
        path for path, item in file_map.items()
        if item["category"] in {"api-tests", "interaction-tests"}
    }
    test_snapshots = tests["snapshots"]
    if (len(test_snapshots) != 4
            or {item["source"] for item in test_snapshots} != expected_test_sources):
        errors.append("Test snapshots must retain exactly the two API and two interaction suites")
    source_tests = {"api": set(), "interaction": set()}
    for snapshot in test_snapshots:
        file = local_file(snapshot["file"])
        if file is None:
            continue
        contents = file.read_bytes()
        actual = hashlib.sha1(b"blob " + str(len(contents)).encode() + b"\0" + contents).hexdigest()
        upstream = file_map.get(snapshot["source"], {})
        if actual != snapshot["sha"] or actual != upstream.get("sha"):
            errors.append(f"Pinned test-suite contents have changed: {snapshot['file']}")
        suite = snapshot["suite"]
        expected_category = {"api": "api-tests", "interaction": "interaction-tests"}.get(suite)
        if expected_category is None or upstream.get("category") != expected_category:
            errors.append(f"Test snapshot has an invalid suite classification: {snapshot['file']}")
            continue
        methods = re.findall(
            r"\[TestMethod\](?:\s*\[[^\]]*\])*\s*public\s+(?:async\s+)?(?:void|Task)\s+(\w+)\s*\(",
            contents.decode("utf-8-sig"),
        )
        source_tests[suite].update((snapshot["source"], method) for method in methods)
    if len(source_tests["api"]) != 65 or len(source_tests["interaction"]) != 8:
        errors.append("Pinned test declarations must contain 65 API and eight interaction methods")
    for suite, key in [("api", "tests"), ("interaction", "interaction_tests")]:
        recorded = {(item["source"], item["method"]) for item in tests[key]}
        if recorded != source_tests[suite] or len(recorded) != len(tests[key]):
            errors.append(f"{suite} test ledger differs from the pinned source methods")

    test_ids = set()
    for item in tests["tests"] + tests["interaction_tests"]:
        if item["id"] in test_ids or item["source"] not in file_map:
            errors.append(f"Invalid or repeated test identity: {item['id']}")
        test_ids.add(item["id"])
        status(item, item["id"])
        if item["status"] in CODE_STATUSES and not item["rust_tests"]:
            errors.append(f"{item['id']}: test coverage status has no mapped test")
        for test in item["rust_tests"]:
            file = local_file(test["file"])
            if file and not re.search(r"\bfn\s+" + re.escape(test["name"]) + r"\s*\(", file.read_text()):
                errors.append(f"Missing Rust test: {test['file']}::{test['name']}")

    for item in dependencies["dependencies"]:
        status(item, item["id"], code=True)
        if item.get("scope") not in {"required", "adjacent_os_service"}:
            errors.append(f"{item['id']}: dependency must declare its required or adjacent scope")

    # These are external Windows dependency declarations, not additional WinUI
    # IDL members. The immutable extract makes an omitted overload visible.
    projection = services["projection"]
    if projection.get("commit") != WINDOWS_COMMIT or projection.get("version") != "0.62.2":
        errors.append("Windows dependency projection differs from the pinned version")
    projection_file = local_file(projection["contract_snapshot"])
    service_contract = {}
    if projection_file:
        raw = projection_file.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        if digest != WINDOWS_CONTRACT_SHA256 or digest != projection.get("contract_sha256"):
            errors.append("Pinned Windows dependency declaration snapshot has changed")
        class_name = None
        for line in raw.decode("utf-8").splitlines():
            if line.startswith("[") and line.endswith("]"):
                class_name = line[1:-1]
            elif match := re.match(r"pub fn (\w+)\b", line):
                identifier = f"{class_name}.{match.group(1)}"
                if class_name is None or identifier in service_contract:
                    errors.append(f"Invalid dependency declaration in snapshot: {line}")
                service_contract[identifier] = line
    service_map = {}
    for item in services["members"]:
        identifier = item["type"] + "." + item["name"]
        if item["id"] != identifier or identifier in service_map:
            errors.append(f"Invalid or duplicate service member identity: {identifier}")
        service_map[identifier] = item["projection_signature"]
        status(item, identifier, code=True)
        if item.get("control_role") not in SERVICE_ROLES:
            errors.append(f"{identifier}: missing or invalid control dependency role")
        references = item.get("upstream_references", [])
        if item.get("control_role") == "required_by_controls" and not references:
            errors.append(f"{identifier}: direct dependency has no upstream call reference")
        if any(path not in file_map for path in references):
            errors.append(f"{identifier}: unknown upstream call reference")
    if service_map != service_contract or len(service_map) != 136:
        errors.append("External service member ledger differs from the 136 pinned declarations")
    if set(services["classes"]) != {item["type"] for item in services["members"]} or len(services["classes"]) != 12:
        errors.append("External service ledger must retain all 12 selected classes")

    sections = {"files": files, "types": api["types"], "members": all_members,
                "api_tests": tests["tests"], "interaction_tests": tests["interaction_tests"],
                "dependencies": dependencies["dependencies"],
                "service_members": services["members"]}
    summary = {key: dict(sorted(Counter(item["status"] for item in records).items()))
               for key, records in sections.items()}
    remaining = []
    for section, records in sections.items():
        for item in records:
            if item["status"] not in GAPS:
                continue
            required = (item.get("scope") != "adjacent_os_service"
                        and (section != "service_members" or item["control_role"] == "required_by_controls"))
            remaining.append({"section": section,
                              "id": item.get("id", item.get("path", item.get("name"))),
                              "status": item["status"], "required_control_scope": required,
                              "notes": item["notes"]})
    gap_count = len(remaining)
    control_gap_count = sum(item["required_control_scope"] for item in remaining)
    ledger_valid = not errors
    if args.require_complete and gap_count:
        errors.append(f"Full parity gate failed: {gap_count} records retain a partial, missing, or external status")
    if args.require_control_complete and control_gap_count:
        errors.append(f"Control parity gate failed: {control_gap_count} required-scope records retain a gap")
    report = {"commit": COMMIT, "summary": summary, "recorded_gaps": gap_count,
              "required_control_gaps": control_gap_count,
              "additional_dependency_gaps": gap_count - control_gap_count,
              "ledger_valid": ledger_valid, "completion_gate_passed": gap_count == 0,
              "control_completion_gate_passed": control_gap_count == 0,
              "remaining_records": remaining,
              "behavioral_conformance": "not assessed by this script", "errors": errors}
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print(f"WinUI inking ledger at {COMMIT}")
        for name, counts in summary.items():
            print(f"  {name}: " + ", ".join(f"{value} {key}" for key, value in counts.items()))
        print(f"  recorded gaps: {gap_count}; behavior and pixels require separate tests")
        print(f"  required control scope: {control_gap_count}; additional dependency scope: {gap_count - control_gap_count}")
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        print("Ledger valid." if ledger_valid else "Ledger invalid.")
        if ledger_valid and args.require_complete and gap_count:
            print("Full parity gate remains open.")
        if ledger_valid and args.require_control_complete and control_gap_count:
            print("Control parity gate remains open.")
    return 1 if errors else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"Inking ledger cannot be validated: {error}", file=sys.stderr)
        raise SystemExit(1)
