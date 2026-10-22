#!/usr/bin/env python3
"""Compare the pinned C++ family-bounds transform with the actual Rust helper.

The harness supplies already-resolved family metrics and primitive dependencies.
It compiles the unchanged native function and the entire current Rust helper;
neither formula is translated or reimplemented here. This checks arithmetic,
not font lookup, LineServices aggregation, native controls or rendered pixels.
"""

import argparse
import hashlib
import itertools
import json
import math
from pathlib import Path
import subprocess
import tempfile


PIN = "fe244ca6ac4d482712b234a9de3cf771cc6481de"
NATIVE_PATH = "dxaml/xcp/core/text/Fonts/CompositeFontFamily.cpp"
NATIVE_SHA256 = "66d3866fcd25ab993a358f5b4433f1dc30cb7bdb5331398e4f0f4b64c6d9da9d"
RUST_PATH = "src/FerroUI.Base/media/text_formatting/font_family_line_metrics.rs"
ROOT = Path(__file__).resolve().parents[2]
ABSOLUTE_TOLERANCE = 1e-5
RELATIVE_TOLERANCE = 1e-6
MATRIX = {
    "design_em": [1000, 2048],
    "ascent": [600, 800, 950],
    "descent": [100, 200, 400],
    "gap": [0, 100, 350],
    "cap_ratio": [0.55, 0.7, 0.9],
    "size": [10, 12, 20, 33, 72],
    "bounds_modes": [0, 1, 2, 3],
}


def sha256(value):
    return hashlib.sha256(value).hexdigest()


def function(source, marker):
    start = source.index(marker)
    brace = source.index("{", start)
    depth = 1
    cursor = brace + 1
    # This pinned bounded function has no braces in strings or comments.
    # Its complete file and extracted function hashes are recorded below.
    while depth:
        if cursor >= len(source):
            raise ValueError("unterminated function: " + marker)
        depth += (source[cursor] == "{") - (source[cursor] == "}")
        cursor += 1
    return source[start:cursor]


CPP_PREFIX = r"""#include <iostream>
#include <iomanip>
#define _Check_return_
#define _In_
#define _Out_
#define IFC_RETURN(expr) do { if ((expr) != 0) return 1; } while (false)
using HRESULT = int;
using XFLOAT = float;
constexpr int S_OK = 0;
namespace DirectUI { enum class TextLineBounds { Full, TrimToCapHeight, TrimToBaseline, Tight }; }
using TextLineBounds = DirectUI::TextLineBounds;
class CCompositeFontFamily {
public:
    float baseline, height, cap;
    HRESULT GetLineMetrics(float* b, float* h, float* c) { *b=baseline; *h=height; *c=cap; return 0; }
    HRESULT GetTextLineBoundsMetrics(TextLineBounds, float*, float*);
};
"""

CPP_MAIN = r"""
int main() {
    int id,em,a,d,g,c,mode; float size;
    std::cout << std::setprecision(17);
    while (std::cin >> id >> em >> a >> d >> g >> c >> size >> mode) {
        CCompositeFontFamily family;
        family.baseline = float(a) / float(em);
        family.height = float(a+d+g) / float(em);
        family.cap = float(c) / float(em);
        float baseline=0,height=0;
        family.GetTextLineBoundsMetrics(static_cast<TextLineBounds>(mode), &baseline, &height);
        std::cout << id << ' ' << baseline*size << ' ' << height*size << '\n';
    }
}
"""

RUST_PREFIX = r"""#![allow(dead_code)]
mod media {
    #[derive(Clone)] pub struct FontFamily;
    pub struct Typeface;
    impl Typeface {
        pub fn new(_: FontFamily) -> Self { Self }
        pub fn glyph_typeface(&self) -> GlyphTypeface { GlyphTypeface }
    }
    pub struct GlyphTypeface;
    impl GlyphTypeface { pub fn metrics(&self) -> FontMetrics { unreachable!() } }
    #[derive(Clone, Copy)] pub struct FontMetrics {
        pub design_em_height: u16, pub ascent: i32, pub descent: i32,
        pub line_gap: i32, pub cap_height: i32,
    }
    impl FontMetrics {
        pub fn line_spacing(&self) -> i32 { self.descent - self.ascent + self.line_gap }
    }
    pub enum TextLineBounds { Full, TrimToCapHeight, TrimToBaseline, Tight }
}
#[path = "font_family_line_metrics.rs"] mod font_family_line_metrics;
"""

RUST_MAIN = r"""use std::io::{self, BufRead};
fn main() {
    for line in io::stdin().lock().lines() {
        let line=line.unwrap();
        let v:Vec<&str>=line.split_whitespace().collect();
        let int=|i:usize| v[i].parse::<i32>().unwrap();
        let metrics=media::FontMetrics {design_em_height:int(1) as u16, ascent:-int(2), descent:int(3), line_gap:int(4), cap_height:int(5)};
        let mode=match int(7) {0=>media::TextLineBounds::Full,1=>media::TextLineBounds::TrimToCapHeight,2=>media::TextLineBounds::TrimToBaseline,_=>media::TextLineBounds::Tight};
        let box_metrics=font_family_line_metrics::FontFamilyLineMetrics::from_font_metrics(metrics,v[6].parse().unwrap(),mode);
        println!("{} {:.17} {:.17}",int(0),box_metrics.baseline,box_metrics.height);
    }
}
"""


