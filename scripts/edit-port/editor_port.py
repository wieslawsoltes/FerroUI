#!/usr/bin/env python3
"""Capture pinned editor sources and verify explicit file/type traceability offline.

This is a source inventory, not an API compatibility or behavioral parity test.
Normal generation and --check use checked-in inventory data and never open an
upstream checkout or access the network. --capture reads Git objects from local
checkouts at the exact revisions below; it never copies source bodies.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path, PurePosixPath

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE.parent / "port-status"))
from rustscan import scan_rust  # noqa: E402

INVENTORY = Path("docs/porting/data/editor-upstream.json")
MAPPINGS = Path("docs/porting/data/editor-mappings.json")
STATUS = Path("docs/porting/data/editor-status.json")
REPORT = Path("docs/porting/editor-source-map.md")
PINS = {
    "avaloniaedit": {
        "title": "AvaloniaEdit",
        "url": "https://github.com/AvaloniaUI/AvaloniaEdit",
        "commit": "7277e36366d754219d7ede16b6613786b32caf4c",
        "extensions": [".cs", ".xaml", ".axaml"],
    },
    "textmatesharp": {
        "title": "TextMateSharp",
        "url": "https://github.com/danipen/TextMateSharp",
        "commit": "e0db45be30278c0f3f80302c31f498c8f8f1c4f2",
        "extensions": [".cs"],
    },
}


class InventoryError(ValueError):
    """Invalid or stale cached inventory or mapping."""


def json_text(value: object) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def blank_csharp(text: str) -> str:
    """Blank comments and C# regular/verbatim/raw/char literals, retaining offsets.

    Interpolated strings are treated as one literal. Type declarations cannot
    occur in interpolation expressions. This lexical inventory does not resolve
    preprocessing symbols, reflection, or source-generator output.
    """
    result = list(text)
    index = 0
    size = len(text)
    while index < size:
        start = index
        if text.startswith("//", index):
            end = text.find("\n", index)
            index = size if end < 0 else end
        elif text.startswith("/*", index):
            end = text.find("*/", index + 2)
            index = size if end < 0 else end + 2
        else:
            match = re.match(r'(?:\$*@|@\$*|\$*)?"+', text[index:]) if text[index] in '$@"' else None
            if match:
                prefix = match.group()
                quotes = len(prefix) - len(prefix.rstrip('"'))
                verbatim = "@" in prefix
                index += len(prefix)
                if quotes >= 3 and not verbatim:
                    end = text.find('"' * quotes, index)
                    index = size if end < 0 else end + quotes
                else:
                    index = start + len(prefix) - quotes + 1
                    while index < size:
                        if text[index] == '"':
                            if verbatim and text.startswith('""', index):
                                index += 2
                            else:
                                index += 1
                                break
                        elif text[index] == "\\" and not verbatim:
                            index += 2
                        else:
                            index += 1
            elif text[index] == "'":
                index += 1
                while index < size:
                    if text[index] == "\\":
                        index += 2
                    elif text[index] == "'":
                        index += 1
                        break
                    else:
                        index += 1
            else:
                index += 1
                continue
        for pos in range(start, min(index, size)):
            if result[pos] not in "\r\n":
                result[pos] = " "
    blank = "".join(result)
    return re.sub(r"(?m)^[ \t]*#[^\r\n]*", lambda match: " " * len(match.group()), blank)


MODIFIERS = r"(?:(?:public|private|protected|internal|static|abstract|sealed|partial|readonly|ref|unsafe|new|file)\s+)*"
DECLARATION = re.compile(
    r"(?<![\w])(?P<mods>" + MODIFIERS + r")(?P<kind>class|struct|interface|enum|record(?:\s+(?:class|struct))?)\s+(?P<name>@?[A-Za-z_]\w*)"
)
DELEGATE = re.compile(
    r"(?P<mods>" + MODIFIERS + r")delegate\s+[\w.<>,?\[\]]+\s+(?P<name>@?[A-Za-z_]\w*)\s*(?:<[^;{}()]*>)?\s*\("
)
NAMESPACE = re.compile(r"\bnamespace\s+([A-Za-z_][\w.]*)\s*([;{])")


def csharp_types(source: str) -> list[dict]:
    text = blank_csharp(source)
    events: dict[int, tuple[str, str]] = {}
    file_namespace = ""
    for match in NAMESPACE.finditer(text):
        if match.group(2) == ";":
            file_namespace = match.group(1)
        else:
            events[match.end() - 1] = ("namespace", match.group(1))
    declarations = []
    for match in DECLARATION.finditer(text):
        # A generic constraint is not a declaration (where T : class, new()).
        before = text[: match.start()].rstrip()
        if before.endswith(":"):
            continue
        end = re.search(r"[;{}]", text[match.end() :])
        if end and end.group() == "{":
            events[match.end() + end.start()] = ("type", match.group("name").lstrip("@"))
        declarations.append((match.start(), match, match.group("kind")))
    declarations += [(m.start(), m, "delegate") for m in DELEGATE.finditer(text)]
    declarations.sort(key=lambda item: item[0])
    stack = []
    cursor = 0
    result = []
    for start, match, kind in declarations:
        for delimiter in re.finditer(r"[{}]", text[cursor:start]):
            pos = cursor + delimiter.start()
            if delimiter.group() == "{":
                stack.append(events.get(pos, ("block", "")))
            elif stack:
                stack.pop()
        cursor = start
        namespace = ".".join(filter(None, [file_namespace] + [name for scope, name in stack if scope == "namespace"]))
        parents = [name for scope, name in stack if scope == "type"]
        name = match.group("name").lstrip("@")
        modifiers = match.group("mods").split()
        visibility = " ".join(word for word in modifiers if word in {"public", "private", "internal", "protected"})
        result.append({
            "name": name,
            "qualified_name": ".".join(filter(None, [namespace, *parents, name])),
            "namespace": namespace,
            "kind": kind,
            "visibility": visibility or ("private" if parents else "internal"),
            "line": source.count("\n", 0, match.start("name")) + 1,
        })
    return result


def category(path: str) -> str:
    parts = PurePosixPath(path).parts
    project = parts[1]
    if any(part.lower() in {"resources", "testdata", "samplefiles"} for part in parts[2:]) and path.endswith(".cs"):
        if "sample" in parts[-1].lower() or any(part.lower() in {"testdata", "samplefiles"} for part in parts):
            return "sample-resource"
    if ".Tests" in project:
        return "tests"
    if ".Benchmarks" in project:
        return "benchmarks"
    if ".Demo" in project:
        return "demo"
    if project == "AvaloniaEdit.TextMate":
        return "integration"
    return "library"


def git(checkout: Path, *args: str, input_data: bytes | None = None) -> bytes:
    return subprocess.run(["git", "-C", str(checkout), *args], input=input_data, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True).stdout


def capture_repository(repo_id: str, checkout: Path) -> dict:
    pin = PINS[repo_id]
    head = git(checkout, "rev-parse", "HEAD").decode().strip()
    if head != pin["commit"]:
        raise InventoryError(f"{repo_id}: expected checkout HEAD {pin['commit']}, found {head}")
    entries = []
    tree = git(checkout, "ls-tree", "-r", "-z", pin["commit"], "--", "src").split(b"\0")
    for entry in tree:
        if not entry:
            continue
        meta, name = entry.split(b"\t", 1)
        path = name.decode()
        mode, kind, oid = meta.decode().split()
        if kind == "blob" and PurePosixPath(path).suffix in pin["extensions"]:
            entries.append((path, oid, mode))
    entries.sort()
    data = git(checkout, "cat-file", "--batch", input_data="".join(oid + "\n" for _, oid, _ in entries).encode())
    offset = 0
    files = []
    for path, oid, mode in entries:
        header_end = data.index(b"\n", offset)
        header_oid, kind, length = data[offset:header_end].decode().split()
        if header_oid != oid or kind != "blob":
            raise InventoryError(f"Unexpected Git object for {path}")
        offset = header_end + 1
        body = data[offset : offset + int(length)]
        offset += int(length) + 1
        source = body.decode("utf-8-sig")
        files.append({
            "path": path,
            "kind": "csharp" if path.endswith(".cs") else "xaml",
            "category": category(path),
            "git_blob": oid,
            "git_mode": mode,
            "sha256": sha256(body),
            "bytes": len(body),
            "lines": source.count("\n") + (bool(source) and not source.endswith("\n")),
            "types": csharp_types(source) if path.endswith(".cs") else [],
        })
    return {"id": repo_id, **pin, "root": "src", "files": files}


def safe_path(repo: Path, value: str) -> Path:
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or "\\" in value or not path.parts:
        raise InventoryError(f"Unsafe repository-relative path: {value!r}")
    resolved = (repo / value).resolve()
    if not resolved.is_relative_to(repo.resolve()):
        raise InventoryError(f"Path escapes repository through a symlink: {value}")
    return resolved


def read_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise InventoryError(f"{path}: {error}") from error


def validate_inventory(inventory: dict) -> dict[str, tuple[dict, dict]]:
    if inventory.get("schema_version") != 1:
        raise InventoryError("Unsupported editor inventory schema")
    indexed = {}
    seen_repos = set()
    for repository in inventory.get("repositories", []):
        repo_id = repository["id"]
        if repo_id not in PINS or repo_id in seen_repos:
            raise InventoryError(f"Unknown or duplicate upstream repository: {repo_id}")
        seen_repos.add(repo_id)
        for key, value in PINS[repo_id].items():
            if repository.get(key) != value:
                raise InventoryError(f"{repo_id}: cached {key} does not match the pinned source")
        for source in repository["files"]:
            path = source["path"]
            if not path.startswith("src/") or ".." in PurePosixPath(path).parts or PurePosixPath(path).suffix not in repository["extensions"]:
                raise InventoryError(f"Invalid upstream source path: {repo_id}:{path}")
            key = f"{repo_id}:{path}"
            if key in indexed:
                raise InventoryError(f"Duplicate inventory source: {key}")
            if not re.fullmatch(r"[0-9a-f]{64}", source["sha256"]):
                raise InventoryError(f"Invalid SHA-256: {key}")
            if not re.fullmatch(r"[0-9a-f]{40}", source["git_blob"]):
                raise InventoryError(f"Invalid Git object ID: {key}")
            indexed[key] = (repository, source)
    if seen_repos != set(PINS):
        raise InventoryError("Inventory must include every pinned repository")
    return indexed


def rust_symbols(path: Path) -> set[str]:
    parsed = scan_rust(str(path), path.read_text(encoding="utf-8"))
    symbols = set(parsed.free_fns) | set(parsed.free_consts)
    for name, item in parsed.types.items():
        if item.defined:
            symbols.add(name)
        symbols.update(f"{name}::{member}" for member in item.fns | item.consts | item.variants | item.properties)
        symbols.update(f"{name}::{member}" for member in item.fields)
    return symbols


def validate_mappings(repo: Path, indexed: dict, mappings: dict) -> list[dict]:
    if mappings.get("schema_version") != 1:
        raise InventoryError("Unsupported editor mappings schema")
    by_source = {}
    symbol_cache = {}
    rows = []
    for mapping in mappings.get("files", []):
        key = mapping["source"]
        if key not in indexed:
            raise InventoryError(f"Mapping refers to a source absent from inventory: {key}")
        if key in by_source:
            raise InventoryError(f"Duplicate source mapping: {key}")
        by_source[key] = mapping
        repository, source = indexed[key]
        status = mapping["status"]
        if status not in {"mapped", "adapted", "unmapped"}:
            raise InventoryError(f"Invalid mapping status: {key}")
        if not mapping.get("notes", "").strip():
            raise InventoryError(f"A mapping rationale is required: {key}")
        targets = mapping.get("targets", [])
        if (status == "unmapped") != (not targets):
            raise InventoryError(f"{key}: unmapped files must have no targets; mapped/adapted files require targets")
        declared = {item["qualified_name"] for item in source["types"]}
        covered = set()
        for target in targets:
            path = target["path"]
            destination = safe_path(repo, path)
            if not destination.is_file() or destination.suffix != ".rs":
                raise InventoryError(f"Missing Rust destination file: {path} ({key})")
            if path not in symbol_cache:
                symbol_cache[path] = rust_symbols(destination)
            symbols = target.get("symbols", [])
            if not symbols or len(symbols) != len(set(symbols)):
                raise InventoryError(f"Empty or duplicated target symbols: {path} ({key})")
            missing = set(symbols) - symbol_cache[path]
            if missing:
                raise InventoryError(f"Missing exact Rust symbol(s) in {path}: {', '.join(sorted(missing))} ({key})")
            for alias in target.get("aliases", []):
                original, rust = alias["upstream"], alias["rust"]
                if original not in declared:
                    raise InventoryError(f"Alias names an undeclared upstream type: {original} ({key})")
                if rust not in symbols:
                    raise InventoryError(f"Alias target not among verified Rust symbols: {rust} ({key})")
                if status == "mapped" and original.rsplit(".", 1)[-1] != rust:
                    raise InventoryError(f"Renamed/consolidated symbol requires adapted status: {original} -> {rust}")
                covered.add(original)
        unmapped = set(mapping.get("unmapped_types", []))
        if covered & unmapped or covered | unmapped != declared:
            missing = declared - covered - unmapped
            extra = unmapped - declared
            raise InventoryError(f"Every declared type needs an alias or explicit unmapped entry: {key}; missing={sorted(missing)}, invalid={sorted(extra)}, overlapping={sorted(covered & unmapped)}")
        if status == "mapped" and (unmapped or not covered):
            raise InventoryError(f"Mapped status requires aliases for all declared types: {key}")
        if status == "unmapped" and unmapped != declared:
            raise InventoryError(f"Unmapped source must list every declared type: {key}")
        if not isinstance(mapping.get("limitations", []), list):
            raise InventoryError(f"Limitations must be an array: {key}")
        rows.append({"source": key, "repository": repository["id"], "path": source["path"], "category": source["category"], "sha256": source["sha256"], "types": source["types"], "mapped_types": sorted(covered), **mapping})
    missing = set(indexed) - set(by_source)
    if missing:
        raise InventoryError("Sources without an explicit mapping decision:\n" + "\n".join(sorted(missing)))
    return sorted(rows, key=lambda item: item["source"])


def table_text(value: str) -> str:
    return value.replace("|", "\\|").replace("\n", " ")


def render(inventory: dict, mappings: dict, rows: list[dict]) -> tuple[dict, str]:
    totals = Counter(row["status"] for row in rows)
    by_repo = {name: Counter(row["status"] for row in rows if row["repository"] == name) for name in PINS}
    status = {
        "schema_version": 1,
        "inventory_sha256": sha256(json_text(inventory).encode()),
        "mappings_sha256": sha256(json_text(mappings).encode()),
        "counts": {key: totals[key] for key in ["mapped", "adapted", "unmapped"]},
        "file_count": len(rows),
        "declared_type_occurrences": sum(len(row["types"]) for row in rows),
        "distinct_type_declarations_per_file": sum(len({item["qualified_name"] for item in row["types"]}) for row in rows),
        "mapped_declared_types": sum(len(row["mapped_types"]) for row in rows),
        "unmapped_declared_types": sum(len(row["unmapped_types"]) for row in rows),
        "repositories": {name: {key: values[key] for key in ["mapped", "adapted", "unmapped"]} for name, values in by_repo.items()},
        "files": rows,
    }
    lines = [
        "# Editor source inventory and Rust mappings", "",
        "Generated by `python3 scripts/edit-port/editor_port.py`. Edit the cached mapping decisions in [`data/editor-mappings.json`](data/editor-mappings.json), then regenerate. `--check` verifies this report, the status JSON, every destination path, and every exact Rust symbol without an upstream checkout or network access.", "",
        "**This is source traceability, not a completeness or behavioral-parity score.** `mapped` means all declared types in that file have explicitly named Rust counterparts with the same names. `adapted` means there is a concrete Rust destination with a rename, consolidation, changed abstraction, or only partial type coverage. `unmapped` means there is no verified destination. A mapped type may still have missing methods, different contracts, or unported tests. The scanner does not compare method bodies or prove API compatibility.", "",
        "The capture includes every tracked `src/**/*.cs`, `src/**/*.xaml`, and `src/**/*.axaml` file from AvaloniaEdit, including TextMate integration, and every tracked `src/**/*.cs` file from TextMateSharp. Tests, demos, benchmarks, assembly attributes, and sample source resources remain visible in the inventory. Grammar/theme assets, project files, and sources outside `src` are outside this file inventory; asset bundling and runtime verification have separate reports.", "",
        "The cached [`editor-upstream.json`](data/editor-upstream.json) retains full SHA-256 hashes, Git blob IDs, byte/line counts, and lexically extracted C# type declarations with source lines. Extraction blanks comments and literals and records nested types, but does not expand preprocessing symbols, source generators, reflection, or external dependencies. [`editor-status.json`](data/editor-status.json) contains the complete machine-readable mapping decisions and unmapped type lists.", "",
        "## Pinned sources", "",
        "| Project | Commit | Captured files |", "|---|---|---:|",
    ]
    for repository in inventory["repositories"]:
        lines.append(f"| {repository['title']} | [`{repository['commit']}`]({repository['url']}/tree/{repository['commit']}) | {len(repository['files'])} |")
    lines += ["", "## Mapping decisions", "", "| Project | Mapped | Adapted | Unmapped | Total |", "|---|---:|---:|---:|---:|"]
    for name, values in by_repo.items():
        lines.append(f"| {PINS[name]['title']} | {values['mapped']} | {values['adapted']} | {values['unmapped']} | {sum(values.values())} |")
    lines.append(f"| **Total** | **{totals['mapped']}** | **{totals['adapted']}** | **{totals['unmapped']}** | **{len(rows)}** |")
    lines += ["", "### Category breakdown", "", "| Category | Mapped | Adapted | Unmapped | Total |", "|---|---:|---:|---:|---:|"]
    for name in sorted({row["category"] for row in rows}):
        values = Counter(row["status"] for row in rows if row["category"] == name)
        lines.append(f"| {name} | {values['mapped']} | {values['adapted']} | {values['unmapped']} | {sum(values.values())} |")
    lines += ["", "## Behavioral reports", "",
              "The [document report](editor-document.md), [editor services report](editor-services.md), [virtual-space report](editor-virtual-space.md), and [TextMate engine report](textmate-engine.md) record behavior, algorithm adaptations, verification, and remaining gaps. The crate notices retain source attribution. Per-file limitations below describe each decision; an absent limitation is not a parity certification. Repeated declarations in sample resources or conditional branches are retained in the cached occurrence list and deduplicated by qualified name within each file for mapping decisions.", ""]
    for name, pin in PINS.items():
        lines += [f"## {pin['title']} files", ""]
        projects = sorted({PurePosixPath(row["path"]).parts[1] for row in rows if row["repository"] == name})
        for project in projects:
            lines += [f"### {project}", "", "| Upstream file / SHA-256 prefix | Declared types | Status | Rust destinations and explicit aliases | Scope and remaining gaps |", "|---|---|---|---|---|"]
            for row in rows:
                if row["repository"] != name or PurePosixPath(row["path"]).parts[1] != project:
                    continue
                file_link = f"[`{row['path']}`]({pin['url']}/blob/{pin['commit']}/{row['path']}) (`{row['sha256'][:12]}`)"
                unique_types = dict.fromkeys(item["qualified_name"] for item in row["types"])
                declarations = ", ".join(f"`{name.rsplit('.', 1)[-1]}`" for name in unique_types) or "—"
                if len(unique_types) != len(row["types"]):
                    declarations += f" ({len(row['types'])} declaration occurrences)"
                destinations = []
                for target in row["targets"]:
                    aliases = ", ".join(f"`{alias['upstream'].rsplit('.', 1)[-1]} → {alias['rust']}`" for alias in target.get("aliases", []))
                    extra = sorted(set(target["symbols"]) - {alias["rust"] for alias in target.get("aliases", [])})
                    symbols = ", ".join(filter(None, [aliases, ", ".join(f"`{symbol}`" for symbol in extra)]))
                    destinations.append(f"[`{target['path']}`](../../{target['path']}): {symbols}")
                notes = row["notes"]
                if row["unmapped_types"] and row["status"] != "unmapped":
                    notes += " Unmapped types: " + ", ".join(f"`{item.rsplit('.', 1)[-1]}`" for item in row["unmapped_types"]) + "."
                if row.get("limitations"):
                    notes += " " + " ".join(row["limitations"])
                lines.append("| " + " | ".join(table_text(value) for value in [file_link, declarations, row["status"], "; ".join(destinations) or "—", notes]) + " |")
            lines.append("")
    return status, "\n".join(lines)


def generate(repo: Path) -> dict[Path, str]:
    inventory = read_json(repo / INVENTORY)
    mappings = read_json(repo / MAPPINGS)
    rows = validate_mappings(repo, validate_inventory(inventory), mappings)
    status, report = render(inventory, mappings, rows)
    return {STATUS: json_text(status), REPORT: report}


def update_outputs(repo: Path, check: bool = False) -> bool:
    stale = []
    for relative, contents in generate(repo).items():
        path = repo / relative
        existing = path.read_text(encoding="utf-8") if path.exists() else None
        if existing == contents:
            continue
        stale.append(str(relative))
        if not check:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents, encoding="utf-8")
    if check and stale:
        raise InventoryError("Stale generated editor reports; regenerate with editor_port.py:\n" + "\n".join(stale))
    return bool(stale)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--repo", type=Path, default=ROOT)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--capture", action="store_true", help="Capture inventory from both exact local Git pins; does not infer mappings")
    parser.add_argument("--avaloniaedit-root", type=Path)
    parser.add_argument("--textmatesharp-root", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.capture:
            if args.check or not args.avaloniaedit_root or not args.textmatesharp_root:
                parser.error("--capture requires both checkout paths and cannot be combined with --check")
            inventory = {"schema_version": 1, "repositories": [capture_repository(name, getattr(args, name + "_root")) for name in PINS]}
            destination = args.repo / INVENTORY
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(json_text(inventory), encoding="utf-8")
            print(f"Captured {sum(len(repo['files']) for repo in inventory['repositories'])} source files from pinned Git objects.")
        else:
            changed = update_outputs(args.repo, args.check)
            print("Editor source inventory and mappings are current." if args.check or not changed else "Regenerated editor source inventory reports.")
        return 0
    except (InventoryError, subprocess.CalledProcessError, UnicodeError) as error:
        print(f"editor-port: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
