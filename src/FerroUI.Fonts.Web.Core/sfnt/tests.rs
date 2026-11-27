use super::*;
use std::path::PathBuf;

/// Fonts of this repository, by path from `src`.
const FONTS: [&str; 12] = [
    "FerroUI.Base/test_assets/fonts/Inter-Regular.ttf",
    "FerroUI.Base/test_assets/fonts/InterVariable.ttf",
    "FerroUI.Base/test_assets/fonts/AdobeBlank2VF.ttf",
    "FerroUI.Base/test_assets/fonts/WenQuanYiMicroHei-Subset.ttf",
    "FerroUI.Fonts.Inter/Assets/Inter-Bold.ttf",
    "FerroUI.Themes.Material/Assets/Roboto-Regular.ttf",
    "FerroUI.Dialogs/Assets/Roboto-Light.ttf",
    "Headless/FerroUI.Headless/BareMinimum.ttf",
    "../samples/ControlCatalog/Assets/Fonts/SourceSansPro-Regular.ttf",
    "../samples/ControlCatalog/Assets/Fonts/FerroUICatalogSymbols.ttf",
    "../samples/ControlCatalog/Assets/FallbackFonts/NotoSansArabic-Catalog.ttf",
    "../samples/ControlCatalog/Assets/FallbackFonts/NotoColorEmoji-Catalog.ttf",
];

fn font(path: &str) -> Vec<u8> {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
    std::fs::read(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
}

fn table<'a>(data: &'a [u8], tag: &Tag) -> Option<&'a [u8]> {
    let (_, records) = read_directory(data, 0, &WebFontLimits::DEFAULT).ok()?;
    records.iter().find(|record| record.tag == *tag).map(|record| &data[record.offset..][..record.length])
}

/// A copy of a font with one table changed, the directory left as it is.
fn with_table_patched(data: &[u8], tag: &Tag, patch: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let (_, records) = read_directory(data, 0, &WebFontLimits::DEFAULT).unwrap();
    let record = records.iter().find(|record| record.tag == *tag).unwrap();
    let mut copy = data.to_vec();
    patch(&mut copy[record.offset..record.offset + record.length]);
    copy
}

fn verify_checksums(data: &[u8]) {
    let count = data.u16_at(4).unwrap() as usize;
    for index in 0..count {
        let entry = 12 + index * 16;
        let tag = &data[entry..entry + 4];
        let (sum, offset, length) =
            (data.u32_at(entry + 4).unwrap(), data.u32_at(entry + 8).unwrap(), data.u32_at(entry + 12).unwrap());
        assert_eq!(offset % 4, 0, "the table {tag:?} is aligned");
        let bytes = &data[offset as usize..(offset + length) as usize];
        let actual =
            if tag == b"head" { checksum(bytes).wrapping_sub(bytes.u32_at(8).unwrap()) } else { checksum(bytes) };
        assert_eq!(actual, sum, "the checksum of {tag:?}");
    }
    assert_eq!(checksum(data), 0xB1B0_AFBA, "the checksum of the whole font");
}

#[test]
fn every_repository_font_passes_and_stays_valid() {
    for path in FONTS {
        let data = font(path);
        let clean = sanitize(&data, &SanitizeOptions::default()).unwrap_or_else(|error| panic!("{path}: {error}"));
        verify_checksums(&clean);
        let again = sanitize(&clean, &SanitizeOptions::default()).unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(clean, again, "{path}: sanitizing is stable");
        for tag in [b"cmap", b"head", b"hhea", b"hmtx", b"maxp"] {
            assert!(table(&clean, tag).is_some(), "{path} keeps {tag:?}");
        }
    }
}

#[test]
fn the_tables_that_matter_are_kept_unchanged() {
    let data = font(FONTS[0]);
    let clean = sanitize(&data, &SanitizeOptions::default()).unwrap();
    for tag in [b"glyf", b"loca", b"GSUB", b"GPOS", b"maxp"] {
        assert_eq!(table(&data, tag), table(&clean, tag), "{tag:?}");
    }
}

#[test]
fn variable_fonts_keep_their_variations() {
    let data = font(FONTS[1]);
    let clean = sanitize(&data, &SanitizeOptions::default()).unwrap();
    for tag in [b"fvar", b"gvar"] {
        assert!(table(&clean, tag).is_some(), "{tag:?}");
    }
}