def inputs():
    rows = []
    for em, ascent, descent, gap, cap_ratio, size, mode in itertools.product(
        *(MATRIX[key] for key in MATRIX)
    ):
        rows.append(
            [len(rows), em, ascent, descent, gap, round(ascent * cap_ratio), size, mode]
        )
    if len(rows) != 3240:
        raise ValueError("unexpected input matrix size")
    return "".join(" ".join(map(str, row)) + "\n" for row in rows)


def output_rows(output, label):
    rows = []
    for index, row in enumerate(output.splitlines()):
        fields = row.split()
        if len(fields) != 3 or int(fields[0]) != index:
            raise ValueError(f"invalid {label} output at row {index}: {row}")
        values = tuple(map(float, fields[1:]))
        if not all(math.isfinite(value) for value in values):
            raise ValueError(f"nonfinite {label} output at row {index}: {row}")
        rows.append(values)
    if len(rows) != 3240:
        raise ValueError(f"unexpected {label} row count: {len(rows)}")
    return rows


def compare(native_output, rust_output):
    native_rows = output_rows(native_output, "C++")
    rust_rows = output_rows(rust_output, "Rust")
    largest = 0.0
    for index, (native, rust) in enumerate(zip(native_rows, rust_rows)):
        for field, (expected, actual) in enumerate(zip(native, rust)):
            error = abs(expected - actual)
            largest = max(largest, error)
            if error > ABSOLUTE_TOLERANCE + RELATIVE_TOLERANCE * abs(expected):
                raise ValueError(
                    f"family-metrics mismatch at row {index}, field {field}: "
                    f"{native} != {rust}"
                )
    return largest


def result(native_bytes, native_function, rust_bytes, native_output, rust_output):
    return {
        "schema_version": 1,
        "result": "PASS",
        "cases": 3240,
        "upstream_pin": PIN,
        "native_path": NATIVE_PATH,
        "rust_path": RUST_PATH,
        "native_source_sha256": sha256(native_bytes),
        "native_function_sha256": sha256(native_function.encode()),
        "rust_source_sha256": sha256(rust_bytes),
        "input_rows_sha256": sha256(inputs().encode()),
        "native_output_sha256": sha256(native_output.encode()),
        "rust_output_sha256": sha256(rust_output.encode()),
        "maximum_absolute_error": compare(native_output, rust_output),
        "absolute_tolerance": ABSOLUTE_TOLERANCE,
        "relative_tolerance": RELATIVE_TOLERANCE,
        "comparison": "absolute error <= absolute tolerance + relative tolerance * abs(native value)",
        "scope": "Actual pinned C++ family-bounds function and current Rust helper; no font lookup, LineServices aggregation, native controls or rendered pixels.",
        "assumptions": [
            "Native GetLineMetrics receives supplied normalized ascent, total spacing and cap height.",
            "Rust receives the same design metrics; the ascent sign follows FerroUI's font convention.",
            "The native calculation uses float and the Rust calculation uses f64.",
            "Cap height is round(ascent * cap_ratio); modes 0, 1, 2, 3 are Full, TrimToCapHeight, TrimToBaseline and Tight.",
        ],
        "inputs": MATRIX,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    reference = parser.add_mutually_exclusive_group(required=True)
    reference.add_argument("--reference", type=Path, help="WinUI Git repository containing the pinned commit")
    reference.add_argument("--source-root", type=Path, help="extracted pinned source tree; exact source hash required")
    parser.add_argument("--rust-root", type=Path, default=ROOT)
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--cxx", default="c++")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.reference:
        native_bytes = subprocess.check_output(
            ["git", "-C", str(args.reference), "show", f"{PIN}:{NATIVE_PATH}"]
        )
    else:
        native_bytes = (args.source_root / NATIVE_PATH).read_bytes()
    if sha256(native_bytes) != NATIVE_SHA256:
        raise ValueError("native source does not match the pinned file hash")
    native_function = function(
        native_bytes.decode("utf-8-sig"),
        "_Check_return_ HRESULT CCompositeFontFamily::GetTextLineBoundsMetrics(",
    )
    rust_bytes = (args.rust_root / RUST_PATH).read_bytes()
    with tempfile.TemporaryDirectory(prefix="ferroui-family-metrics-") as temporary:
        directory = Path(temporary)
        cpp_file, rust_file = directory / "oracle.cpp", directory / "candidate.rs"
        cpp_file.write_text(CPP_PREFIX + native_function + CPP_MAIN, encoding="utf-8")
        rust_file.write_text(RUST_PREFIX + RUST_MAIN, encoding="utf-8")
        (directory / "font_family_line_metrics.rs").write_bytes(rust_bytes)
        oracle, candidate = directory / "oracle", directory / "candidate"
        subprocess.run([args.cxx, "-std=c++17", "-O0", str(cpp_file), "-o", str(oracle)], check=True)
        subprocess.run([args.rustc, "--edition=2021", str(rust_file), "-o", str(candidate)], check=True)
        native_output = subprocess.check_output([str(oracle)], input=inputs(), text=True)
        rust_output = subprocess.check_output([str(candidate)], input=inputs(), text=True)
    record = result(native_bytes, native_function, rust_bytes, native_output, rust_output)
    record["execution"] = "compiled current sources with this script"
    encoded = json.dumps(record, indent=2) + "\n"
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    print(encoded, end="")


if __name__ == "__main__":
    main()
