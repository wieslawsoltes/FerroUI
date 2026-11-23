use super::*;
use crate::sfnt::{sanitize, SanitizeOptions};
use crate::{decode_font, sniff_container, FontContainer, FontDecoder};
use flate2::{Compress, Compression, FlushCompress};
use std::path::PathBuf;

fn font(path: &str) -> Vec<u8> {
    let full = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(path);
    std::fs::read(&full).unwrap_or_else(|error| panic!("{}: {error}", full.display()))
}

fn inter() -> Vec<u8> {
    font("FerroUI.Base/test_assets/fonts/Inter-Regular.ttf")
}

fn small() -> Vec<u8> {
    font("Headless/FerroUI.Headless/BareMinimum.ttf")
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut compress = Compress::new(Compression::default(), true);
    let mut output = Vec::with_capacity(data.len() + 1024);
    loop {
        let consumed = compress.total_in() as usize;
        let status = compress.compress_vec(&data[consumed..], &mut output, FlushCompress::Finish).unwrap();
        if matches!(status, Status::StreamEnd) {
            return output;
        }
        output.reserve(output.capacity().max(1024));
    }
}

fn sfnt_tables(data: &[u8]) -> Vec<(Tag, Vec<u8>)> {
    let count = data.u16_at(4).unwrap() as usize;
    let mut tables: Vec<(Tag, Vec<u8>)> = (0..count)
        .map(|index| {
            let record = 12 + index * 16;
            let tag: Tag = data[record..record + 4].try_into().unwrap();
            let (offset, length) =
                (data.u32_at(record + 8).unwrap() as usize, data.u32_at(record + 12).unwrap() as usize);
            (tag, data[offset..offset + length].to_vec())
        })
        .collect();
    tables.sort_by_key(|(tag, _)| *tag);
    tables
}

/// Writes a WOFF file of a font's tables, with optional metadata and private blocks.
fn encode(data: &[u8], metadata: Option<&[u8]>, private: Option<&[u8]>) -> Vec<u8> {
    let tables = sfnt_tables(data);
    let count = tables.len();
    let mut body = Vec::new();
    let mut directory = Vec::new();
    let start = HEADER_SIZE + count * ENTRY_SIZE;
    let mut total = 12 + 16 * count;
    for (tag, table) in &tables {
        let packed = deflate(table);
        let stored: &[u8] = if packed.len() < table.len() { &packed } else { table };
        directory.extend_from_slice(tag);
        directory.extend_from_slice(&((start + body.len()) as u32).to_be_bytes());
        directory.extend_from_slice(&(stored.len() as u32).to_be_bytes());
        directory.extend_from_slice(&(table.len() as u32).to_be_bytes());
        directory.extend_from_slice(&0u32.to_be_bytes());
        body.extend_from_slice(stored);
        body.resize((body.len() + 3) & !3, 0);
        total += (table.len() + 3) & !3;
    }
    let mut file = vec![0u8; HEADER_SIZE];
    file.extend_from_slice(&directory);
    file.extend_from_slice(&body);
    let mut header = [0u32; 3];
    if let Some(metadata) = metadata {
        let packed = deflate(metadata);
        header = [file.len() as u32, packed.len() as u32, metadata.len() as u32];
        file.extend_from_slice(&packed);
    }
    let mut private_fields = [0u32; 2];
    if let Some(private) = private {
        file.resize((file.len() + 3) & !3, 0);
        private_fields = [file.len() as u32, private.len() as u32];
        file.extend_from_slice(private);
    }
    let length = file.len() as u32;
    file[0..4].copy_from_slice(b"wOFF");
    file[4..8].copy_from_slice(&data[0..4]);
    file[8..12].copy_from_slice(&length.to_be_bytes());
    file[12..14].copy_from_slice(&(count as u16).to_be_bytes());
    file[16..20].copy_from_slice(&(total as u32).to_be_bytes());
    file[24..28].copy_from_slice(&header[0].to_be_bytes());
    file[28..32].copy_from_slice(&header[1].to_be_bytes());
    file[32..36].copy_from_slice(&header[2].to_be_bytes());
    file[36..40].copy_from_slice(&private_fields[0].to_be_bytes());
    file[40..44].copy_from_slice(&private_fields[1].to_be_bytes());
    file
}

fn options() -> SanitizeOptions {
    SanitizeOptions::default()
}