#[test]
fn stepping_gives_the_same_font() {
    let data = font(FONTS[0]);
    let whole = sanitize(&data, &SanitizeOptions::default()).unwrap();
    let mut sanitizer = Sanitizer::new(&data, SanitizeOptions::default()).unwrap();
    let mut steps = 0;
    let stepped = loop {
        steps += 1;
        if let Some(font) = sanitizer.step(4096).unwrap() {
            break font;
        }
    };
    assert!(steps > 2, "the work is spread over steps");
    assert_eq!(whole, stepped);
}

#[test]
fn stripping_hinting_leaves_a_valid_smaller_font() {
    let data = font(FONTS[5]);
    let options = SanitizeOptions { strip_hinting: true, ..Default::default() };
    let clean = sanitize(&data, &options).unwrap();
    verify_checksums(&clean);
    for tag in [b"fpgm", b"prep", b"cvt "] {
        assert!(table(&clean, tag).is_none(), "{tag:?}");
    }
    assert!(clean.len() < data.len());
    assert_eq!(sanitize(&clean, &SanitizeOptions::default()).map(|font| font.len()), Ok(clean.len()));
}

#[test]
fn a_table_outside_the_list_is_dropped() {
    let data = font(FONTS[0]);
    let mut renamed = data.clone();
    let (_, records) = read_directory(&data, 0, &WebFontLimits::DEFAULT).unwrap();
    let count = data.u16_at(4).unwrap() as usize;
    let gasp = (0..count).map(|index| 12 + index * 16).find(|entry| &data[*entry..*entry + 4] == b"gasp");
    let Some(entry) = gasp else {
        assert!(!records.is_empty());
        return;
    };
    renamed[entry..entry + 4].copy_from_slice(b"XXXX");
    let clean = sanitize(&renamed, &SanitizeOptions::default()).unwrap();
    assert!(table(&clean, b"XXXX").is_none());
    assert!(table(&clean, b"gasp").is_none());
}

#[test]
fn missing_and_broken_required_tables_are_refused() {
    let missing = font("FerroUI.Base/test_assets/fonts/WenQuanYiMicroHei-NoHead.ttf");
    assert_eq!(sanitize(&missing, &SanitizeOptions::default()).err(), Some(FontDecodeError::MissingTable(*b"head")));
    let data = font(FONTS[0]);
    let bad_magic = with_table_patched(&data, b"head", |head| head[12] ^= 0xFF);
    assert_eq!(sanitize(&bad_magic, &SanitizeOptions::default()).err(), Some(FontDecodeError::Malformed("head")));
    let no_glyphs = with_table_patched(&data, b"maxp", |maxp| maxp[4..6].fill(0));
    assert_eq!(sanitize(&no_glyphs, &SanitizeOptions::default()).err(), Some(FontDecodeError::Malformed("maxp")));
    let bad_cmap = with_table_patched(&data, b"cmap", |cmap| cmap[0] = 9);
    assert_eq!(sanitize(&bad_cmap, &SanitizeOptions::default()).err(), Some(FontDecodeError::Malformed("cmap")));
}

#[test]
fn an_optional_table_that_is_broken_is_dropped_and_the_font_lives() {
    let data = font(FONTS[0]);
    let broken = with_table_patched(&data, b"GSUB", |gsub| gsub[0] = 7);
    let clean = sanitize(&broken, &SanitizeOptions::default()).unwrap();
    assert!(table(&clean, b"GSUB").is_none());
    assert!(table(&clean, b"GPOS").is_some());
}

#[test]
fn a_broken_fvar_takes_the_variation_tables_with_it() {
    let data = font(FONTS[1]);
    let broken = with_table_patched(&data, b"fvar", |fvar| fvar[10] = 0xFF);
    let clean = sanitize(&broken, &SanitizeOptions::default()).unwrap();
    for tag in [b"fvar", b"gvar"] {
        assert!(table(&clean, tag).is_none(), "{tag:?}");
    }
}

