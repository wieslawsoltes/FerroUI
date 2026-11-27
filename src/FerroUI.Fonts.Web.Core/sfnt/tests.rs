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
                let (_, records) = read_directory(&data, 0, &WebFontLimits::DEFAULT).unwrap();
                let dropped: Vec<String> = records
                    .iter()
                    .filter(|record| table(&clean, &record.tag).is_none())
                    .map(|record| String::from_utf8_lossy(&record.tag).into_owned())
                    .collect();
                if !dropped.is_empty() {
                    println!("{}: dropped {}", path.rsplit('/').next().unwrap(), dropped.join(" "));
                }
            }
            Err(error) => refused.push(format!("{path}: {error}")),
        }
    }
    println!("passed {passed}, refused {}:\n{}", refused.len(), refused.join("\n"));
}

mod contents {
    use super::super::deep::{has_color_cycle, has_versioned_fields, TableCheck, TableContext, Verdict};
    use super::*;

    const INTER: &str = "FerroUI.Base/test_assets/fonts/Inter-Regular.ttf";
    const VARIABLE: &str = "FerroUI.Base/test_assets/fonts/InterVariable.ttf";

    fn context(data: &[u8]) -> TableContext {
        let maxp = table(data, b"maxp").unwrap();
        TableContext {
            glyph_count: maxp.u16_at(4).unwrap(),
            axis_count: table(data, b"fvar").and_then(|fvar| fvar.u16_at(8)).unwrap_or(0),
            palette_entries: table(data, b"CPAL").and_then(|cpal| cpal.u16_at(2)),
        }
    }

    fn verdict(tag: &Tag, bytes: &[u8], context: &TableContext) -> Verdict {
        let mut check = TableCheck::new(*tag, bytes.len());
        let mut budget = usize::MAX;
        check.step(bytes, context, &mut budget)
    }

    #[test]
    fn the_tables_of_real_fonts_validate() {
        for path in [INTER, VARIABLE] {
            let data = font(path);
            let context = context(&data);
            for tag in [b"GSUB", b"GPOS", b"GDEF", b"cmap", b"name", b"post", b"OS/2"] {
                if let Some(bytes) = table(&data, tag) {
                    assert_eq!(verdict(tag, bytes, &context), Verdict::Valid, "{path}: {tag:?}");
                }
            }
        }
        let data = font(VARIABLE);
        let context = context(&data);
        for tag in [b"fvar", b"gvar", b"HVAR", b"STAT"] {
            if let Some(bytes) = table(&data, tag) {
                assert_eq!(verdict(tag, bytes, &context), Verdict::Valid, "{tag:?}");
            }
        }
    }

    #[test]
    fn a_walk_resumes_where_it_stopped() {
        let data = font(INTER);
        let context = context(&data);
        for tag in [b"GSUB", b"GPOS"] {
            let bytes = table(&data, tag).unwrap();
            let mut check = TableCheck::new(*tag, bytes.len());
            let mut steps = 0;
            let result = loop {
                steps += 1;
                let mut budget = 512;
                match check.step(bytes, &context, &mut budget) {
                    Verdict::Pending => {}
                    verdict => break verdict,
                }
            };
            assert_eq!(result, Verdict::Valid);
            assert!(steps > 4, "{tag:?} took {steps} steps");
        }
    }

    /// A small GSUB: one script-less feature list, one lookup of type 1, format 2, with a
    /// coverage of the given glyphs.
    fn small_gsub(coverage: &[u16], feature_lookup: u16, substitute: u16) -> Vec<u8> {
        let mut gsub = vec![0, 1, 0, 0, 0, 10, 0, 12, 0, 26];
        gsub.extend_from_slice(&[0, 0]); // ScriptList: no scripts.
        gsub.extend_from_slice(&[0, 1]); // FeatureList with one record.
        gsub.extend_from_slice(b"liga");
        gsub.extend_from_slice(&[0, 8]);
        gsub.extend_from_slice(&[0, 0, 0, 1]); // Feature: no params, one lookup.
        gsub.extend_from_slice(&feature_lookup.to_be_bytes());
        assert_eq!(gsub.len(), 26);
        gsub.extend_from_slice(&[0, 1, 0, 4]); // LookupList with one lookup.
        gsub.extend_from_slice(&[0, 1, 0, 0, 0, 1, 0, 8]); // Lookup of type 1 with one subtable.
        let count = coverage.len() as u16;
        let coverage_offset = 6 + 2 * count;
        gsub.extend_from_slice(&[0, 2]);
        gsub.extend_from_slice(&coverage_offset.to_be_bytes());
        gsub.extend_from_slice(&count.to_be_bytes());
        for _ in coverage {
            gsub.extend_from_slice(&substitute.to_be_bytes());
        }
        gsub.extend_from_slice(&[0, 1]);
        gsub.extend_from_slice(&count.to_be_bytes());
        for glyph in coverage {
            gsub.extend_from_slice(&glyph.to_be_bytes());
        }
        gsub
    }