fn decode_error(file: &[u8]) -> Option<FontDecodeError> {
    decode_font(file, &options()).err()
}

#[test]
fn a_font_round_trips_through_woff() {
    for data in [inter(), small(), font("FerroUI.Base/test_assets/fonts/InterVariable.ttf")] {
        let file = encode(&data, None, None);
        assert!(file.len() < data.len());
        assert_eq!(sniff_container(&file), Some(FontContainer::Woff));
        let decoded = decode_font(&file, &options()).unwrap();
        assert!(decoded == sanitize(&data, &options()).unwrap(), "the same font as from the sfnt");
    }
}

#[test]
fn metadata_and_private_blocks_are_accepted_and_ignored() {
    let data = small();
    let expected = sanitize(&data, &options()).unwrap();
    let file = encode(&data, Some(b"<metadata version=\"1.0\"></metadata>"), Some(b"private"));
    assert!(decode_font(&file, &options()).unwrap() == expected);
    let file = encode(&data, None, Some(b"private data"));
    assert!(decode_font(&file, &options()).unwrap() == expected);
}

#[test]
fn invalid_metadata_content_does_not_invalidate_the_font() {
    let data = small();
    let mut file = encode(&data, Some(b"<metadata/>"), None);
    let offset = file.u32_at(24).unwrap() as usize;
    file[offset] ^= 0xFF;
    assert!(decode_font(&file, &options()).is_ok());
}

#[test]
fn stepping_gives_the_same_font() {
    let data = inter();
    let file = encode(&data, None, None);
    let whole = decode_font(&file, &options()).unwrap();
    let mut decoder = FontDecoder::new(&file, options()).unwrap();
    let mut steps = 0;
    let stepped = loop {
        steps += 1;
        if let Some(font) = decoder.step(8192).unwrap() {
            break font;
        }
    };
    assert!(steps > 8, "the work is spread over steps");
    assert!(whole == stepped);
    assert!(decoder.step(1).is_err(), "a finished decoder has nothing more");
}

#[test]
fn every_rejection_rule() {
    let good = encode(&small(), None, None);
    let count = good.u16_at(12).unwrap() as usize;
    let patched = |patch: &dyn Fn(&mut Vec<u8>)| {
        let mut file = good.clone();
        patch(&mut file);
        file
    };
    let entry = |index: usize| HEADER_SIZE + index * ENTRY_SIZE;

    assert_eq!(
        decode_error(&patched(&|f| f[8..12].copy_from_slice(&1u32.to_be_bytes()))),
        Some(malformed("WOFF length"))
    );
    assert_eq!(decode_error(&patched(&|f| f[15] = 1)), Some(malformed("WOFF reserved field")));
    assert_eq!(decode_error(&patched(&|f| f[12..14].fill(0))), Some(malformed("WOFF without tables")));
    let total = good.u32_at(16).unwrap();
    assert_eq!(
        decode_error(&patched(&|f| f[16..20].copy_from_slice(&(total + 1).to_be_bytes()))),
        Some(malformed("WOFF totalSfntSize"))
    );
    assert_eq!(
        decode_error(&patched(&|f| f[16..20].copy_from_slice(&(total + 4).to_be_bytes()))),
        Some(malformed("WOFF totalSfntSize"))
    );
    // Directory order.
    let swapped = patched(&|f| {
        let (a, b) = (entry(0), entry(1));
        let first: Vec<u8> = f[a..a + 4].to_vec();
        let second: Vec<u8> = f[b..b + 4].to_vec();
        f[a..a + 4].copy_from_slice(&second);
        f[b..b + 4].copy_from_slice(&first);
    });
    assert_eq!(decode_error(&swapped), Some(malformed("WOFF directory order")));
    // A compressed length above the original.
    let compressed = entry(0) + 8;
    let original = good.u32_at(entry(0) + 12).unwrap();
    assert_eq!(
        decode_error(&patched(&|f| f[compressed..compressed + 4].copy_from_slice(&(original + 1).to_be_bytes()))),
        Some(malformed("WOFF table larger than its original"))
    );
    // A misaligned table.
    let offset = good.u32_at(entry(count - 1) + 4).unwrap();
    assert_eq!(
        decode_error(&patched(
            &|f| f[entry(count - 1) + 4..entry(count - 1) + 8].copy_from_slice(&(offset + 1).to_be_bytes())
        )),
        Some(malformed("WOFF table alignment"))
    );
    // A table placed on top of another.
    let first_offset = good.u32_at(entry(0) + 4).unwrap();
    assert_eq!(
        decode_error(&patched(&|f| f[entry(1) + 4..entry(1) + 8].copy_from_slice(&first_offset.to_be_bytes()))),
        Some(malformed("WOFF table layout"))
    );
    // Extra data after the last block.
    let extra = patched(&|f| {
        f.extend_from_slice(&[0; 8]);
        let length = f.len() as u32;
        f[8..12].copy_from_slice(&length.to_be_bytes());
    });
    assert_eq!(decode_error(&extra), Some(malformed("WOFF data after the last block")));
    // A table that runs past the end of the file.
    let length = good.len() as u32;
    assert_eq!(
        decode_error(&patched(
            &|f| f[entry(count - 1) + 8..entry(count - 1) + 12].copy_from_slice(&length.to_be_bytes())
        )),
        Some(FontDecodeError::Truncated)
    );
    // Padding that is not zero.
    let padded = (0..count)
        .map(|index| (good.u32_at(entry(index) + 4).unwrap() as usize, good.u32_at(entry(index) + 8).unwrap() as usize))
        .find(|(offset, length)| length % 4 != 0 && offset + length < good.len())
        .expect("a table with padding after it");
    assert_eq!(decode_error(&patched(&|f| f[padded.0 + padded.1] = 1)), Some(malformed("WOFF table layout")));
    // A block with an offset and no length.
    assert_eq!(
        decode_error(&patched(&|f| f[24..28].copy_from_slice(&64u32.to_be_bytes()))),
        Some(malformed("WOFF block with an offset or a length alone"))
    );
}