#[test]
fn directories_that_lie_are_refused() {
    let data = font(FONTS[0]);
    let mut beyond = data.clone();
    beyond[12 + 12..12 + 16].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(sanitize(&beyond, &SanitizeOptions::default()).is_err());
    let mut overlap = data.clone();
    let second_offset = overlap.u32_at(12 + 16 + 8).unwrap();
    overlap[12 + 8..12 + 12].copy_from_slice(&second_offset.to_be_bytes());
    assert!(sanitize(&overlap, &SanitizeOptions::default()).is_err());
    let mut duplicate = data.clone();
    let tag: [u8; 4] = duplicate[12..16].try_into().unwrap();
    duplicate[12 + 16..12 + 20].copy_from_slice(&tag);
    assert!(sanitize(&duplicate, &SanitizeOptions::default()).is_err());
    assert_eq!(sanitize(b"NOPE and more", &SanitizeOptions::default()).err(), Some(FontDecodeError::UnknownFormat));
}

#[test]
fn limits_are_enforced() {
    let data = font(FONTS[0]);
    let few_tables = WebFontLimits { max_tables: 2, ..WebFontLimits::DEFAULT };
    let options = SanitizeOptions { limits: few_tables, ..Default::default() };
    assert_eq!(sanitize(&data, &options).err(), Some(FontDecodeError::LimitExceeded("tables")));
    let small = WebFontLimits { max_decoded_size: 1024, ..WebFontLimits::DEFAULT };
    let options = SanitizeOptions { limits: small, ..Default::default() };
    assert_eq!(sanitize(&data, &options).err(), Some(FontDecodeError::LimitExceeded("decoded size")));
}

/// A minimal TrueType font whose glyphs are given as raw bytes.
fn font_with_glyphs(glyphs: &[Vec<u8>]) -> Vec<u8> {
    let count = glyphs.len() as u16;
    let mut head = vec![0u8; 54];
    head[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
    head[12..16].copy_from_slice(&0x5F0F_3CF5u32.to_be_bytes());
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    head[50..52].copy_from_slice(&1i16.to_be_bytes());
    let mut maxp = vec![0u8; 6];
    maxp[0..4].copy_from_slice(&0x0000_5000u32.to_be_bytes());
    maxp[4..6].copy_from_slice(&count.to_be_bytes());
    let mut hhea = vec![0u8; 36];
    hhea[0..2].copy_from_slice(&1u16.to_be_bytes());
    hhea[34..36].copy_from_slice(&1u16.to_be_bytes());
    let hmtx = vec![0u8; 4 + (count as usize - 1) * 2];
    let mut cmap = vec![0, 0, 0, 1, 0, 3, 0, 10, 0, 0, 0, 12];
    cmap.extend_from_slice(&[0, 6, 0, 12, 0, 0, 0, 65, 0, 1, 0, 0]);
    let (mut glyf, mut loca) = (Vec::new(), Vec::new());
    for glyph in glyphs {
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        glyf.extend_from_slice(glyph);
    }
    loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
    let mut tables: Vec<(Tag, &[u8])> = vec![
        (*b"cmap", &cmap),
        (*b"glyf", &glyf),
        (*b"head", &head),
        (*b"hhea", &hhea),
        (*b"hmtx", &hmtx),
        (*b"loca", &loca),
        (*b"maxp", &maxp),
    ];
    assemble(TRUETYPE, &mut tables, &WebFontLimits::DEFAULT).unwrap()
}

fn composite(children: &[u16]) -> Vec<u8> {
    let mut glyph = vec![0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0];
    for (index, child) in children.iter().enumerate() {
        let more = if index + 1 < children.len() { 0x0020u16 } else { 0 };
        glyph.extend_from_slice(&more.to_be_bytes());
        glyph.extend_from_slice(&child.to_be_bytes());
        glyph.extend_from_slice(&[0, 0]);
    }
    glyph
}

#[test]
fn composite_cycles_and_depth_are_refused() {
    let options = SanitizeOptions::default();
    let ok = font_with_glyphs(&[Vec::new(), composite(&[0]), composite(&[1, 0])]);
    assert!(sanitize(&ok, &options).is_ok());
    let itself = font_with_glyphs(&[Vec::new(), composite(&[1])]);
    assert!(matches!(sanitize(&itself, &options), Err(FontDecodeError::Malformed(_))));
    let cycle = font_with_glyphs(&[composite(&[1]), composite(&[2]), composite(&[0])]);
    assert!(matches!(sanitize(&cycle, &options), Err(FontDecodeError::Malformed(_))));
    let beyond = font_with_glyphs(&[Vec::new(), composite(&[7])]);
    assert_eq!(sanitize(&beyond, &options).err(), Some(FontDecodeError::Malformed("glyf")));
    let mut deep: Vec<Vec<u8>> = vec![Vec::new()];
    for index in 0..40u16 {
        deep.push(composite(&[index]));
    }
    // Each glyph refers to the one before it, so the walk meets the chain from its short end.
    let deep = font_with_glyphs(&deep);
    assert_eq!(sanitize(&deep, &options).err(), Some(FontDecodeError::LimitExceeded("composite glyph depth")));
    // The same chain the other way round, met from its long end.
    let mut reversed: Vec<Vec<u8>> = (1..=40u16).map(|index| composite(&[index])).collect();
    reversed.push(Vec::new());
    let reversed = font_with_glyphs(&reversed);
    assert_eq!(sanitize(&reversed, &options).err(), Some(FontDecodeError::LimitExceeded("composite glyph depth")));
    // Exactly at the limit is accepted.
    let limit = WebFontLimits::DEFAULT.max_component_depth as u16;
    let mut at_limit: Vec<Vec<u8>> = vec![Vec::new()];
    at_limit.extend((0..limit).map(|index| composite(&[index])));
    assert!(sanitize(&font_with_glyphs(&at_limit), &options).is_ok());
    let mut over_limit: Vec<Vec<u8>> = vec![Vec::new()];
    over_limit.extend((0..=limit).map(|index| composite(&[index])));
    assert!(sanitize(&font_with_glyphs(&over_limit), &options).is_err());
}

#[test]
fn glyphs_that_lie_are_refused() {
    let options = SanitizeOptions::default();
    // Two points claimed, and no room for their flags.
    let short = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0];
    assert!(sanitize(&font_with_glyphs(&[Vec::new(), short]), &options).is_err());
    // Contour ends that go backwards.
    let backwards = vec![0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 1, 0, 0, 0x31, 0x31];
    assert!(sanitize(&font_with_glyphs(&[Vec::new(), backwards]), &options).is_err());
    // A repeat count past the number of points.
    let repeat = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0x39, 9];
    assert!(sanitize(&font_with_glyphs(&[Vec::new(), repeat]), &options).is_err());
    // One point, one flag, both coordinates the same as before.
    let good = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x31];
    assert!(sanitize(&font_with_glyphs(&[Vec::new(), good]), &options).is_ok());
}