    #[test]
    fn values_that_point_nowhere_are_refused() {
        let context = TableContext { glyph_count: 16, ..Default::default() };
        assert_eq!(verdict(b"GSUB", &small_gsub(&[1, 2], 0, 3), &context), Verdict::Valid);
        assert_eq!(verdict(b"GSUB", &small_gsub(&[1, 99], 0, 3), &context), Verdict::Invalid, "a covered glyph");
        assert_eq!(verdict(b"GSUB", &small_gsub(&[1, 2], 0, 99), &context), Verdict::Invalid, "a substitute");
        assert_eq!(verdict(b"GSUB", &small_gsub(&[1, 2], 7, 3), &context), Verdict::Invalid, "a lookup index");
        assert_eq!(verdict(b"GSUB", &small_gsub(&[2, 1], 0, 3), &context), Verdict::Invalid, "coverage order");
        assert_eq!(verdict(b"GSUB", &small_gsub(&[2, 2], 0, 3), &context), Verdict::Valid, "a repeat, as shipped");
    }

    #[test]
    fn shared_subtables_cannot_multiply_the_work() {
        // A lookup list whose thousands of lookups are all the same lookup, each with thousands
        // of subtable offsets to the same subtable.
        const LOOKUPS: u16 = 3000;
        let mut gsub = vec![0, 1, 0, 0, 0, 10, 0, 12, 0, 14, 0, 0, 0, 0];
        gsub.extend_from_slice(&LOOKUPS.to_be_bytes());
        let lookup = 2 + LOOKUPS * 2;
        for _ in 0..LOOKUPS {
            gsub.extend_from_slice(&lookup.to_be_bytes());
        }
        const SUBTABLES: u16 = 12000;
        gsub.extend_from_slice(&[0, 1, 0, 0]);
        gsub.extend_from_slice(&SUBTABLES.to_be_bytes());
        let subtable = 6 + SUBTABLES * 2;
        for _ in 0..SUBTABLES {
            gsub.extend_from_slice(&subtable.to_be_bytes());
        }
        gsub.extend_from_slice(&[0, 1, 0, 6, 0, 1, 0, 1, 0, 1, 0, 1]);
        let context = TableContext { glyph_count: 16, ..Default::default() };
        assert_eq!(verdict(b"GSUB", &gsub, &context), Verdict::Invalid);
    }

