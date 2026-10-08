#!/usr/bin/env python3
"""Generates the two substitute fonts of the glyph typeface tests.

Avalonia's `GlyphTypefaceTests.cs` loads `MiSans-Normal.ttf` (a CJK font with a
`vmtx` table) and `NISC18030.ttf` (a bitmap font without a `head` table). Neither
file states a licence that allows redistributing it here, so the port's tests
load two small fonts derived from WenQuanYi Micro Hei (Apache License 2.0),
which the ControlCatalog sample already ships:

- `WenQuanYiMicroHei-Subset.ttf`: the font subset to U+0020, U+4E2D, U+6587 and
  U+5B57, keeping `vhea` and `vmtx` (the shape of `MiSans-Normal.ttf` the tests
  rely on).
- `WenQuanYiMicroHei-NoHead.ttf`: the same subset reduced to the tables of
  `NISC18030.ttf` that a glyph typeface reads (`OS/2`, `cmap`, `maxp`, `name`,
  `post`), with the `head` table renamed to `bhed` as in that bitmap font.

Requires fontTools (`pip install fonttools`). Run from the repository root:

    python3 scripts/generate_glyph_typeface_test_fonts.py

The output is deterministic for a given fontTools version; the generated files
are committed under `src/FerroUI.Base/test_assets/fonts/`.
"""

import io
import os
import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.ttLib.sfnt import SFNTReader, SFNTWriter

SOURCE = "samples/ControlCatalog/Assets/Fonts/WenQuanYiMicroHei-01.ttf"
OUT_DIR = "src/FerroUI.Base/test_assets/fonts"
CODEPOINTS = [0x20, 0x4E2D, 0x6587, 0x5B57]
NO_HEAD_TABLES = ["OS/2", "cmap", "maxp", "name", "post"]


def make_subset():
    options = subset.Options()
    options.layout_features = []
    options.name_IDs = ["*"]
    options.name_languages = ["*"]
    options.notdef_outline = True
    options.recalc_timestamp = False
    options.drop_tables += ["FFTM", "GDEF", "GPOS", "GSUB", "fpgm", "prep", "cvt ", "gasp"]

    font = subset.load_font(SOURCE, options)
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=CODEPOINTS)
    subsetter.subset(font)

    if "vmtx" not in font or "vhea" not in font:
        sys.exit("the subset lost its vertical metrics")

    buffer = io.BytesIO()
    font.save(buffer, reorderTables=True)
    return buffer.getvalue()


def make_no_head(subset_bytes):
    reader = SFNTReader(io.BytesIO(subset_bytes))
    tables = {tag: reader[tag] for tag in NO_HEAD_TABLES}
    tables["bhed"] = reader["head"]

    buffer = io.BytesIO()
    writer = SFNTWriter(buffer, len(tables), sfntVersion=reader.sfntVersion)
    for tag in sorted(tables):
        writer[tag] = tables[tag]
    writer.close()
    return buffer.getvalue()


def main():
    subset_bytes = make_subset()
    no_head_bytes = make_no_head(subset_bytes)

    # Check that both parse.
    TTFont(io.BytesIO(subset_bytes))
    TTFont(io.BytesIO(no_head_bytes), lazy=True)

    with open(os.path.join(OUT_DIR, "WenQuanYiMicroHei-Subset.ttf"), "wb") as f:
        f.write(subset_bytes)
    with open(os.path.join(OUT_DIR, "WenQuanYiMicroHei-NoHead.ttf"), "wb") as f:
        f.write(no_head_bytes)


if __name__ == "__main__":
    main()
