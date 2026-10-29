#!/usr/bin/env python3
"""Check the frozen source inventory and explicit port records.

This is a structural audit, not an API parser or a proof of behavioral parity.
Only enum declarations have mechanically checked member/value correspondence.
"""

from __future__ import annotations

import argparse
import ast
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


PORT_ROOT = Path(__file__).resolve().parents[2]
SOURCE_ROOT = PORT_ROOT.parent / 'Dock'


def source_digest(data: bytes) -> str:
    """Hash text independent of Git's platform checkout line-ending conversion."""
    return hashlib.sha256(data.replace(b"\r\n", b"\n")).hexdigest()


def source_paths(root: Path) -> set[str]:
    """Tracked source/project inputs, excluding artifacts and documentation."""
    result = subprocess.run(
        ["git", "ls-files", "-z", "--", "src", "tests", "samples"],
        cwd=root, check=True, capture_output=True,
    )
    return {
        name for name in result.stdout.decode().split("\0")
        if Path(name).suffix in {".cs", ".axaml", ".csproj"}
    }


def integer_expression(expression: str, names: dict[str, int]) -> int:
    """Evaluate only the integer grammar used in the source enums."""
    def evaluate(node: ast.AST) -> int:
        if isinstance(node, ast.Constant) and type(node.value) is int:
            return node.value
        if isinstance(node, ast.Name):
            return names[node.id]
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
            return -evaluate(node.operand)
        if isinstance(node, ast.BinOp):
            if isinstance(node.op, ast.LShift):
                return evaluate(node.left) << evaluate(node.right)
            if isinstance(node.op, ast.BitOr):
                return evaluate(node.left) | evaluate(node.right)
        raise ValueError(f"unsupported enum expression: {expression}")
    return evaluate(ast.parse(expression, mode="eval").body)


def enum_members(source: str) -> tuple[str, dict[str, int], bool] | None:
    """Read simple enum declarations; comments are removed before member parsing."""
    source = re.sub(r"/\*.*?\*/|//[^\n]*", "", source, flags=re.S)
    declaration = re.search(r"public enum (\w+)\s*\{([^}]*)\}", source, re.S)
    if declaration is None:
        return None
    name, body = declaration.groups()
    members: dict[str, int] = {}
    value = -1
    for entry in body.split(","):
        entry = entry.strip()
        if not entry:
            continue
        member, separator, expression = entry.partition("=")
        value = integer_expression(expression.strip(), members) if separator else value + 1
        members[member.strip()] = value
    flags = re.search(r"\[(?:System\.)?Flags\]", source) is not None
    return name, members, flags


def verify_enum(source: str, target: str, listed_members: list[str]) -> list[str]:
    """Compare explicit Rust enum/bitflag constants with all source members."""
    parsed = enum_members(source)
    if parsed is None:
        return []
    name, members, flags = parsed
    errors = []
    if listed_members != list(members):
        errors.append(f"{name}: inventory members do not match the source enum")
    expected = {}
    for member, value in members.items():
        label = re.sub(r"(?<!^)(?=[A-Z])", "_", member).upper() if flags else member
        expected[label] = value
    # These ports use explicit integer values. Restrict parsing to the declaration
    # so test assertions and TryFrom arms cannot masquerade as definitions.
    pattern = rf"pub struct {name}\s*:\s*i32\s*\{{([^}}]*)\}}" if flags else rf"pub enum {name}\s*\{{([^}}]*)\}}"
    declaration = re.search(pattern, target, re.S)
    if declaration is None:
        # Open Int32 enums retain unknown values without unsafe Rust discriminants.
        # Inspect only actual public Self constants on the transparent value type.
        if flags or not re.search(rf"#\[repr\(transparent\)\]\s*pub struct {name}\(i32\);", target):
            return errors + [f"{name}: missing Rust enum/flags declaration"]
        constants = []
        for implementation in re.finditer(rf"\bimpl {name}\s*\{{", target):
            start = implementation.end()
            depth = 1
            end = start
            while end < len(target) and depth:
                depth += (target[end] == "{") - (target[end] == "}")
                end += 1
            if depth:
                return errors + [f"{name}: incomplete open enum implementation"]
            body = re.sub(r"//[^\n]*", "", target[start:end - 1])
            constants.extend(re.findall(r"pub const (\w+): Self\s*=\s*Self\((-?\d+)\);", body))
        actual = {label: int(value) for label, value in constants}
        if actual != expected or len(constants) != len(actual):
            errors.append(f"{name}: Rust numeric members differ: expected {expected}, got {actual}")
        return errors
    body = re.sub(r"//[^\n]*", "", declaration.group(1))
    constant = r"const (\w+)\s*=\s*(-?\d+)" if flags else r"(\w+)\s*=\s*(-?\d+)"
    actual = {label: int(value) for label, value in re.findall(constant, body)}
    if actual != expected:
        errors.append(f"{name}: Rust numeric members differ: expected {expected}, got {actual}")
    return errors