#[test]
fn a_table_that_inflates_to_another_size_is_refused() {
    let data = small();
    let tables = sfnt_tables(&data);
    let index = tables.iter().position(|(_, table)| deflate(table).len() < table.len()).unwrap();
    for delta in [-1i64, 1] {
        let mut file = encode(&data, None, None);
        let entry = HEADER_SIZE + index * ENTRY_SIZE;
        let original = file.u32_at(entry + 12).unwrap() as i64 + delta;
        file[entry + 12..entry + 16].copy_from_slice(&(original as u32).to_be_bytes());
        let mut total = 12 + 16 * tables.len();
        for position in 0..tables.len() {
            total += (file.u32_at(HEADER_SIZE + position * ENTRY_SIZE + 12).unwrap() as usize + 3) & !3;
        }
        file[16..20].copy_from_slice(&(total as u32).to_be_bytes());
        assert_eq!(decode_error(&file), Some(malformed("WOFF compressed table")), "{delta}");
    }
}

#[test]
fn limits_are_enforced_before_allocating() {
    let file = encode(&small(), None, None);
    let limits = WebFontLimits { max_decoded_size: 64, ..WebFontLimits::DEFAULT };
    let tight = SanitizeOptions { limits, ..options() };
    assert_eq!(decode_font(&file, &tight).err(), Some(FontDecodeError::LimitExceeded("decoded size")));
    let limits = WebFontLimits { max_encoded_size: 64, ..WebFontLimits::DEFAULT };
    let tight = SanitizeOptions { limits, ..options() };
    assert_eq!(decode_font(&file, &tight).err(), Some(FontDecodeError::LimitExceeded("encoded size")));
    let mut huge = file.clone();
    huge[16..20].copy_from_slice(&0xFFFF_FFFCu32.to_be_bytes());
    assert_eq!(decode_error(&huge), Some(FontDecodeError::LimitExceeded("decoded size")));
}

#[test]
fn truncation_and_corruption_never_panic() {
    let file = encode(&small(), Some(b"<metadata/>"), Some(b"private"));
    for length in 0..file.len().min(4096) {
        let _ = decode_font(&file[..length], &options());
    }
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state as usize
    };
    for _ in 0..3000 {
        let mut copy = file.clone();
        for _ in 0..4 {
            // The header and directory are where a lie does the most damage.
            let range = if next() % 2 == 0 { copy.len().min(512) } else { copy.len() };
            let position = next() % range;
            copy[position] = next() as u8;
        }
        let _ = decode_font(&copy, &options());
    }
}
