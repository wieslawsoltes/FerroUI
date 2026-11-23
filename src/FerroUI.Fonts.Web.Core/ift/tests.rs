use super::patch::GlyphPatchJob;
use super::sparse::decode_sparse_bit_set;
use super::template::{expand_template, validate_template, EntryId};
use super::testing::*;
use super::*;
use crate::sfnt::{assemble, sanitize, ReadBe, SanitizeOptions, TRUETYPE};
use crate::woff2::testing::{font_table, font_tables, outlines};
use crate::woff2::{BrotliStatus, IBrotliDecoder};
use read_fonts::{FontRef, TableProvider};
use std::path::PathBuf;

struct PlainDecoder;

impl IBrotliDecoder for PlainDecoder {
    fn decompress(&mut self, input: &[u8], in_at: &mut usize, output: &mut [u8], out_at: &mut usize) -> BrotliStatus {
        let count = (input.len() - *in_at).min(output.len() - *out_at);
        output[*out_at..*out_at + count].copy_from_slice(&input[*in_at..*in_at + count]);
        *in_at += count;
        *out_at += count;
        if *in_at == input.len() {
            BrotliStatus::Finished
        } else {
            BrotliStatus::NeedsMoreOutput
        }
    }
}

fn plain() -> Box<dyn IBrotliDecoder> {
    Box::new(PlainDecoder)
}