    #[test]
    fn tables_too_short_for_their_version_do_not_panic() {
        let context = TableContext { glyph_count: 4, axis_count: 1, palette_entries: Some(2) };
        let cases: [(&Tag, Vec<u8>); 12] = [
            (b"BASE", vec![0, 1, 0, 1, 0, 0, 0, 0]),
            (b"COLR", vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"CPAL", vec![0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 14, 0, 0, 1, 2, 3, 4]),
            (b"GDEF", vec![0, 1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"GDEF", vec![0, 1, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"GSUB", vec![0, 1, 0, 1, 0, 0, 0, 0, 0, 0]),
            (b"GPOS", vec![0, 1, 0, 1, 0, 0, 0, 0, 0, 0]),
            (b"STAT", vec![0, 1, 0, 1, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            (b"avar", vec![0, 2, 0, 0, 0, 0, 0, 1, 0, 0]),
            (b"name", vec![0, 1, 0, 0, 0, 6]),
            (b"post", {
                let mut post = vec![0; 32];
                post[1] = 2;
                post
            }),
            (b"OS/2", vec![0, 5, 0, 0]),
        ];
        for (tag, bytes) in cases {
            assert_eq!(verdict(tag, &bytes, &context), Verdict::Invalid, "{tag:?}");
        }
        // A lookup that says it has a mark filtering set and ends before it.
        let gsub = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 10, 0, 1, 0, 4, 0, 1, 0, 0x10, 0, 0];
        assert_eq!(verdict(b"GSUB", &gsub, &context), Verdict::Invalid);
    }

    /// The fields listed next to the dependency, and the version they were audited for.
    fn guarded_fields() -> (String, Vec<String>) {
        let manifest = include_str!("../Cargo.toml");
        let section = manifest.split("[package.metadata.read-fonts-guards]").nth(1).expect("the list of guards");
        let section = section.split("\n[").next().unwrap();
        let audited = section.lines().find_map(|line| line.strip_prefix("audited = ")).expect("the audited version");
        let fields = section
            .lines()
            .filter_map(|line| line.trim().strip_prefix('"')?.strip_suffix("\",").map(str::to_owned))
            .collect();
        (audited.trim_matches('"').to_owned(), fields)
    }

    #[test]
    fn the_guards_were_audited_for_the_pinned_version() {
        let (audited, _) = guarded_fields();
        let workspace =
            std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml")).unwrap();
        let pin = workspace.lines().find(|line| line.starts_with("read-fonts = ")).expect("the pin");
        assert!(pin.contains(&format!("\"={audited}\"")), "audit the traversal for the new pin: {pin}");
    }

    /// For each guarded field, a table that promises it and is too short to hold it.
    fn too_short_for(field: &str) -> Option<Vec<u8>> {
        let os2 = |version: u8, length: usize| {
            let mut table = vec![0u8; length];
            table[1] = version;
            table
        };
        Some(match field {
            "BASE.item_var_store_offset" => vec![0, 1, 0, 1, 0, 0, 0, 0],
            "COLR.base_glyph_list_offset"
            | "COLR.clip_list_offset"
            | "COLR.item_variation_store_offset"
            | "COLR.layer_list_offset"
            | "COLR.var_index_map_offset" => vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "CPAL.palette_entry_labels_array_offset"
            | "CPAL.palette_labels_array_offset"
            | "CPAL.palette_types_array_offset" => vec![0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 0, 14, 0, 0, 1, 2, 3, 4],
            "GDEF.mark_glyph_sets_def_offset" => vec![0, 1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0],
            "GDEF.item_var_store_offset" => vec![0, 1, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "GPOS.feature_variations_offset" | "GSUB.feature_variations_offset" => vec![0, 1, 0, 1, 0, 0, 0, 0, 0, 0],
            "GPOS.mark_filtering_set" | "GSUB.mark_filtering_set" => {
                vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 10, 0, 1, 0, 4, 0, 1, 0, 0x10, 0, 0]
            }
            "OS/2.ul_code_page_range_1" | "OS/2.ul_code_page_range_2" => os2(1, 85),
            "OS/2.s_cap_height"
            | "OS/2.sx_height"
            | "OS/2.us_break_char"
            | "OS/2.us_default_char"
            | "OS/2.us_max_context" => os2(2, 95),
            "OS/2.us_lower_optical_point_size" | "OS/2.us_upper_optical_point_size" => os2(5, 99),
            "STAT.elided_fallback_name_id" => vec![0, 1, 0, 1, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "avar.axis_index_map_offset" | "avar.var_store_offset" => vec![0, 2, 0, 0, 0, 0, 0, 1, 0, 0],
            "name.lang_tag_count" | "name.lang_tag_record" => vec![0, 1, 0, 0, 0, 6],
            "post.num_glyphs" | "post.glyph_name_index" => {
                let mut table = vec![0u8; 32];
                table[1] = 2;
                table
            }
            _ => return None,
        })
    }

    #[test]
    fn every_guarded_field_has_a_guard() {
        let (_, fields) = guarded_fields();
        assert_eq!(fields.len(), 31);
        for field in fields {
            let bytes = too_short_for(&field).unwrap_or_else(|| panic!("{field} has no case proving its guard"));
            let tag: Tag = field.as_bytes()[..4].try_into().unwrap();
            assert!(!has_versioned_fields(&tag, &bytes), "{field} is not guarded");
        }
    }

    #[test]
    fn tuple_stores_are_read_to_their_end() {
        let context = TableContext { glyph_count: 1, axis_count: 1, ..Default::default() };
        // One tuple with an embedded peak, whose data is one run of two zero deltas.
        let good = vec![0, 1, 0, 0, 0, 1, 0, 14, 0, 1, 0x80, 0, 0x40, 0, 0x81];
        assert_eq!(verdict(b"cvar", &good, &context), Verdict::Valid);
        let mut no_peak = good.clone();
        no_peak[10] = 0;
        assert_eq!(verdict(b"cvar", &no_peak, &context), Verdict::Invalid, "cvar has no shared tuples");
        let mut too_long = good.clone();
        too_long[9] = 40;
        assert_eq!(verdict(b"cvar", &too_long, &context), Verdict::Invalid, "a tuple past the end");
        let mut run_past = good.clone();
        run_past[14] = 0x05;
        assert_eq!(verdict(b"cvar", &run_past, &context), Verdict::Invalid, "a run past the tuple");
    }

    #[test]
    fn kern_pairs_are_checked() {
        let context = TableContext { glyph_count: 8, ..Default::default() };
        let kern = |first: u16, second: u16| {
            let mut bytes = vec![0, 0, 0, 1, 0, 0, 0, 26, 0, 1, 0, 2, 0, 0, 0, 0, 0, 0];
            for pair in [(1u16, 2u16), (first, second)] {
                bytes.extend_from_slice(&pair.0.to_be_bytes());
                bytes.extend_from_slice(&pair.1.to_be_bytes());
                bytes.extend_from_slice(&[0, 1]);
            }
            bytes
        };
        assert_eq!(verdict(b"kern", &kern(1, 3), &context), Verdict::Valid);
        assert_eq!(verdict(b"kern", &kern(1, 1), &context), Verdict::Invalid, "out of order");
        assert_eq!(verdict(b"kern", &kern(1, 9), &context), Verdict::Invalid, "no such glyph");
    }

    #[test]
    fn color_glyph_cycles_are_found() {
        assert!(!has_color_cycle(&[(1, 2), (2, 3), (1, 3)]));
        assert!(has_color_cycle(&[(1, 2), (2, 3), (3, 1)]));
        assert!(has_color_cycle(&[(5, 5)]));
    }

    #[test]
    fn a_table_with_no_validator_is_not_kept() {
        let data = font(INTER);
        let mut renamed = data.clone();
        let count = data.u16_at(4).unwrap() as usize;
        // `MATH` sorts where this name does, so the directory stays in order.
        let entry = (0..count).map(|index| 12 + index * 16).find(|entry| &data[*entry..*entry + 4] == b"GSUB").unwrap();
        renamed[entry..entry + 4].copy_from_slice(b"MATH");
        let clean = sanitize(&renamed, &SanitizeOptions::default()).unwrap();
        assert!(table(&clean, b"MATH").is_none());
    }

    /// Seeded corruption of each table on its own, where a lie reaches the deep checks instead
    /// of being stopped by the directory.
    fn corrupt_tables(path: &str, rounds: usize) {
        let data = font(path);
        let context = context(&data);
        let (_, records) = read_directory(&data, 0, &WebFontLimits::DEFAULT).unwrap();
        let mut state = 0x7F4A_7C15_9E37_79B9u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as usize
        };
        for record in records.iter().filter(|record| record.length >= 4 && record.length <= 128 * 1024) {
            let original = &data[record.offset..record.offset + record.length];
            for _ in 0..rounds {
                let mut copy = original.to_vec();
                for _ in 0..1 + next() % 8 {
                    // Headers are where offsets and counts are.
                    let range = if next() % 2 == 0 { copy.len().min(256) } else { copy.len() };
                    let position = next() % range;
                    copy[position] = if next() % 3 == 0 { 0xFF } else { next() as u8 };
                }
                if next() % 4 == 0 {
                    copy.truncate(next() % copy.len() + 1);
                }
                let _ = verdict(&record.tag, &copy, &context);
            }
        }
    }

    #[test]
    fn corrupted_tables_never_panic() {
        corrupt_tables(INTER, 24);
        corrupt_tables(VARIABLE, 12);
        corrupt_tables("../samples/ControlCatalog/Assets/FallbackFonts/NotoColorEmoji-Catalog.ttf", 24);
    }

    /// The same, for as long as asked: `FERROUI_FUZZ_ROUNDS` rounds of each table.
    #[test]
    #[ignore = "a long run"]
    fn corrupted_tables_never_panic_long() {
        let rounds = std::env::var("FERROUI_FUZZ_ROUNDS").ok().and_then(|text| text.parse().ok()).unwrap_or(2000);
        for path in FONTS {
            corrupt_tables(path, rounds);
        }
    }
}
