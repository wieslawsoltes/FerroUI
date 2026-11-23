use crate::harf_buzz_text_shaper::{cached_cultures, language_of, MAX_CACHED_CULTURES};
use crate::{hb, HarfBuzzTextShaper};
use ferroui_base::media::fonts::testing::TestPlatformTypeface;
use ferroui_base::media::text_formatting::TextShaperOptions;
use ferroui_base::media::{FontSimulations, GlyphTypeface};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::{Locale, ReadOnlyMemory};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use std::rc::Rc;

const INTER_FONT: &[u8] = include_bytes!("../../FerroUI.Base/test_assets/fonts/Inter-Regular.ttf");

fn be16(output: &mut Vec<u8>, values: &[u16]) {
    values.iter().for_each(|value| output.extend_from_slice(&value.to_be_bytes()));
}

/// A `GSUB` whose `latn` script has the language systems `ZHS ` and `ZHT `, each with a `locl`
/// that replaces `from` by a glyph of its own.
fn chinese_gsub(from: u16, simplified: u16, traditional: u16) -> Vec<u8> {
    let mut scripts = Vec::new();
    be16(&mut scripts, &[1]);
    scripts.extend_from_slice(b"latn");
    be16(&mut scripts, &[8, 0, 2]);
    scripts.extend_from_slice(b"ZHS ");
    be16(&mut scripts, &[16]);
    scripts.extend_from_slice(b"ZHT ");
    be16(&mut scripts, &[24]);
    be16(&mut scripts, &[0, 0xFFFF, 1, 0]);
    be16(&mut scripts, &[0, 0xFFFF, 1, 1]);
    let mut features = Vec::new();
    be16(&mut features, &[2]);
    features.extend_from_slice(b"locl");
    be16(&mut features, &[14]);
    features.extend_from_slice(b"locl");
    be16(&mut features, &[20]);
    be16(&mut features, &[0, 1, 0]);
    be16(&mut features, &[0, 1, 1]);
    let mut lookups = Vec::new();
    be16(&mut lookups, &[2, 6, 28]);
    for target in [simplified, traditional] {
        be16(&mut lookups, &[1, 0, 1, 8]);
        be16(&mut lookups, &[2, 8, 1, target]);
        be16(&mut lookups, &[1, 1, from]);
    }
    let mut gsub = Vec::new();
    let feature_at = 10 + scripts.len() as u16;
    let lookup_at = feature_at + features.len() as u16;
    be16(&mut gsub, &[1, 0, 10, feature_at, lookup_at]);
    gsub.extend_from_slice(&scripts);
    gsub.extend_from_slice(&features);
    gsub.extend_from_slice(&lookups);
    gsub
}

/// `font` with its `GSUB` replaced.
fn with_gsub(font: &[u8], gsub: &[u8]) -> Vec<u8> {
    let be_u16 = |at: usize| u16::from_be_bytes([font[at], font[at + 1]]) as usize;
    let be_u32 = |at: usize| u32::from_be_bytes([font[at], font[at + 1], font[at + 2], font[at + 3]]) as usize;
    let count = be_u16(4);
    let mut tables: Vec<([u8; 4], &[u8])> = (0..count)
        .map(|index| {
            let record = 12 + index * 16;
            let tag: [u8; 4] = font[record..record + 4].try_into().unwrap();
            let table: &[u8] = if &tag == b"GSUB" { gsub } else { &font[be_u32(record + 8)..][..be_u32(record + 12)] };
            (tag, table)
        })
        .collect();
    tables.sort_by_key(|(tag, _)| *tag);
    let mut output = font[..12].to_vec();
    let mut offset = 12 + count * 16;
    for (tag, table) in &tables {
        output.extend_from_slice(tag);
        output.extend_from_slice(&[0; 4]);
        output.extend_from_slice(&(offset as u32).to_be_bytes());
        output.extend_from_slice(&(table.len() as u32).to_be_bytes());
        offset += table.len().next_multiple_of(4);
    }
    for (_, table) in &tables {
        output.extend_from_slice(table);
        output.resize(output.len().next_multiple_of(4), 0);
    }
    output
}

#[test]
fn the_script_and_region_of_a_culture_reach_harfbuzz() {
    let shaper = Rc::new(HarfBuzzTextShaper::new());
    let _scope = UnitTestApplication::start(
        TestServices::mock_threading_interface().with_text_shaper_impl(shaper.clone()),
    );
    let load = |bytes: Vec<u8>| {
        let platform = TestPlatformTypeface::from_bytes(bytes, FontSimulations::None).expect("the font loads");
        GlyphTypeface::new(platform, FontSimulations::None).expect("the font has tables")
    };
    let plain = load(INTER_FONT.to_vec());
    let glyph = |character: char| plain.character_to_glyph_map().try_get_glyph(character as i32).unwrap();
    let (a, b, c) = (glyph('a'), glyph('b'), glyph('c'));
    let font = load(with_gsub(INTER_FONT, &chinese_gsub(a, b, c)));
    let shaped = |culture: &str| {
        let culture = Some(culture.parse::<Locale>().unwrap());
        let options = TextShaperOptions::with_all(font.clone(), 16.0, 0, culture, 0.0, 0.0, None);
        let text = ReadOnlyMemory::from_vec("a".encode_utf16().collect());
        shaper.shape_text(&text, &options).glyph_infos()[0].glyph_index
    };
    assert_eq!(shaped("zh-Hans"), b);
    assert_eq!(shaped("zh-Hant"), c);
    assert_eq!(shaped("zh-TW"), c);
    assert_eq!(shaped("zh"), b, "Chinese alone stays Simplified");
    assert_eq!(shaped("en"), a, "a culture with no language system of the font");
}

#[test]
fn the_language_comes_from_the_culture_without_its_extensions() {
    let of = |culture: &str| language_of(&culture.parse::<Locale>().unwrap());
    assert!(of("en") == hb::Language::from_tag("en"));
    assert!(of("zh-Hant") == hb::Language::from_tag("zh-Hant"));
    assert!(of("en-u-ca-gregory") == hb::Language::from_tag("en"));
    assert!(language_of(&Locale::UNKNOWN) == hb::Language::from_tag("iv"), "the invariant culture");
}

#[test]
fn the_cache_of_cultures_is_bounded() {
    for index in 0..MAX_CACHED_CULTURES * 3 {
        let first = (b'a' + (index / 26) as u8) as char;
        let second = (b'a' + (index % 26) as u8) as char;
        language_of(&format!("{first}{second}").parse::<Locale>().unwrap());
    }
    assert!(cached_cultures() <= MAX_CACHED_CULTURES);
}