fn font(path: &str) -> Vec<u8> {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
    std::fs::read(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
}

const INTER: &str = "FerroUI.Base/test_assets/fonts/Inter-Regular.ttf";
const VARIABLE: &str = "FerroUI.Base/test_assets/fonts/InterVariable.ttf";

fn same(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

fn apply(fixture: &[u8], patches: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, FontDecodeError> {
    let maps = read_patch_maps(fixture, &WebFontLimits::DEFAULT)?;
    let mut job = GlyphPatchJob::new(fixture.to_vec(), maps, patches, plain, WebFontLimits::DEFAULT)?;
    loop {
        if let Some(font) = job.step(usize::MAX)? {
            return Ok(font);
        }
    }
}

fn code_points(text: &str) -> Vec<u32> {
    text.chars().map(u32::from).collect()
}

fn glyph_of(font: &[u8], character: char) -> usize {
    let reader = FontRef::new(font).unwrap();
    reader.cmap().unwrap().map_codepoint(character).unwrap().to_u32() as usize
}

#[test]
fn a_sparse_set_round_trips_and_merges() {
    let values = [3, 4, 5, 64, 65, 1000];
    let bytes = encode_sparse_bit_set(&values);
    let mut nodes = 1000;
    let (ranges, used) = decode_sparse_bit_set(&bytes, 0, &mut nodes).unwrap();
    assert_eq!(ranges, [(3, 5), (64, 65), (1000, 1000)]);
    assert_eq!(used, bytes.len());
    let (ranges, _) = decode_sparse_bit_set(&bytes, 10, &mut nodes).unwrap();
    assert_eq!(ranges[0], (13, 15), "the bias is added");
}

#[test]
fn a_sparse_node_of_zeros_is_its_whole_interval() {
    // Branch factor 4, height 2: a root of four zero bits is every value below 16.
    let (ranges, used) = decode_sparse_bit_set(&[0b01 | 2 << 2, 0], 0, &mut 10).unwrap();
    assert_eq!((ranges, used), (vec![(0, 15)], 2));
}

#[test]
fn a_sparse_set_refuses_bad_input_and_drops_what_is_past_unicode() {
    assert!(decode_sparse_bit_set(&[], 0, &mut 10).is_err());
    assert!(decode_sparse_bit_set(&[0x80], 0, &mut 10).is_err(), "the reserved bit");
    assert!(decode_sparse_bit_set(&[0b11 | 8 << 2], 0, &mut 10).is_err(), "too high");
    assert!(decode_sparse_bit_set(&[0b01 | 2 << 2], 0, &mut 10).is_err(), "no bits");
    assert_eq!(decode_sparse_bit_set(&[0b01], 0, &mut 10).unwrap(), (Vec::new(), 1), "height zero is empty");
    assert!(decode_sparse_bit_set(&[0b01 | 2 << 2, 0xFF, 0xFF, 0xFF], 0, &mut 2).is_err(), "over the node budget");
    let (ranges, _) = decode_sparse_bit_set(&[0b01 | 2 << 2, 0], 0x10_FFFA, &mut 10).unwrap();
    assert_eq!(ranges, [(0x10_FFFA, 0x10_FFFF)]);
}

#[test]
fn a_template_expands_as_in_the_standard() {
    let mut template = vec![16];
    template.extend_from_slice(b"https://foo.bar/");
    template.push(128);
    assert_eq!(expand_template(&template, EntryId::Number(123)).unwrap(), "https://foo.bar/FC");
    let digits = [129, 1, b'/', 130, 1, b'/', 131, 1, b'/', 132];
    assert_eq!(expand_template(&digits, EntryId::Number(123)).unwrap(), "C/F/_/_");
    assert_eq!(expand_template(&[128], EntryId::Number(0)).unwrap(), "00");
    assert_eq!(expand_template(&[133], EntryId::Bytes(b"a")).unwrap(), "YQ%3D%3D");
    assert_eq!(expand_template(&[133], EntryId::Bytes(b"abc")).unwrap(), "YWJj");
    assert_eq!(expand_template(&[128], EntryId::Bytes(b"")).unwrap(), "");
}

#[test]
fn a_bad_template_is_refused() {
    for bad in [&[0][..], &[134], &[255], &[3, b'a'], &[1, 0xFF]] {
        assert!(validate_template(bad).is_err(), "{bad:?}");
    }
    assert!(validate_template(&[1, b'a', 128, 133]).is_ok());
}

fn urls(map: &[u8], points: &[u32]) -> Vec<String> {
    let maps = [parse_patch_map(IFT_TABLE, map).unwrap()];
    let target = SubsetTarget::new(points.iter().copied(), DEFAULT_FEATURES.iter().copied(), Vec::new());
    patches_to_load(&maps, &target, |_| false).unwrap()
}

#[test]
fn entries_are_chosen_by_their_code_points() {
    let map = patch_map(&FIXTURE_ID, &[2, b'p', b'/', 128], &[&[97, 98], &[99]], 3, None);
    assert_eq!(urls(&map, &[97]), ["p/04"]);
    assert_eq!(urls(&map, &[99]), ["p/08"]);
    assert_eq!(urls(&map, &[98, 99]), ["p/04", "p/08"]);
    assert!(urls(&map, &[200]).is_empty());
}

/// A map whose entries are written by hand: `body` follows the header, with no id strings.
fn map_with(entry_count: u8, body: &[u8], strings: &[u8]) -> Vec<u8> {
    let template = [128];
    let entries_at = 35 + template.len();
    let mut table = vec![2, 0, 0, 0, 0];
    table.extend_from_slice(&FIXTURE_ID);
    table.push(3);
    table.extend_from_slice(&[0, 0, entry_count]);
    table.extend_from_slice(&(entries_at as u32).to_be_bytes());
    let strings_at = if strings.is_empty() { 0 } else { entries_at + body.len() };
    table.extend_from_slice(&(strings_at as u32).to_be_bytes());
    table.extend_from_slice(&(template.len() as u16).to_be_bytes());
    table.extend_from_slice(&template);
    table.extend_from_slice(body);
    table.extend_from_slice(strings);
    table
}

#[test]
fn numeric_ids_follow_their_deltas() {
    // No id: 1. Delta 10 (5): 1 + 1 + 5 = 7. Delta -4 (-2): 7 + 1 - 2 = 6.
    let body = [0, 1 << 2, 0, 0, 10, 1 << 2, 0xFF, 0xFF, 0xFC];
    let map = parse_patch_map(IFT_TABLE, &map_with(3, &body, &[])).unwrap();
    let target = SubsetTarget::new([], [], Vec::new());
    let found = patches_to_load(&[map], &target, |_| false).unwrap();
    let expect: Vec<String> =
        [1, 7, 6].iter().map(|id| expand_template(&[128], EntryId::Number(*id)).unwrap()).collect();
    assert_eq!(found, expect);
}

#[test]
fn one_entry_can_name_several_patches_and_string_ids() {
    // Two lengths in one entry (the first has the continuation bit), then an entry that repeats.
    let body = [1 << 2, 0x80, 0, 1, 0, 0, 2, 0];
    let map = parse_patch_map(IFT_TABLE, &map_with(2, &body, b"abc")).unwrap();
    let target = SubsetTarget::new([], [], Vec::new());
    let found = patches_to_load(&[map], &target, |_| false).unwrap();
    let id = |bytes: &[u8]| expand_template(&[128], EntryId::Bytes(bytes)).unwrap();
    assert_eq!(found, [id(b"a"), id(b"bc")]);
}

#[test]
fn an_id_outside_32_bits_is_refused() {
    let body = [1 << 2, 0xFF, 0xFF, 0xF0];
    assert!(parse_patch_map(IFT_TABLE, &map_with(1, &body, &[])).is_err());
}

#[test]
fn child_entries_combine_as_their_mode_says() {
    let a = encode_sparse_bit_set(&[97]);
    let b = encode_sparse_bit_set(&[98]);
    let mut body = vec![1 << 4];
    body.extend_from_slice(&a);
    body.push(1 << 4);
    body.extend_from_slice(&b);
    // Conjunctive and disjunctive parents of entries 0 and 1, with no code points of their own.
    body.extend_from_slice(&[1 << 1, 0x82, 0, 0, 0, 0, 0, 1]);
    body.extend_from_slice(&[1 << 1, 0x02, 0, 0, 0, 0, 0, 1]);
    let table = map_with(4, &body, &[]);
    let id = |number: u32| expand_template(&[128], EntryId::Number(number)).unwrap();
    assert_eq!(urls(&table, &[97]), [id(1), id(4)]);
    assert_eq!(urls(&table, &[97, 98]), [id(1), id(2), id(3), id(4)]);
}

#[test]
fn a_child_must_come_before_its_parent() {
    let body = [1 << 1, 0x01, 0, 0, 0];
    assert!(parse_patch_map(IFT_TABLE, &map_with(1, &body, &[])).is_err());
}

#[test]
fn features_and_design_space_narrow_an_entry() {
    let mut body = vec![1 << 0, 1];
    body.extend_from_slice(b"smcp");
    body.extend_from_slice(&[0, 1]);
    body.extend_from_slice(b"wght");
    body.extend_from_slice(&(400i32 << 16).to_be_bytes());
    body.extend_from_slice(&(700i32 << 16).to_be_bytes());
    let map = |target: &SubsetTarget| {
        let maps = [parse_patch_map(IFT_TABLE, &map_with(1, &body, &[])).unwrap()];
        patches_to_load(&maps, target, |_| false).unwrap().len()
    };
    let axis = |first: i32, last: i32| vec![(*b"wght", first << 16, last << 16)];
    assert_eq!(map(&SubsetTarget::new([], [*b"smcp"], axis(500, 600))), 1);
    assert_eq!(map(&SubsetTarget::new([], [*b"liga"], axis(500, 600))), 0, "not that feature");
    assert_eq!(map(&SubsetTarget::new([], [*b"smcp"], axis(800, 900))), 0, "outside the range");
    assert_eq!(map(&SubsetTarget::new([], [*b"smcp"], Vec::new())), 0, "no design space");
}

#[test]
fn an_ignored_entry_is_not_loaded_and_still_counts_for_its_id() {
    let table = map_with(2, &[1 << 6, 0], &[]);
    let id = expand_template(&[128], EntryId::Number(2)).unwrap();
    assert_eq!(urls(&table, &[97]), [id]);
}

#[test]
fn a_font_with_a_table_keyed_entry_is_refused() {
    let inter = font(INTER);
    let map = patch_map(&FIXTURE_ID, &[128], &[&[97]], 2, None);
    let mut tables = font_tables(&inter);
    tables.push((IFT_TABLE, map));
    let mut references: Vec<(Tag, &[u8])> = tables.iter().map(|(tag, data)| (*tag, data.as_slice())).collect();
    let file = assemble(TRUETYPE, &mut references, &WebFontLimits::DEFAULT).unwrap();
    assert!(matches!(read_patch_maps(&file, &WebFontLimits::DEFAULT), Err(FontDecodeError::Unsupported(_))));
}

#[test]
fn both_maps_must_have_different_ids() {
    let inter = font(INTER);
    let map = patch_map(&FIXTURE_ID, &[128], &[&[97]], 3, None);
    let mut tables = font_tables(&inter);
    tables.push((IFT_TABLE, map.clone()));
    tables.push((IFTX_TABLE, map));
    let mut references: Vec<(Tag, &[u8])> = tables.iter().map(|(tag, data)| (*tag, data.as_slice())).collect();
    let file = assemble(TRUETYPE, &mut references, &WebFontLimits::DEFAULT).unwrap();
    assert!(read_patch_maps(&file, &WebFontLimits::DEFAULT).is_err());
}

#[test]
fn a_font_without_a_map_is_not_incremental() {
    assert!(read_patch_maps(&font(INTER), &WebFontLimits::DEFAULT).unwrap().is_empty());
}

#[test]
fn a_patch_brings_back_its_glyphs_and_leaves_the_others() {
    let inter = font(INTER);
    let fixture = incremental_font(&inter, &[&code_points("abc"), &code_points("xyz")], &same);
    let before = outlines(&fixture.font);
    let original = outlines(&inter);
    let (a, x) = (glyph_of(&inter, 'a'), glyph_of(&inter, 'x'));
    assert_ne!(before[a], original[a], "the glyph was taken out");
    let patched = apply(&fixture.font, vec![fixture.patches[0].clone()]).unwrap();
    let after = outlines(&patched);
    assert_eq!(after[a], original[a]);
    assert_eq!(after[x], before[x], "the glyph of the other patch is still out");
    let maps = read_patch_maps(&patched, &WebFontLimits::DEFAULT).unwrap();
    let target = SubsetTarget::new(code_points("ax"), DEFAULT_FEATURES.iter().copied(), Vec::new());
    assert_eq!(patches_to_load(&maps, &target, |_| false).unwrap(), [fixture.patches[1].0.clone()], "applied");
    let done = apply(&patched, vec![fixture.patches[1].clone()]).unwrap();
    assert_eq!(outlines(&done), original, "every glyph is back");
    let maps = read_patch_maps(&done, &WebFontLimits::DEFAULT).unwrap();
    assert!(patches_to_load(&maps, &target, |_| false).unwrap().is_empty());
}

#[test]
fn patches_apply_as_a_batch() {
    let inter = font(INTER);
    let fixture = incremental_font(&inter, &[&code_points("abc"), &code_points("xyz")], &same);
    let patched = apply(&fixture.font, fixture.patches.clone()).unwrap();
    assert_eq!(outlines(&patched), outlines(&inter));
}

#[test]
fn the_result_passes_the_sanitizer_with_its_map() {
    let inter = font(INTER);
    let fixture = incremental_font(&inter, &[&code_points("abc")], &same);
    let options = SanitizeOptions { keep_patch_maps: true, ..Default::default() };
    let clean = sanitize(&fixture.font, &options).unwrap();
    assert!(font_table(&clean, &IFT_TABLE).is_some(), "the map is kept when asked for");
    let patched = apply(&clean, fixture.patches.clone()).unwrap();
    let clean = sanitize(&patched, &options).unwrap();
    assert_eq!(outlines(&clean)[glyph_of(&inter, 'a')], outlines(&inter)[glyph_of(&inter, 'a')]);
    let dropped = sanitize(&fixture.font, &SanitizeOptions::default()).unwrap();
    assert!(font_table(&dropped, &IFT_TABLE).is_none(), "and dropped by default");
}

#[test]
fn working_in_small_steps_gives_the_same_font() {
    let inter = font(INTER);
    let fixture = incremental_font(&inter, &[&code_points("abcdefgh")], &same);
    let whole = apply(&fixture.font, fixture.patches.clone()).unwrap();
    let maps = read_patch_maps(&fixture.font, &WebFontLimits::DEFAULT).unwrap();
    let mut job =
        GlyphPatchJob::new(fixture.font.clone(), maps, fixture.patches.clone(), plain, WebFontLimits::DEFAULT).unwrap();
    let mut steps = 0;
    let stepped = loop {
        steps += 1;
        if let Some(font) = job.step(512).unwrap() {
            break font;
        }
    };
    assert!(steps > 10, "{steps}");
    assert_eq!(stepped, whole);
}

#[test]
fn variation_data_comes_back_too() {
    let variable = font(VARIABLE);
    let fixture = incremental_font(&variable, &[&code_points("ab")], &same);
    let patched = apply(&fixture.font, fixture.patches.clone()).unwrap();
    let per_glyph = |font: &[u8]| {
        let gvar = font_table(font, b"gvar").unwrap();
        let (long, data) = (gvar.u16_at(14).unwrap() & 1 != 0, gvar.u32_at(16).unwrap() as usize);
        let at = |glyph: usize| {
            data + if long {
                gvar.u32_at(20 + glyph * 4).unwrap() as usize
            } else {
                gvar.u16_at(20 + glyph * 2).unwrap() as usize * 2
            }
        };
        let glyph = glyph_of(font, 'a');
        gvar[at(glyph)..at(glyph + 1)].to_vec()
    };
    let (got, expected) = (per_glyph(&patched), per_glyph(&variable));
    assert!(!expected.is_empty());
    assert_eq!(got[..expected.len()], expected[..], "padding to two bytes may follow");
    assert!(per_glyph(&fixture.font).is_empty(), "it was taken out");
    let options = SanitizeOptions { keep_patch_maps: true, ..Default::default() };
    assert!(font_table(&sanitize(&patched, &options).unwrap(), b"gvar").is_some(), "the table still validates");
}

/// A font with a small `CFF ` or `CFF2` table whose INDEX holds three objects.
fn with_index(tag: Tag, count_size: usize) -> (Vec<u8>, usize) {
    let inter = font(INTER);
    let mut table = vec![1, 0, 4, 1];
    let start = table.len();
    table.extend_from_slice(&3u32.to_be_bytes()[4 - count_size..]);
    table.push(1);
    table.extend_from_slice(&[1, 3, 4, 7]);
    table.extend_from_slice(b"aabccc");
    let flag = if count_size == 2 { 1 } else { 2 };
    let mut tables = font_tables(&inter);
    tables.push((tag, table));
    tables.push((IFT_TABLE, patch_map(&FIXTURE_ID, &[128], &[&[97]], 3, Some((flag, start as u32)))));
    let mut references: Vec<(Tag, &[u8])> = tables.iter().map(|(tag, data)| (*tag, data.as_slice())).collect();
    (assemble(TRUETYPE, &mut references, &WebFontLimits::DEFAULT).unwrap(), start)
}

#[test]
fn a_charstrings_index_is_written_again_with_a_wider_offset_when_it_must() {
    for (tag, count_size) in [(*b"CFF ", 2), (*b"CFF2", 4)] {
        let (file, start) = with_index(tag, count_size);
        let big = vec![b'z'; 300];
        let patch = glyph_patch(&FIXTURE_ID, &[(tag, vec![(1, big.clone())])], &same);
        let url = expand_template(&[128], EntryId::Number(1)).unwrap();
        let patched = apply(&file, vec![(url, patch)]).unwrap();
        let table = font_table(&patched, &tag).unwrap();
        assert_eq!(table[..start], [1, 0, 4, 1], "what comes before the INDEX stays");
        assert_eq!(table[start + count_size], 2, "two-byte offsets");
        let offsets = start + count_size + 1;
        let offset = |index: usize| table.u16_at(offsets + index * 2).unwrap() as usize;
        assert_eq!([offset(0), offset(1), offset(2), offset(3)], [1, 3, 303, 306]);
        let data = offsets + 8 - 1;
        assert_eq!(table[data + 1..data + 3], *b"aa");
        assert_eq!(table[data + 3..data + 303], big[..]);
        assert_eq!(table[data + 303..data + 306], *b"ccc");
    }
}

#[test]
fn a_patch_for_another_font_or_with_bad_structure_is_refused() {
    let inter = font(INTER);
    let fixture = incremental_font(&inter, &[&code_points("a")], &same);
    let (url, good) = fixture.patches[0].clone();
    let refused = |patch: Vec<u8>| assert!(apply(&fixture.font, vec![(url.clone(), patch)]).is_err());
    let mut other = good.clone();
    other[9] ^= 1;
    refused(other);
    let mut wrong = good.clone();
    wrong[0] = b'x';
    refused(wrong);
    refused(good[..20].to_vec());
    let mut short = good.clone();
    short[25..29].copy_from_slice(&1u32.to_be_bytes());
    refused(short);
    let id = FIXTURE_ID;
    let glyph = |id_value: u16| (id_value, vec![0u8; 2]);
    refused(glyph_patch(&id, &[(*b"glyf", vec![glyph(0xFFFF)])], &same));
    refused(glyph_patch(&id, &[(*b"glyf", vec![glyph(5), glyph(5)])], &same));
    refused(glyph_patch(&id, &[(*b"glyf", vec![glyph(5), glyph(4)])], &same));
    refused(glyph_patch(&id, &[(*b"gvar", vec![glyph(1)])], &same));
    let mut declared = glyph_patch(&id, &[(*b"glyf", vec![glyph(1)])], &same);
    let body = 29;
    // The first data offset, moved past the end of the patch.
    declared[body + 5 + 2 + 4..body + 5 + 2 + 8].copy_from_slice(&u32::MAX.to_be_bytes());
    refused(declared);
}

#[test]
fn tables_a_patch_may_not_touch_are_ignored() {
    let inter = font(INTER);
    let fixture = incremental_font(&inter, &[&code_points("a")], &same);
    let patch = glyph_patch(&FIXTURE_ID, &[(*b"cmap", vec![(1, vec![1, 2, 3])])], &same);
    let patched = apply(&fixture.font, vec![(fixture.patches[0].0.clone(), patch)]).unwrap();
    assert_eq!(font_table(&patched, b"cmap"), font_table(&fixture.font, b"cmap"));
}

#[test]
fn a_short_loca_that_would_overflow_is_refused() {
    let inter = font(INTER);
    let mut tables = font_tables(&inter);
    let count = tables.iter().find(|(tag, _)| tag == b"maxp").unwrap().1.u16_at(4).unwrap() as usize;
    for (tag, data) in &mut tables {
        match &*tag {
            b"head" => data[50..52].fill(0),
            b"glyf" => data.clear(),
            b"loca" => *data = vec![0; (count + 1) * 2],
            _ => {}
        }
    }
    tables.push((IFT_TABLE, patch_map(&FIXTURE_ID, &[128], &[&[97]], 3, None)));
    let mut references: Vec<(Tag, &[u8])> = tables.iter().map(|(tag, data)| (*tag, data.as_slice())).collect();
    let file = assemble(TRUETYPE, &mut references, &WebFontLimits::DEFAULT).unwrap();
    let patch = glyph_patch(&FIXTURE_ID, &[(*b"glyf", vec![(1, vec![0; 0x2_0000])])], &same);
    let url = expand_template(&[128], EntryId::Number(1)).unwrap();
    assert!(apply(&file, vec![(url, patch)]).is_err());
}

/// A small random generator, so that a failure can be repeated.
fn random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn corrupt_maps_and_patches_never_panic() {
    let inter = font(INTER);
    let fixture = incremental_font(&inter, &[&code_points("abc"), &code_points("xyz")], &same);
    let map = font_table(&fixture.font, &IFT_TABLE).unwrap();
    let (url, patch) = fixture.patches[0].clone();
    let mut state = 0x1234_5678_9ABC_DEF1u64;
    let target = SubsetTarget::new(code_points("abcxyz"), DEFAULT_FEATURES.iter().copied(), Vec::new());
    for _ in 0..3000 {
        let mut bytes = map.clone();
        for _ in 0..1 + random(&mut state) % 4 {
            let at = random(&mut state) as usize % bytes.len();
            bytes[at] = random(&mut state) as u8;
        }
        if random(&mut state) % 4 == 0 {
            bytes.truncate(random(&mut state) as usize % bytes.len());
        }
        if let Ok(parsed) = parse_patch_map(IFT_TABLE, &bytes) {
            let _ = patches_to_load(&[parsed], &target, |_| false);
        }
    }
    for _ in 0..300 {
        let mut bytes = patch.clone();
        for _ in 0..1 + random(&mut state) % 6 {
            // Mostly the structure at the start of the patch, past the compatibility id.
            let span = if random(&mut state) % 4 == 0 { bytes.len() } else { bytes.len().min(160) };
            let at = 25 + random(&mut state) as usize % (span - 25);
            bytes[at] = random(&mut state) as u8;
        }
        let _ = apply(&fixture.font, vec![(url.clone(), bytes)]);
    }
}