def audit(port_root: Path, source_root: Path, require_complete: bool = False) -> tuple[list[str], Counter]:
    """Check frozen inputs, target paths and the declared structural conventions."""
    baseline = json.loads((port_root / "docs/porting/dock/upstream-files.json").read_text())
    records = json.loads((port_root / "docs/porting/dock/ports.json").read_text())
    errors = []
    if baseline["source_revision"] != records["source_revision"]:
        errors.append("source revisions in the inventory and port records differ")
    expected = {entry["path"] for entry in baseline["files"]}
    actual = source_paths(source_root)
    for path in sorted(actual - expected):
        errors.append(f"new source input needs inventory reconciliation: {path}")
    for path in sorted(expected - actual):
        errors.append(f"removed source input needs inventory reconciliation: {path}")
    for entry in baseline["files"]:
        path = source_root / entry["path"]
        if not path.is_file() or source_digest(path.read_bytes()) != entry["sha256"]:
            errors.append(f"source changed from frozen baseline: {entry['path']}")

    seen = set()
    target_owners: dict[str, list[str]] = {}
    counts = Counter()
    for record in records["files"]:
        source = record["source"]
        status = record["status"]
        counts[status] += 1
        if source in seen or source not in expected:
            errors.append(f"duplicate or unknown source in port records: {source}")
        seen.add(source)
        if status not in {"ported", "partial"}:
            errors.append(f"unsupported status for {source}: {status}")
        if status == "partial" and not record.get("deferred_members"):
            errors.append(f"partial source needs explicit deferred members: {source}")
        targets = record.get("targets", [])
        if not targets:
            errors.append(f"port has no targets: {source}")
        for target in targets:
            path = port_root / target
            if not path.resolve().is_relative_to(port_root.resolve()) or not path.is_file():
                errors.append(f"invalid or missing target for {source}: {target}")
                continue
            target_owners.setdefault(target, []).append(source)
        if (source_root / source).is_file():
            source_text = (source_root / source).read_text(encoding="utf-8-sig")
            parsed = enum_members(source_text)
            if parsed is not None and Path(source).stem == parsed[0]:
                # Adding a descriptor/test target must not bypass the declaration audit.
                name = parsed[0]
                candidates = [target for target in targets if target.endswith(".rs")
                              and (port_root / target).is_file()
                              and re.search(rf"pub enum {name}\b|pub struct {name}\s*[:(]", (port_root / target).read_text())]
                if len(candidates) != 1:
                    errors.append(f"{name}: expected one actual Rust enum declaration target")
                else:
                    errors.extend(verify_enum(source_text, (port_root / candidates[0]).read_text(), record.get("members", [])))
            elif len(targets) == 1 and (port_root / targets[0]).is_file():
                errors.extend(verify_enum(source_text, (port_root / targets[0]).read_text(), record.get("members", [])))

    counts["pending"] = len(expected - seen)
    rust_sources = sorted(path for folder in ("src", "samples") for path in (port_root / folder).rglob("*.rs"))
    for path in rust_sources:
        content = path.read_text()
        if re.search(r"avalonia|avn", str(path.relative_to(port_root)) + content, re.I):
            errors.append(f"upstream UI identifier in Rust source: {path.relative_to(port_root)}")
        if re.search(r"\b(?:todo|unimplemented)!\s*\(", content):
            errors.append(f"placeholder implementation in {path.relative_to(port_root)}")
        # Module declarations and crate roots are port infrastructure, not source ports.
        if path.name not in {"lib.rs", "mod.rs"} and str(path.relative_to(port_root)) not in target_owners:
            errors.append(f"Rust file needs a source mapping or explicit infrastructure record: {path.relative_to(port_root)}")
    if require_complete and (counts["pending"] or counts["partial"]):
        errors.append("full port is incomplete: pending or partial source files remain")
    return errors, counts


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--require-complete", action="store_true", help="also reject pending and partial sources")
    args = parser.parse_args()
    errors, counts = audit(PORT_ROOT, SOURCE_ROOT, args.require_complete)
    print(f"Source files: {sum(counts.values())}; ported: {counts['ported']}; partial: {counts['partial']}; pending: {counts['pending']}.")
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    if not errors:
        print("Structural inventory checks passed. Behavioral parity requires the ported tests and integration evidence.")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
