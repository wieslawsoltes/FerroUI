#!/usr/bin/env python3
"""Reproducible source, IDL and scenario inventory for the WinUI Repeater port.

The committed extraction is immutable input; normal runs need Python 3.11 only.
Presence of a Rust identifier is a navigation aid, never a parity or test result.
Use --extract PATH only with the pinned upstream checkout/archive. A git checkout
must be at the pinned commit. An archive must originate at the URL in README.md.
"""

from __future__ import annotations

import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
DOC = ROOT / "docs/porting/winui-repeater"
DATA = DOC / "data"
PIN = "7e4b0c2096be402f51cc8f7d203ebf67cff183d0"
PREFIX = "controls/dev/Repeater/"
UPSTREAM = "https://github.com/microsoft/microsoft-ui-xaml/blob/" + PIN + "/"
REPEATER = "src/FerroUI.Controls.Repeater/"


def snake(name: str) -> str:
    name = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", name)
    name = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name)
    return name.lower().replace(".", "_")


def normalized(name: str) -> str:
    return re.sub(r"[^a-z0-9]", "", name.lower())


def strip_comments(text: str) -> str:
    # Preserve newlines and offsets so every entry keeps its original line.
    pattern = r'"(?:\\.|[^"\\])*"|//[^\n]*|/\*[\s\S]*?\*/'
    return re.sub(pattern, lambda m: m[0] if m[0].startswith('"') else
                  re.sub(r"[^\n]", " ", m[0]), text)


def strip_attributes(text: str) -> str:
    # Empty [] is an array suffix, not a MIDL attribute.
    return re.sub(r"\[[A-Za-z_][^\]]*\]", lambda m: re.sub(r"[^\n]", " ", m[0]), text)


def extract_idl(text: str, path: str) -> list[dict]:
    """Extract every defined IDL type/member, including preview/private hooks.

    This is a deliberately small scanner for the pinned MIDL3 source. Forward
    declarations and commented members are excluded; version blocks are opened
    recursively. Property get/set braces stay one declaration. Full signatures
    preserve overloads, enum values, modifiers and readable source line numbers.
    """
    clean = strip_attributes(strip_comments(text))
    result = []
    type_pattern = re.compile(r"\b(?:unsealed\s+)?(enum|runtimeclass|interface|struct)\s+(\w+)\s*([^;{]*)\{")
    for match in type_pattern.finditer(clean):
        kind, owner = match[1], match[2]
        start = match.end()
        depth = 1
        end = start
        while end < len(clean) and depth:
            depth += (clean[end] == "{") - (clean[end] == "}")
            end += 1
        if depth:
            raise ValueError(f"Unterminated IDL type {path}:{owner}")
        result.append({"path": path, "line": text.count("\n", 0, match.start()) + 1,
                       "owner": owner, "kind": "type", "name": owner,
                       "signature": " ".join(match[0][:-1].split())})
        body = clean[start:end - 1]
        if kind == "enum":
            for field in re.finditer(r"\b(\w+)\s*=\s*([^,}]+)", body):
                result.append({"path": path, "line": text.count("\n", 0, start + field.start()) + 1,
                               "owner": owner, "kind": "enum-value", "name": field[1],
                               "signature": " ".join(field[0].split())})
            continue
        # Remove version-group braces, retaining property braces. Every
        # declaration ends in a top-level semicolon (not a property's get/set).
        statement_start = 0
        property_depth = 0
        for index, char in enumerate(body):
            if char == "{":
                prefix = body[statement_start:index].strip()
                if not prefix:
                    statement_start = index + 1
                else:
                    property_depth += 1
            elif char == "}":
                if property_depth:
                    property_depth -= 1
                else:
                    statement_start = index + 1
            elif char == ";" and property_depth == 0:
                statement = body[statement_start:index].strip()
                offset = statement_start + len(body[statement_start:index]) - len(body[statement_start:index].lstrip())
                statement_start = index + 1
                if not statement:
                    continue
                signature = " ".join(statement.split())
                before = statement.split("(", 1)[0].split("{", 1)[0]
                names = re.findall(r"\b[A-Za-z_]\w*\b", before)
                if not names:
                    raise ValueError(f"Unrecognized IDL declaration {path}:{owner}: {signature}")
                name = names[-1]
                member_kind = ("constructor" if name == owner else "event" if "event" in names else
                               "property" if "{" in statement else "method" if "(" in statement else "field")
                result.append({"path": path, "line": text.count("\n", 0, start + offset) + 1,
                               "owner": owner, "kind": member_kind, "name": name, "signature": signature})
    # Delegates are top-level declarations and do not have a body.
    for match in re.finditer(r"\bdelegate\s+\w+\s+(\w+)\s*\([^;]*\)\s*;", clean):
        result.append({"path": path, "line": text.count("\n", 0, match.start()) + 1,
                       "owner": match[1], "kind": "delegate", "name": match[1],
                       "signature": " ".join(match[0].rstrip(";").split())})
    return sorted(result, key=lambda item: (item["path"], item["line"], item["signature"]))


