#!/usr/bin/env python3
"""Compare the actual pinned C++ and current Rust line-stacking functions.

Only the primitive types and TextLine::GetHeight dependency are supplied by the
harness. Neither function body is translated or reimplemented by this script.
This checks arithmetic, not native shaping, control layout, or rendered pixels.
"""

import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import tempfile


PIN = "fe244ca6ac4d482712b234a9de3cf771cc6481de"
NATIVE_PATH = "dxaml/xcp/core/text/BlockLayout/ParagraphNode.cpp"
RUST_PATH = "src/FerroUI.Controls/rich_text_layout.rs"
ROOT = Path(__file__).resolve().parents[2]


def function(source, marker):
    start = source.index(marker)
    brace = source.index("{", start)
    depth = 1
    cursor = brace + 1
    # These two bounded functions have no string literals or braces in comments.
    # A change to their source remains visible in the hashes in the result.
    while depth:
        if cursor >= len(source):
            raise ValueError("unterminated function: " + marker)
        depth += (source[cursor] == "{") - (source[cursor] == "}")
        cursor += 1
    return source[start:cursor]


CPP_PREFIX = r"""
#include <algorithm>
#include <iomanip>
#include <iostream>
#define _In_
#define _Out_
#define MAX std::max
using XFLOAT = float;
namespace DirectUI { enum LineStackingStrategy { MaxHeight, BlockLineHeight, BaselineToBaseline }; }
using DirectUI::LineStackingStrategy;
namespace RichTextServices { struct TextLine { float height; float GetHeight() const { return height; } }; }
struct ParagraphNode { static void ApplyLineStackingStrategy(LineStackingStrategy, XFLOAT, XFLOAT, XFLOAT, RichTextServices::TextLine*, bool, bool, XFLOAT*, XFLOAT*); };
"""

CPP_MAIN = r"""
int main() {
    std::cout << std::setprecision(10);
    for (auto strategy : {DirectUI::MaxHeight, DirectUI::BlockLineHeight, DirectUI::BaselineToBaseline})
    for (float baseline : {700.0f, 800.0f})
    for (float font_advance : {1000.0f, 1100.0f})
    for (float height : {0.0f, 6.0f, 15.0f, 26.0f, 55.0f})
    for (float natural : {11.0f, 22.0f, 33.0f, 44.0f})
    for (bool first : {false, true})
    for (bool last : {false, true}) {
        RichTextServices::TextLine line {natural};
        float advance, offset;
        ParagraphNode::ApplyLineStackingStrategy(strategy, baseline, font_advance,
            height, &line, first, last, &advance, &offset);
        std::cout << advance << " " << offset << "\n";
    }
}
"""

RUST_PREFIX = r"""
#[derive(Clone, Copy)]
enum LineStackingStrategy { MaxHeight, BlockLineHeight, BaselineToBaseline }
struct LineStackingMetrics { advance: f64, offset: f64 }
"""

RUST_MAIN = r"""
fn main() {
    for strategy in [LineStackingStrategy::MaxHeight, LineStackingStrategy::BlockLineHeight, LineStackingStrategy::BaselineToBaseline] {
    for baseline in [700.0, 800.0] {
    for font_advance in [1000.0, 1100.0] {
    for height in [0.0, 6.0, 15.0, 26.0, 55.0] {
    for natural in [11.0, 22.0, 33.0, 44.0] {
    for first in [false, true] {
    for last in [false, true] {
        let value = apply_line_stacking_strategy(strategy, baseline / font_advance,
            height, natural, first, last);
        println!("{:.10} {:.10}", value.advance, value.offset);
    }}}}}}}
}
"""


def run(arguments):
    return subprocess.check_output(arguments, text=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--cxx", default="c++")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    native_source = run(["git", "-C", str(args.reference), "show", f"{PIN}:{NATIVE_PATH}"])
    rust_source = (ROOT / RUST_PATH).read_text()
    native = function(native_source, "void ParagraphNode::ApplyLineStackingStrategy(")
    rust = function(rust_source, "fn apply_line_stacking_strategy(")
    with tempfile.TemporaryDirectory(prefix="ferroui-line-stacking-") as temporary:
        directory = Path(temporary)
        cpp_file, rust_file = directory / "oracle.cpp", directory / "candidate.rs"
        cpp_file.write_text(CPP_PREFIX + native + CPP_MAIN)
        rust_file.write_text(RUST_PREFIX + rust + RUST_MAIN)
        oracle, candidate = directory / "oracle", directory / "candidate"
        subprocess.run([args.cxx, "-std=c++17", "-O0", str(cpp_file), "-o", str(oracle)], check=True)
        subprocess.run([args.rustc, "--edition=2021", str(rust_file), "-o", str(candidate)], check=True)
        expected = [tuple(map(float, row.split())) for row in run([str(oracle)]).splitlines()]
        actual = [tuple(map(float, row.split())) for row in run([str(candidate)]).splitlines()]
    if len(expected) != 960 or len(actual) != len(expected):
        raise ValueError(f"unexpected row counts: C++={len(expected)}, Rust={len(actual)}")
    largest = 0.0
    for index, (left, right) in enumerate(zip(expected, actual)):
        if len(left) != 2 or len(right) != 2:
            raise ValueError(f"invalid output at row {index}")
        for native_value, rust_value in zip(left, right):
            if not math.isclose(native_value, rust_value, rel_tol=1e-6, abs_tol=1e-5):
                raise ValueError(f"line-stacking mismatch at row {index}: {left} != {right}")
            largest = max(largest, abs(native_value - rust_value))
    result = {
        "result": "PASS", "cases": len(expected), "upstream_pin": PIN,
        "native_path": NATIVE_PATH, "rust_path": RUST_PATH,
        "native_function_sha256": hashlib.sha256(native.encode()).hexdigest(),
        "rust_function_sha256": hashlib.sha256(rust.encode()).hexdigest(),
        "largest_absolute_error": largest,
        "tolerance": "relative 1e-6 or absolute 1e-5; native float versus Rust f64",
        "scope": "actual extracted source functions; no framework or native rendering execution",
        "inputs": {
            "strategies": ["MaxHeight", "BlockLineHeight", "BaselineToBaseline"],
            "font_baseline": [700, 800], "font_advance": [1000, 1100],
            "line_height": [0, 6, 15, 26, 55], "natural_height": [11, 22, 33, 44],
            "first_line": [False, True], "last_line": [False, True],
        },
    }
    encoded = json.dumps(result, indent=2) + "\n"
    if args.output:
        args.output.write_text(encoded)
    print(encoded, end="")


if __name__ == "__main__":
    main()
