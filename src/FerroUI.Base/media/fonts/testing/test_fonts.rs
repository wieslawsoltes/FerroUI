//! A set of named test fonts modelled on the font files the upstream tests
//! embed, built in code, and an asset loader serving them as resources.

use std::rc::Rc;

use crate::media::{FontStyle, FontWeight};

use super::{TestAssetLoader, TestFontBuilder};

/// The location of the embedded test fonts.
pub const ASSETS: &str = "resm:FerroUI.UnitTests.Assets?assembly=FerroUI.UnitTests";

const LATIN: [(u32, u32); 2] = [(0x20, 0x7E), (0xA0, 0xFF)];

/// `Inter`, regular: Latin.
pub fn inter_regular() -> TestFontBuilder {
    TestFontBuilder::new("Inter").codepoints(&LATIN)
}

/// `Inter`, bold: Latin.
pub fn inter_bold() -> TestFontBuilder {
    TestFontBuilder::new("Inter").weight(FontWeight::Bold).codepoints(&LATIN)
}

/// `Noto Mono`, regular, fixed pitch: Latin.
pub fn noto_mono() -> TestFontBuilder {
    TestFontBuilder::new("Noto Mono").codepoints(&LATIN).fixed_pitch(true)
}

/// `Noto Sans`, italic: Latin.
pub fn noto_sans_italic() -> TestFontBuilder {
    TestFontBuilder::new("Noto Sans").style(FontStyle::Italic).codepoints(&LATIN)
}

/// `Noto Sans Arabic`, regular: U+0600 to U+06FF, shapes Arabic.
pub fn noto_sans_arabic() -> TestFontBuilder {
    TestFontBuilder::new("Noto Sans Arabic").codepoints(&[(0x600, 0x6FF)]).shaping_scripts(&["arab"])
}

/// `Noto Sans Hebrew`, regular: U+0590 to U+05FF.
pub fn noto_sans_hebrew() -> TestFontBuilder {
    TestFontBuilder::new("Noto Sans Hebrew").codepoints(&[(0x590, 0x5FF)])
}

/// `Noto Sans Tamil`, regular: U+0B80 to U+0BFF.
pub fn noto_sans_tamil() -> TestFontBuilder {
    TestFontBuilder::new("Noto Sans Tamil").codepoints(&[(0xB80, 0xBFF)])
}

/// `Manrope Light` (typographic family `Manrope`), weight 300: Latin.
pub fn manrope_light() -> TestFontBuilder {
    TestFontBuilder::new("Manrope Light").typographic_family_name("Manrope").weight(FontWeight::Light).codepoints(&LATIN)
}

/// `MiSans Normal` (typographic family `MiSans`), weight 305: Latin and a
/// part of the CJK ideographs including U+4E2D and U+534E.
pub fn mi_sans_normal() -> TestFontBuilder {
    TestFontBuilder::new("MiSans Normal")
        .typographic_family_name("MiSans")
        .weight(FontWeight(305))
        .codepoints(&[(0x20, 0x7E), (0x4E00, 0x4EFF), (0x5300, 0x53FF)])
}

/// `Noto Sans SC`, thin: only U+4E2D.
pub fn noto_sans_sc_subset() -> TestFontBuilder {
    TestFontBuilder::new("Noto Sans SC").weight(FontWeight::Thin).codepoints(&[(0x4E2D, 0x4E2D)])
}

/// `Source Serif 4 36pt`, italic: Latin.
pub fn source_serif_italic() -> TestFontBuilder {
    TestFontBuilder::new("Source Serif 4 36pt").style(FontStyle::Italic).codepoints(&LATIN)
}

/// `Twitter Color Emoji`, regular: U+1F300 to U+1F64F.
pub fn twitter_color_emoji() -> TestFontBuilder {
    TestFontBuilder::new("Twitter Color Emoji").codepoints(&[(0x1F300, 0x1F64F)]).advance(1000)
}

/// `Adobe Blank 2 VF R`, a last resort font: U+0000 to U+FFFF is too large
/// for a test font, so it covers U+2A700 to U+2A7FF and the Hebrew block.
pub fn adobe_blank() -> TestFontBuilder {
    TestFontBuilder::new("Adobe Blank 2 VF R").codepoints(&[(0x590, 0x5FF), (0x2A700, 0x2A7FF)]).last_resort(true)
}

/// The embedded fonts by file name, in file name order.
pub fn asset_fonts() -> Vec<(&'static str, TestFontBuilder)> {
    vec![
        ("AdobeBlank2VF.ttf", adobe_blank()),
        ("Inter-Bold.ttf", inter_bold()),
        ("Inter-Regular.ttf", inter_regular()),
        ("Manrope-Light.ttf", manrope_light()),
        ("MiSans-Normal.ttf", mi_sans_normal()),
        ("NotoMono-Regular.ttf", noto_mono()),
        ("NotoSans-Italic.ttf", noto_sans_italic()),
        ("NotoSansArabic-Regular.ttf", noto_sans_arabic()),
        ("NotoSansHebrew-Regular.ttf", noto_sans_hebrew()),
        ("NotoSansTamil-Regular.ttf", noto_sans_tamil()),
        ("SourceSerif4_36pt-Italic.ttf", source_serif_italic()),
        ("TwitterColorEmoji-SVGinOT.ttf", twitter_color_emoji()),
    ]
}

/// The URI of an embedded font file, e.g. `asset_uri("Inter-Regular.ttf")`.
pub fn asset_uri(file_name: &str) -> String {
    format!("resm:FerroUI.UnitTests.Assets.{file_name}?assembly=FerroUI.UnitTests")
}

/// An asset loader serving [`asset_fonts`] at [`ASSETS`].
pub fn asset_loader() -> Rc<TestAssetLoader> {
    let loader = TestAssetLoader::new();

    for (file_name, font) in asset_fonts() {
        loader.add_asset(&asset_uri(file_name), font.build_bytes());
    }

    Rc::new(loader)
}