def extract(checkout: Path) -> dict:
    if (checkout / ".git").exists():
        actual = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
        if actual != PIN:
            raise ValueError(f"Expected pinned checkout {PIN}, found {actual}")
    source = checkout / PREFIX
    if not source.is_dir():
        raise ValueError(f"Missing {source}")
    files, apis, tests, includes = [], [], [], []
    for file in sorted(path for path in source.rglob("*") if path.is_file()):
        relative = file.relative_to(checkout).as_posix()
        raw = file.read_bytes()
        files.append({"path": relative, "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()})
        if file.suffix not in (".idl", ".cpp", ".h", ".cs"):
            continue
        text = raw.decode("utf-8-sig")
        if file.suffix == ".idl":
            apis.extend(extract_idl(text, relative))
        clean = strip_comments(text)
        if file.suffix in (".cpp", ".h"):
            for include in re.finditer(r'^\s*#\s*include\s*["<]([^">]+)[">]', clean, re.M):
                includes.append({"source": relative, "include": include[1],
                                 "line": text.count("\n", 0, include.start()) + 1})
        if file.suffix == ".cs":
            pattern = r"\[TestMethod\](?:(?!\[TestMethod\])[\s\S])*?public\s+(?:(?:async|override|virtual|static)\s+)*(?:void|Task)\s+(\w+)\s*\("
            for test in re.finditer(pattern, text):
                state = "declared"
                if clean[test.start():test.start() + 12] != "[TestMethod]":
                    state = "commented-out"
                elif re.search(r'TestProperty\s*\(\s*"Ignore"\s*,\s*"True"\s*\)', clean[test.start():test.end()]):
                    state = "ignored"
                tests.append({"path": relative, "name": test[1],
                              "line": text.count("\n", 0, test.start()) + 1,
                              "upstream_state": state,
                              "suite": "interaction" if "/InteractionTests/" in relative else "api"})
    # Catch extraction drift, omission, or unexpectedly different upstream input.
    if len(files) != 302 or len(tests) != 265:
        raise ValueError(f"Pinned extraction changed: {len(files)} sources, {len(tests)} scenarios; expected 302/265")
    return {"schema": 1, "repository": "microsoft/microsoft-ui-xaml", "commit": PIN,
            "scope": PREFIX.rstrip("/"), "files": files, "apis": apis,
            "tests": tests, "include_edges": includes}


def load_json(path: Path) -> dict:
    return json.loads(path.read_text())


def source_targets(path: str, mappings: dict) -> tuple[list[str], str, str]:
    relative = path.removeprefix(PREFIX)
    if relative in mappings["source_overrides"]:
        row = mappings["source_overrides"][relative]
        return row["targets"], row["status"], row["note"]
    name = Path(relative).name
    stem = name.split(".", 1)[0]
    if relative.startswith("TestUI/"):
        return [], "sample-reference", "Reference/sample application; exact WinUI sample page has not been ported. See the FerroUI catalog and validation plan."
    if Path(relative).suffix in (".projitems", ".shproj", ".vcxitems", ".filters"):
        return ["src/FerroUI.Controls/Cargo.toml"], "build-replaced", "MSBuild/C++ project wiring is replaced by Cargo and FerroUI registration."
    if relative.startswith(("APITests/", "InteractionTests/")):
        if name in mappings["test_files"]:
            return mappings["test_files"][name], "test-mapped", "Scenario mappings are enumerated independently in TESTS.md."
        return [], "test-support-reference", "WinUI test fixture/helper; equivalent Rust fixtures live with the ported scenario tests."
    row = mappings["families"].get(stem)
    if row:
        return row["targets"], row.get("status", "implementation-mapped"), row["note"]
    candidates = [REPEATER + snake(stem) + ".rs", REPEATER + "layouts/" + snake(stem) + ".rs",
                  REPEATER + "selection/" + snake(stem) + ".rs"]
    targets = [candidate for candidate in candidates if (ROOT / candidate).is_file()]
    if targets:
        return targets, "implementation-mapped", "Direct Rust implementation; behavioral acceptance remains scenario-based."
    return [], "unmapped", "Needs an explicit implementation or documented framework replacement."


def rust_tests() -> list[dict]:
    paths = set((ROOT / REPEATER).rglob("*.rs"))
    paths.update((ROOT / "tests/FerroUI.Markup.Xaml.UnitTests").rglob("*.rs"))
    paths.add(ROOT / "src/FerroUI.Controls/items_source_view.rs")
    result = []
    for path in sorted(paths):
        if not path.is_file():
            continue
        text = strip_comments(path.read_text())
        # Additional cfg/ignore/serial attributes between #[test] and fn are allowed.
        for match in re.finditer(r"#\[test\]\s*(?:#\[[^\]]*\]\s*)*(?:pub\s+)?fn\s+(\w+)\s*\(", text):
            result.append({"path": path.relative_to(ROOT).as_posix(), "name": match[1],
                           "line": text.count("\n", 0, match.start()) + 1})
    return result


def render_link(path: str, label: str | None = None) -> str:
    return f"[{label or path}](../../../{path})"


def upstream_link(item: dict, label: str) -> str:
    return f"[{label}]({UPSTREAM}{item['path']}#L{item['line']})"


def format_cell(value: str) -> str:
    return value.replace("|", "\\|").replace("\n", " ")


def generate(inventory: dict, mappings: dict, aliases: dict) -> dict[str, str]:
    outputs = {}
    signature_keys = {api["owner"] + "::" + api["signature"] for api in inventory["apis"]}
    stale_signatures = set(mappings.get("api_signature_overrides", {})) - signature_keys
    if stale_signatures:
        raise ValueError(f"Stale API signature mappings: {sorted(stale_signatures)}")
    file_rows = []
    for file in inventory["files"]:
        targets, status, note = source_targets(file["path"], mappings)
        for target in targets:
            if not (ROOT / target).is_file():
                raise ValueError(f"Source map target does not exist: {file['path']} -> {target}")
        file_rows.append({**file, "targets": targets, "status": status, "note": note})
    tests = rust_tests()
    by_name = defaultdict(list)
    for test in tests:
        by_name[normalized(test["name"])].append(test)
    test_rows = []
    for test in inventory["tests"]:
        key = test["path"].removeprefix(PREFIX) + "::" + test["name"]
        alias = aliases.get(key)
        if alias:
            targets = [row for row in tests if row["name"] in alias["rust_tests"]]
            missing = set(alias["rust_tests"]) - {row["name"] for row in targets}
            if missing:
                raise ValueError(f"Stale scenario alias {key}: {sorted(missing)}")
            note = alias["reason"]
            status = alias.get("status", "explicit-alias")
        else:
            targets = by_name[normalized(test["name"])]
            # Avoid associating a common short test name across unrelated suites.
            allowed, _, _ = source_targets(test["path"], mappings)
            targets = [target for target in targets if target["path"] in allowed]
            note = "Matching test name in the mapped suite; inspect assertions for behavioral acceptance." if targets else "No exact-name scenario mapping recorded."
            status = "name-matched" if targets else "unmapped"
        test_rows.append({**test, "status": status, "targets": targets, "note": note})
    file_targets = {Path(row["path"]).stem: row["targets"] for row in file_rows if "/" not in row["path"].removeprefix(PREFIX)}
    api_rows = []
    for api in inventory["apis"]:
        owner = api["owner"]
        targets = mappings["api_types"].get(owner, file_targets.get(owner, []))
        signature_key = owner + "::" + api["signature"]
        override = mappings.get("api_signature_overrides", {}).get(signature_key)
        if override is None:
            override = mappings.get("api_overrides", {}).get(owner + "." + api["name"])
        if override:
            targets = override["targets"]
        contents = [(path, (ROOT / path).read_text()) for path in targets]
        search_names = [api["name"]]
        if api["kind"] == "constructor":
            search_names = ["new", "construct", "default", "with_source"]
        if override and "symbols" in override:
            search_names = override["symbols"]
        found = []
        found_symbols = set()
        for path, text in contents:
            stripped = strip_comments(text)
            for candidate in search_names:
                alternatives = {candidate, snake(candidate), snake(candidate).upper()}
                present = {name for name in alternatives if re.search(r"\b" + re.escape(name) + r"\b", stripped)}
                if present:
                    found.append(path)
                    found_symbols.update(present)
                    break
        status = "symbol-found" if found else "needs-review"
        note = (override or {}).get("note", "Identifier is present in the mapped implementation; signature, overload and behavior require the scenario evidence.")
        if override and override.get("replacement"):
            status = "framework-replacement"
        api_rows.append({**api, "targets": targets, "symbol_paths": found,
                         "rust_symbols": sorted(found_symbols), "status": status, "note": note})
    by_include = defaultdict(list)
    local = {Path(row["path"]).name for row in file_rows}
    for edge in inventory["include_edges"]:
        if Path(edge["include"]).name not in local:
            by_include[edge["include"]].append(edge)
    dependencies = []
    for name, edges in sorted(by_include.items()):
        row = mappings["dependencies"].get(name)
        if row is None and name.endswith((".g.h", ".g.cpp")):
            row = {"targets": ["src/FerroUI.Controls/register_types.rs", REPEATER + "markup.rs"],
                   "status": "framework-replacement", "note": "Generated WinRT activation/metadata is replaced by FerroUI class and markup metadata."}
        if row is None:
            row = {"targets": [], "status": "needs-review", "note": "Direct include boundary requires source review; no silent dependency waiver."}
        for target in row["targets"]:
            if not (ROOT / target).exists():
                raise ValueError(f"Dependency target does not exist: {name} -> {target}")
        dependencies.append({"include": name, **row, "edges": edges})
    summary = {"source_files": len(file_rows), "source_status": dict(sorted(Counter(row["status"] for row in file_rows).items())),
               "idl_declarations": len(api_rows), "api_status": dict(sorted(Counter(row["status"] for row in api_rows).items())),
               "upstream_scenarios": len(test_rows), "test_status": dict(sorted(Counter(row["status"] for row in test_rows).items())),
               "upstream_test_states": dict(sorted(Counter(row["upstream_state"] for row in test_rows).items())),
               "rust_test_functions": len(tests), "dependency_boundaries": len(dependencies),
               "dependency_status": dict(sorted(Counter(row["status"] for row in dependencies).items()))}
    native_extensions = mappings.get("native_extensions", [])
    for extension in native_extensions:
        for target in extension["targets"]:
            if not (ROOT / target).is_file():
                raise ValueError(f"Native extension target does not exist: {target}")
    report = {"schema": 1, "upstream_commit": PIN, "summary": summary,
              "sources": file_rows, "apis": api_rows, "tests": test_rows,
              "rust_tests": tests, "dependencies": dependencies,
              "native_extensions": native_extensions}
    outputs["data/status.json"] = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    header = f"Generated by `scripts/winui-repeater/inventory.py`. Upstream commit [`{PIN}`](https://github.com/microsoft/microsoft-ui-xaml/tree/{PIN}/controls/dev/Repeater).\n\n"
    source_lines = ["# Repeater source map\n", header,
                    "Every upstream file is listed, including private implementations, IDL, tests, build inputs and sample assets. **Mapped** records an implementation location, not completed semantic parity. Sample pages are separately recorded so they cannot disappear from the inventory.\n",
                    "| State | Files |", "|---|---:|"]
    source_lines.extend(f"| {state} | {count} |" for state, count in summary["source_status"].items())
    source_lines += ["", "| Upstream file | State | FerroUI target | Decision |", "|---|---|---|---|"]
    for row in file_rows:
        source_lines.append(f"| [{row['path'].removeprefix(PREFIX)}]({UPSTREAM}{row['path']}) | {row['status']} | " +
                            "; ".join(render_link(path, Path(path).name) for path in row["targets"]) + " | " + format_cell(row["note"]) + " |")
    outputs["SOURCES.md"] = "\n".join(source_lines) + "\n"
    api_lines = ["# Repeater IDL contract map\n", header,
                 "This inventory includes every defined type and member in all five Repeater IDL files, including preview APIs, private scrolling contracts and test hooks. Forward declarations and commented-out APIs are excluded. Property accessors form one declaration; overloads remain separate rows. **Symbol-found is a source navigation check, not an ABI, overload, runtime-markup or behavior guarantee.** Types and inherited members are mapped to FerroUI rather than WinRT.\n",
                 "| State | Declarations |", "|---|---:|"]
    api_lines.extend(f"| {state} | {count} |" for state, count in summary["api_status"].items())
    for owner in sorted({row["owner"] for row in api_rows}):
        api_lines += ["", f"## {owner}", "", "| Upstream declaration | Kind | State | FerroUI target | Native identifier |", "|---|---|---|---|---|"]
        for row in (row for row in api_rows if row["owner"] == owner):
            api_lines.append(f"| {upstream_link(row, '`' + format_cell(row['signature']) + '`')} | {row['kind']} | {row['status']} | " +
                             "; ".join(render_link(path, Path(path).name) for path in row["targets"]) + " | " +
                             ", ".join("`" + symbol + "`" for symbol in row["rust_symbols"]) + " |")
    outputs["API.md"] = "\n".join(api_lines) + "\n"
    test_lines = ["# Repeater upstream scenario map\n", header,
                  "The extraction contains **265 `[TestMethod]` declarations** (262 API scenarios and 3 interaction scenarios): 235 declared, 27 ignored and 3 commented out. All remain visible below. Parameter rows are not separate entry points. Matching a name does not assert equivalent coverage, a successful run or native Windows interaction. Run evidence belongs in [VALIDATION.md](VALIDATION.md). An explicit alias must name existing `#[test]` functions and state the behavioral relation. `related-regressions` identifies useful partial evidence rather than equivalent scenario coverage; `known-upstream-difference` identifies a deliberately different assertion.\n",
                  "| State | Scenarios |", "|---|---:|"]
    test_lines.extend(f"| {state} | {count} |" for state, count in summary["test_status"].items())
    test_lines += ["", f"The scanned FerroUI areas contain {len(tests)} Rust `#[test]` functions, including supplementary regressions and markup tests. This count is not a pass count."]
    for path in sorted({row["path"] for row in test_rows}):
        test_lines += ["", "## " + path.removeprefix(PREFIX), "", "| Upstream scenario | Upstream state | Mapping state | Rust test | Mapping note |", "|---|---|---|---|---|"]
        for row in (row for row in test_rows if row["path"] == path):
            test_lines.append(f"| {upstream_link(row, row['name'])} | {row['upstream_state']} | {row['status']} | " +
                              "; ".join(render_link(target["path"], "`" + target["name"] + "`") for target in row["targets"]) +
                              " | " + format_cell(row["note"]) + " |")
    outputs["TESTS.md"] = "\n".join(test_lines) + "\n"
    dep_lines = ["# Repeater direct include boundaries\n", header,
                 "This table records every unique direct C++ include whose basename is outside the pinned Repeater subtree. Internal Repeater include edges remain in `data/upstream.json`. It is a first-order dependency inventory; it does not claim the transitive WinUI framework has been ported. FerroUI service replacements are explained in [DESIGN.md](DESIGN.md). Generated C++/WinRT headers do not require a second object system.\n",
                 "| Include | State | FerroUI service | Decision |", "|---|---|---|---|"]
    for row in dependencies:
        dep_lines.append(f"| `{row['include']}` | {row['status']} | " +
                         "; ".join(render_link(path, Path(path).name) for path in row["targets"]) + " | " + format_cell(row["note"]) + " |")
    if native_extensions:
        dep_lines += ["", "## Native framework extensions", "",
                      "These are explicit changes to existing FerroUI services needed by a Repeater source contract. They are separate from the direct C++ include-boundary count.", "",
                      "| Source contract | Native implementation | Rationale |", "|---|---|---|"]
        for row in native_extensions:
            dep_lines.append("| " + format_cell(row["source"]) + " | " +
                             "; ".join(render_link(path, Path(path).name) for path in row["targets"]) +
                             " | " + format_cell(row["note"]) + " |")
    outputs["DEPENDENCIES.md"] = "\n".join(dep_lines) + "\n"
    return outputs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify generated output without writing")
    parser.add_argument("--extract", type=Path, metavar="UPSTREAM", help="extract the pinned upstream source checkout/archive")
    args = parser.parse_args()
    if args.extract and args.check:
        parser.error("--extract and --check are separate operations")
    DATA.mkdir(parents=True, exist_ok=True)
    if args.extract:
        inventory = extract(args.extract)
        (DATA / "upstream.json").write_text(json.dumps(inventory, indent=2, ensure_ascii=False) + "\n")
    else:
        inventory = load_json(DATA / "upstream.json")
    if inventory["commit"] != PIN:
        raise ValueError("Committed upstream inventory does not match the source notice pin")
    outputs = generate(inventory, load_json(DATA / "mapping.json"), load_json(DATA / "test-aliases.json"))
    changed = []
    for relative, text in outputs.items():
        destination = DOC / relative
        if not destination.exists() or destination.read_text() != text:
            changed.append(relative)
            if not args.check:
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_text(text)
    summary = json.loads(outputs["data/status.json"])["summary"]
    print(json.dumps(summary, indent=2))
    if args.check and changed:
        print("Stale Repeater inventory outputs: " + ", ".join(changed), file=sys.stderr)
        return 1
    print("Repeater inventories are current." if args.check else "Generated Repeater inventories.")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, json.JSONDecodeError) as error:
        print(f"Repeater inventory: {error}", file=sys.stderr)
        raise SystemExit(1)
