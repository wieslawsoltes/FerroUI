#!/usr/bin/env python3
"""Audit pinned WinUI predefined weights against existing FerroUI values and metadata.

This static source check executes no Rust or native runtime tests.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET

WINUI_COMMIT = "fe244ca6ac4d482712b234a9de3cf771cc6481de"
WINUI_BLOB = "4a6eecae47376db0199afaaf0c9d8c7a5a503df8"
WINUI_PATH = "packaging/Intellisense/generated/Microsoft.UI.Text.xml"
NAMES = {
    "Black", "Bold", "ExtraBlack", "ExtraBold", "ExtraLight", "Light", "Medium",
    "Normal", "SemiBold", "SemiLight", "Thin",
}


def blob(data: bytes) -> str:
    return hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()


def line(source: str, anchor: str) -> int:
    position = source.index(anchor)
    return source.count("\n", 0, position) + 1


def main() -> None:
    if not __debug__:
        raise SystemExit("Run this source audit without Python optimization (-O).")
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--reference", type=Path, required=True, help="checkout of pinned microsoft-ui-xaml")
    parser.add_argument("--output", type=Path, help="optional detailed JSON audit")
    args = parser.parse_args()
    upstream_bytes = subprocess.check_output([
        "git", "-C", str(args.reference), "show", f"{WINUI_COMMIT}:{WINUI_PATH}"
    ])
    assert blob(upstream_bytes) == WINUI_BLOB, "the XML must match the pinned Git blob"
    upstream = upstream_bytes.decode("utf-8")
    entries = ET.fromstring(upstream).findall("./members/member")
    properties = {
        item.attrib["name"].rsplit(".", 1)[-1]: item
        for item in entries
        if item.attrib["name"].startswith("P:Microsoft.UI.Text.FontWeights.")
    }
    assert set(properties) == NAMES, "audit every pinned static property"
    source_path = "src/FerroUI.Base/media/font_weight.rs"
    metadata_path = "src/FerroUI.Base/markup_types/values.rs"
    source = (args.repo / source_path).read_text()
    metadata = (args.repo / metadata_path).read_text()
    const_values = dict(re.findall(r"pub const (\w+): FontWeight = FontWeight\((\d+)\);", source))
    font_metadata = metadata.split("ferro_markup_type!(struct FontWeight {", 1)[1].split("});", 1)[0]
    fields = dict(re.findall(r"(\w+): FontWeight => \|\| FontWeight::(\w+),", font_metadata))
    parse_names = dict(re.findall(r'\("(\w+)", FontWeight::(\w+)\)', source))
    rows = []
    for name, entry in sorted(properties.items()):
        summary = "".join(entry.find("summary").itertext()).strip()
        returns = "".join(entry.find("returns").itertext()).strip()
        summary_value = int(re.fullmatch(r"Specifies a font weight value of (\d+)\.", summary)[1])
        return_value = int(re.fullmatch(r"A FontWeight with a Weight value of (\d+)\.", returns)[1])
        assert summary_value == return_value == int(const_values[name]), name
        assert fields[name] == name, f"metadata field {name} must return the same typed constant"
        assert parse_names[name] == name, f"named parser entry missing for {name}"
        anchor = f"pub const {name}: FontWeight = FontWeight({summary_value});"
        metadata_anchor = f"{name}: FontWeight => || FontWeight::{name},"
        rows.append({
            "id": entry.attrib["name"], "name": name, "native_value": summary_value,
            "rust_value": int(const_values[name]),
            "target": {"path": source_path, "symbol": f"pub const {name}:", "line": line(source, anchor)},
            "markup": {"path": metadata_path, "symbol": metadata_anchor, "line": line(metadata, metadata_anchor)},
            "named_parser_entry": f'("{name}", FontWeight::{name})',
            "pinned_xml_line": line(upstream, f'<member name="{entry.attrib["name"]}">'),
        })
    assert 'pub struct FontWeight(pub i32);' in source
    assert 'pub const fn value(self) -> i32' in source
    assert 'impl FromStr for FontWeight' in source
    assert 'impl From<i32> for FontWeight' in source
    assert 'namespace: "FerroUI.Media"' in font_metadata
    assert 'parse: |s: &str| s.parse::<FontWeight>()' in font_metadata
    assert '<FontWeight as MarkupTyped>::MARKUP' in metadata
    assert 'ValueTypes::register_nullable::<FontWeight>()' in metadata
    assert 'ValueTypes::register_display::<FontWeight>()' in metadata
    assert 'pub use font_weight::FontWeight;' in (args.repo / 'src/FerroUI.Base/media/mod.rs').read_text()
    assert 'crate::media::FontWeight,' in (args.repo / 'src/FerroUI.Base/rust_paths.rs').read_text()
    tests = [
        (source_path, 'fn aliases_and_parsing(', 'Bold named parsing; Regular/Heavy aliases; numeric 450; display; invalid text'),
        ('src/FerroUI.Base/markup_types/markup_types_tests.rs', 'fn parse_converts_a_sample_of_each_text_convertible_type(', 'FontWeight Bold is in PARSE_SAMPLES; typed metadata parse'),
        ('src/FerroUI.Base/markup_types/markup_types_tests.rs', 'fn the_types_written_as_elements_with_text_convert_from_their_own_text(', 'FontWeight Bold is resolved and parsed as a value element'),
        ('src/FerroUI.Base/markup_types/markup_types_tests.rs', 'fn handles_resolve_to_their_type(', 'Registered value and nullable handles resolve to matching metadata'),
        ('src/FerroUI.Base/markup_types/markup_types_tests.rs', 'fn every_value_type_has_its_default_value_constructor(', 'FontWeight struct default constructor creates its declared value type'),
        ('tests/FerroUI.Markup.Xaml.UnitTests/emitter/differential_tests.rs', 'fn both_back_ends_build_equal_object_trees(', 'Corpus includes Bold Run/TextBlock and numeric FontWeight=600; interpreted/compiled trees compared'),
        ('tests/FerroUI.Markup.Xaml.UnitTests/emitter/differential_tests.rs', 'fn generated_output_is_up_to_date(', 'Generated corpus parses FontWeight Bold and 600 through the existing Rust type'),
    ]
    anchors = []
    for path, symbol, covers in tests:
        content = (args.repo / path).read_text()
        anchors.append({"path": path, "symbol": symbol, "line": line(content, symbol), "covers": covers})
    corpus = (args.repo / 'tests/FerroUI.Markup.Xaml.UnitTests/emitter/corpus.rs').read_text()
    for name in ['text_block_font_style_weight.xaml', 'text_block_inlines.xaml', 'font_weight_number.xaml']:
        assert f'("{name}",' in corpus
    result = {
        "scope": "Read-only static audit of eleven pinned FontWeights values and existing FerroUI API/metadata/test anchors",
        "upstream": {"repository": "microsoft/microsoft-ui-xaml", "commit": WINUI_COMMIT, "path": WINUI_PATH, "git_blob": WINUI_BLOB},
        "source_blobs": {path: blob((args.repo / path).read_bytes()) for path in [source_path, metadata_path]},
        "rows": rows, "test_anchors": anchors,
        "runtime_executed_by_this_audit": False,
        "limits": [
            "Existing Rust tests do not individually assert all eleven constants against WinUI. This independent pin-to-source audit supplies that comparison.",
            "FontWeight is metadata Struct with associated/static fields; the Enum-only enumeration_members_are_found_by_name test does not cover it.",
            "No Microsoft.UI.Text.FontWeights markup namespace alias, static-class ABI, COM activation, or native FontWeight memory layout is asserted.",
            "The existing open i32 wrapper and numeric parser accept arbitrary integers; this audit concerns the eleven named values, not native weight validation.",
        ],
    }
    if args.output:
        args.output.write_text(json.dumps(result, indent=2) + "\n")
    print(f"PASS: {len(rows)} pinned values, {len(rows)} typed metadata fields, {len(rows)} named-parser entries; {len(anchors)} existing test anchors verified.")
    print("Static audit only; no Rust or native runtime tests executed.")


if __name__ == '__main__':
    main()