#[test]
fn truncation_and_bit_flips_never_panic() {
    let data = font("Headless/FerroUI.Headless/BareMinimum.ttf");
    let options = SanitizeOptions::default();
    for length in (0..data.len()).step_by(data.len() / 512 + 1) {
        let _ = sanitize(&data[..length], &options);
    }
    // A small deterministic generator, so that a failure can be replayed.
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state as usize
    };
    for _ in 0..2000 {
        let mut copy = data.clone();
        for _ in 0..8 {
            let position = next() % copy.len();
            copy[position] = next() as u8;
        }
        let strip = SanitizeOptions { strip_hinting: next() % 2 == 0, ..options };
        if let Ok(font) = sanitize(&copy, &strip) {
            verify_checksums(&font);
        }
    }
}

/// Sanitizes every font named in the file at `FERROUI_FONT_LIST`, one path a line.
#[test]
#[ignore = "needs a list of fonts"]
fn sanitizes_a_list_of_fonts() {
    let list = std::env::var("FERROUI_FONT_LIST").expect("FERROUI_FONT_LIST");
    let (mut passed, mut refused) = (0, Vec::new());
    for path in std::fs::read_to_string(list).unwrap().lines() {
        let data = std::fs::read(path).unwrap();
        match sanitize(&data, &SanitizeOptions::default()) {
            Ok(clean) => {
                verify_checksums(&clean);
                passed += 1;
            }
            Err(error) => refused.push(format!("{path}: {error}")),
        }
    }
    println!("passed {passed}, refused {}:\n{}", refused.len(), refused.join("\n"));
}
