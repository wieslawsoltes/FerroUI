#!/usr/bin/env python3
"""Check all WinUI Symbol numeric values and modern font glyphs, offline by default."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


REPO = Path(__file__).resolve().parents[2]
DATA = REPO / "docs/porting/winui-gallery/data"
MODEL = "dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Controls.cs"
RENDERER = "dxaml/xcp/core/core/elements/icon.cpp"


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def extract(model: str, renderer: str) -> list[tuple[str, int, int]]:
    match = re.search(r"public enum Symbol\s*\{(.*?)\n\s*\}", model, re.S)
    if not match:
        raise ValueError("Symbol enum missing from pinned model")
    values = [(name, int(value, 16)) for name, value in re.findall(r"\b(\w+)\s*=\s*(0x[0-9A-Fa-f]+)\s*,", match[1])]
    if not values or len({name for name, _ in values}) != len(values) or len({v for _, v in values}) != len(values):
        raise ValueError("Symbol model is empty or contains duplicate names/numeric values")
    match = re.search(r"CSymbolIcon::ConvertSymbolValueToGlyph\([^)]*\)\s*\{(.*?)\n\}", renderer, re.S)
    if not match:
        raise ValueError("Symbol glyph conversion missing from pinned renderer")
    pairs = re.findall(r"case\s+(0x[0-9A-Fa-f]+):\s*return static_cast<WCHAR>\((0x[0-9A-Fa-f]+)\);", match[1])
    mapping = {int(value, 16): int(glyph, 16) for value, glyph in pairs}
    if len(mapping) != len(pairs):
        raise ValueError("Duplicate Symbol rendering cases")
    if not re.search(r"(?:default:|\};)\s*return static_cast<WCHAR>\(symbolValue\);", match[1]):
        raise ValueError("Unrecognized Symbol rendering default; manual review required")
    return [(name, value, mapping.get(value, value)) for name, value in values]


def table(entries: list[tuple[str, int, int]], commit: str) -> str:
    return (f"# Symbol enum values and rendered glyphs from microsoft-ui-xaml {commit}\n"
            "# name\tapi-value\tglyph-codepoint\n"
            + "".join(f"{name}\t0x{value:04X}\t0x{glyph:04X}\n" for name, value, glyph in entries))


def verify_rust(source: str, entries: list[tuple[str, int, int]]) -> None:
    expected_values = {name: value for name, value, _ in entries}
    expected_glyphs = {name: glyph for name, _, glyph in entries}
    match = re.search(r"pub enum Symbol\s*\{(.*?)\n\}", source, re.S)
    actual_values = {name: int(value, 16) for name, value in re.findall(r"\b(\w+)\s*=\s*(0x[0-9A-Fa-f]+)\s*,", match[1] if match else "")}
    actual_glyphs = {name: int(glyph, 16) for name, glyph in re.findall(r"Self::(\w+)\s*=>\s*'\\u\{([0-9A-Fa-f]+)\}'", source)}
    if actual_values != expected_values:
        raise ValueError("Rust Symbol enum differs from pinned public values")
    if actual_glyphs != expected_glyphs:
        raise ValueError("Rust Symbol::glyph differs from pinned rendering map")
    markup = re.search(r"ferro_markup_enum!\(Symbol\s*\{(.*?)\}", source, re.S)
    if not markup or set(re.findall(r"\b\w+\b", markup[1])) != set(expected_values):
        raise ValueError("Symbol markup metadata omits or adds enum members")
    all_values = re.search(r"pub const ALL:.*?=\s*&\[(.*?)\];", source, re.S)
    if not all_values or re.findall(r"Self::(\w+)", all_values[1]) != list(expected_values):
        raise ValueError("Symbol::ALL differs from pinned declaration order")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Offline verification (the default)")
    parser.add_argument("--refresh-source", action="store_true", help="Refresh glyph reference from a clean pinned checkout")
    parser.add_argument("--winui-root", type=Path)
    args = parser.parse_args()
    commit = json.loads((DATA / "sources.lock.json").read_text())["repositories"]["winui"]["commit"]
    reference = DATA / "symbol-glyphs.tsv"
    lock_path = DATA / "symbol-sources.lock.json"
    if args.refresh_source:
        if not args.winui_root:
            parser.error("--refresh-source requires --winui-root")
        root = args.winui_root
        actual = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        if actual != commit:
            raise ValueError("WinUI checkout does not match sources.lock.json")
        subprocess.run(["git", "-C", str(root), "diff", "--exit-code", "HEAD", "--", MODEL, RENDERER], check=True, stdout=subprocess.DEVNULL)
        inputs = {path: (root / path).read_bytes() for path in [MODEL, RENDERER]}
        entries = extract(inputs[MODEL].decode("utf-8-sig"), inputs[RENDERER].decode("utf-8-sig"))
        reference.write_text(table(entries, commit))
        lock = {"schema_version": 1, "commit": commit, "source_sha256": {path: sha(data) for path, data in inputs.items()}, "snapshot_sha256": sha(reference.read_bytes())}
        lock_path.write_text(json.dumps(lock, indent=2, sort_keys=True) + "\n")
    lock = json.loads(lock_path.read_text())
    if lock["commit"] != commit or lock["snapshot_sha256"] != sha(reference.read_bytes()):
        raise ValueError("Symbol reference pin/hash mismatch")
    entries = [(name, int(value, 16), int(glyph, 16)) for line in reference.read_text().splitlines() if not line.startswith("#") for name, value, glyph in [line.split("\t")]]
    verify_rust((REPO / "src/FerroUI.Controls/symbol_icon.rs").read_text(), entries)
    print(f"Symbol map verified: {len(entries)} public values, rendered glyphs, markup names and iteration entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
