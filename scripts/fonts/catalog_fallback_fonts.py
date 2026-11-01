#!/usr/bin/env python3
"""Writes the fallback fonts of the ControlCatalog for hosts without system fonts
(`samples/ControlCatalog/Assets/FallbackFonts`).

The browser has no system fonts: a character the font of a text lacks (Inter, the default family
of the browser catalog) is drawn as a missing-glyph box unless a font of the application covers it.
The catalog's pages show emoji, Arabic, CJK and a few symbols that Inter lacks, so `App::
font_fallbacks` (`samples/ControlCatalog/app.rs`) names these fonts as the font fallbacks of the
browser host. Each is a subset of an open font, small enough to load at start-up:

    NotoColorEmoji-Catalog.ttf          Noto Color Emoji (COLRv1, vector), the emoji the catalog
                                        shows, with their skin tone and ZWJ sequences (SIL OFL 1.1)
    NotoSansArabic-Catalog.ttf          Noto Sans Arabic, the Arabic block (SIL OFL 1.1)
    WenQuanYiMicroHei-Catalog.ttf       WenQuanYi Micro Hei (`Assets/Fonts/WenQuanYiMicroHei-01.ttf`),
                                        the CJK characters of the catalog's sources (Apache 2.0)
    FerroUICatalogSymbolsFallback.ttf   the symbols of the catalog's sources that Inter lacks, from
                                        Noto Sans Math, Noto Sans Symbols and Noto Sans Symbols 2
                                        (SIL OFL 1.1)

The source fonts are not in the repository; download them into a directory first:

    https://github.com/googlefonts/noto-emoji/raw/main/2D/fonts/Noto-COLRv1.ttf
    https://github.com/notofonts/notofonts.github.io/raw/main/fonts/NotoSansArabic/unhinted/ttf/NotoSansArabic-Regular.ttf
    https://github.com/notofonts/notofonts.github.io/raw/main/fonts/NotoSansMath/unhinted/ttf/NotoSansMath-Regular.ttf
    https://github.com/notofonts/notofonts.github.io/raw/main/fonts/NotoSansSymbols/unhinted/ttf/NotoSansSymbols-Regular.ttf
    https://github.com/notofonts/notofonts.github.io/raw/main/fonts/NotoSansSymbols2/unhinted/ttf/NotoSansSymbols2-Regular.ttf

    catalog_fallback_fonts.py <directory of the source fonts>

Needs fontTools (`pip install fonttools`). A new emoji, symbol or CJK character in the catalog's
text needs a run of the script (emoji and symbols: add them to the lists below).
"""

import os
import sys

from fontTools import subset
from fontTools.merge import Merger
from fontTools.ttLib import TTFont

REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CATALOG = os.path.join(REPOSITORY, "samples/ControlCatalog")
OUTPUT = os.path.join(CATALOG, "Assets/FallbackFonts")
WENQUANYI = os.path.join(CATALOG, "Assets/Fonts/WenQuanYiMicroHei-01.ttf")

# The emoji of the catalog's pages, the skin tone modifiers, ZWJ, the emoji presentation selector,
# and the symbols with an emoji presentation the pages show (heavy plus and minus, star, crab).
EMOJI = [0x1F600, 0x1F44C, *range(0x1F466, 0x1F46B), *range(0x1F3FB, 0x1F400), 0x200D, 0xFE0F,
         0x2795, 0x2796, 0x2B50, 0x1F980]
# The Arabic block and the joining controls.
ARABIC = [*range(0x0600, 0x0700), *range(0x200C, 0x2010)]
# The symbols of the catalog that Inter lacks, by the font that draws them.
SYMBOLS = {
    "NotoSansMath-Regular.ttf": [0x2295, 0x2500],
    "NotoSansSymbols-Regular.ttf": [0x2315, 0x230A, 0x230B],
    "NotoSansSymbols2-Regular.ttf": [0x2299, 0x25C9, 0x25CE, 0x2726],
}
SYMBOLS_FAMILY = "FerroUI Catalog Symbols Fallback"


def is_cjk(codepoint):
    return (0x2E80 <= codepoint <= 0x9FFF or 0xF900 <= codepoint <= 0xFAFF
            or 0xFF00 <= codepoint <= 0xFFEF)


def catalog_cjk():
    """The CJK characters of the documents and Rust sources of the catalog."""
    found = set()
    for root, _, files in os.walk(CATALOG):
        for name in files:
            if name.endswith((".xaml", ".rs")):
                with open(os.path.join(root, name), encoding="utf-8") as source:
                    found.update(ord(c) for c in source.read() if is_cjk(ord(c)))
    return sorted(found)


def subset_font(source, unicodes, output, layout=True):
    options = subset.Options()
    options.layout_features = ["*"] if layout else []
    options.hinting = False
    options.desubroutinize = True
    options.name_IDs = ["*"]
    options.notdef_outline = True
    font = subset.load_font(source, options)
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=unicodes)
    subsetter.subset(font)
    if output:
        subset.save_font(font, output, options)
    return font


def main():
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    sources = sys.argv[1]
    os.makedirs(OUTPUT, exist_ok=True)

    subset_font(os.path.join(sources, "Noto-COLRv1.ttf"), EMOJI,
                os.path.join(OUTPUT, "NotoColorEmoji-Catalog.ttf"))
    subset_font(os.path.join(sources, "NotoSansArabic-Regular.ttf"), ARABIC,
                os.path.join(OUTPUT, "NotoSansArabic-Catalog.ttf"))
    subset_font(WENQUANYI, catalog_cjk(), os.path.join(OUTPUT, "WenQuanYiMicroHei-Catalog.ttf"))

    # The symbols come from three fonts of the same em: subsets without layout and vertical
    # metrics, merged into one font with a family of its own.
    parts = []
    for name, unicodes in SYMBOLS.items():
        font = subset_font(os.path.join(sources, name), unicodes, None, layout=False)
        for tag in ("MATH", "GSUB", "GPOS", "GDEF", "STAT", "meta", "vhea", "vmtx"):
            if tag in font:
                del font[tag]
        part = os.path.join(OUTPUT, f".part-{len(parts)}.ttf")
        font.save(part)
        parts.append(part)
    merged = Merger().merge(parts)
    for part in parts:
        os.remove(part)
    names = merged["name"]
    for name_id in (1, 2, 4, 6, 16, 17):
        names.removeNames(nameID=name_id)
    names.setName(SYMBOLS_FAMILY, 1, 3, 1, 0x409)
    names.setName("Regular", 2, 3, 1, 0x409)
    names.setName(f"{SYMBOLS_FAMILY} Regular", 4, 3, 1, 0x409)
    names.setName(SYMBOLS_FAMILY.replace(" ", "") + "-Regular", 6, 3, 1, 0x409)
    merged.save(os.path.join(OUTPUT, "FerroUICatalogSymbolsFallback.ttf"))

    for name in sorted(os.listdir(OUTPUT)):
        path = os.path.join(OUTPUT, name)
        font = TTFont(path)
        print(f"{name}: {os.path.getsize(path)} bytes, {len(font.getBestCmap())} code points, "
              f"family {font['name'].getDebugName(1)!r}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
