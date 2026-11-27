use super::testing::*;
use super::*;
use crate::sfnt::sanitize;
use crate::{decode_font_with_brotli, sniff_container, FontContainer, FontDecoder};
use std::path::PathBuf;

/// A decoder of the "compression" of these tests: the stream as it is.
#[derive(Default)]
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
const SMALL: &str = "Headless/FerroUI.Headless/BareMinimum.ttf";
const VARIABLE: &str = "FerroUI.Base/test_assets/fonts/InterVariable.ttf";
const ROBOTO: &str = "FerroUI.Themes.Material/Assets/Roboto-Regular.ttf";

const ALL: EncodeOptions = EncodeOptions { transform_glyf: true, transform_hmtx: true };

fn encoded(data: &[u8], options: EncodeOptions) -> Vec<u8> {
    encode_woff2(data, options, &|stream| stream.to_vec())
}

fn decode(file: &[u8]) -> Result<Vec<u8>, FontDecodeError> {
    decode_font_with_brotli(file, &SanitizeOptions::default(), Some(plain))
}

fn assert_same_font(original: &[u8], decoded: &[u8]) {
    let clean = sanitize(original, &SanitizeOptions::default()).unwrap();
    assert_eq!(outlines(&clean), outlines(decoded), "the outlines");
    for (tag, mut table) in font_tables(&clean) {
        if tag == *b"glyf" || tag == *b"loca" {
            continue;
        }
        let mut other = font_table(decoded, &tag).unwrap();
        if tag == *b"head" {
            // The whole-font checksum, which depends on how the glyphs were packed.
            table[8..12].fill(0);
            other[8..12].fill(0);
        }
        assert!(other == table, "the table {}", String::from_utf8_lossy(&tag));
    }
    assert_eq!(font_tables(&clean).len(), font_tables(decoded).len());
}

#[test]
fn fonts_round_trip_with_every_transform() {
    for path in [SMALL, INTER, ROBOTO, VARIABLE] {
        let data = font(path);
        for (transform_glyf, transform_hmtx) in [(true, true), (true, false), (false, false)] {
            let file = encoded(&data, EncodeOptions { transform_glyf, transform_hmtx });
            assert_eq!(sniff_container(&file), Some(FontContainer::Woff2));
            let decoded = decode(&file).unwrap_or_else(|error| panic!("{path}: {error}"));
            assert_same_font(&data, &decoded);
        }
    }
}

#[test]
fn a_font_without_a_decoder_is_refused() {
    let file = encoded(&font(SMALL), ALL);
    assert_eq!(
        crate::decode_font(&file, &SanitizeOptions::default()).err(),
        Some(FontDecodeError::Unsupported("WOFF2 needs a Brotli decoder"))
    );
}

#[test]
fn stepping_gives_the_same_font() {
    let data = font(INTER);
    let file = encoded(&data, ALL);
    let whole = decode(&file).unwrap();
    let mut decoder = FontDecoder::with_brotli(&file, SanitizeOptions::default(), Some(plain)).unwrap();
    let mut steps = 0;
    let stepped = loop {
        steps += 1;
        if let Some(font) = decoder.step(4096).unwrap() {
            break font;
        }
    };
    assert!(steps > 16, "the work is spread over steps");
    assert!(whole == stepped);
}

#[test]
fn numbers_of_the_directory() {
    let read = |bytes: &[u8]| Reader::new(bytes, 0).base128();
    assert_eq!(read(&[0x3F]), Ok(63));
    assert_eq!(read(&[0x81, 0x00]), Ok(128));
    assert_eq!(read(&[0x8F, 0xFF, 0xFF, 0xFF, 0x7F]), Ok(u32::MAX));
    assert!(read(&[0x80, 0x01]).is_err(), "a leading zero");
    assert!(read(&[0x90, 0x80, 0x80, 0x80, 0x00]).is_err(), "more than 32 bits");
    assert!(read(&[0x81, 0x80, 0x80, 0x80, 0x80, 0x00]).is_err(), "more than five bytes");
    assert!(read(&[0x81]).is_err(), "cut short");
    let u255 = |bytes: &[u8]| Reader::new(bytes, 0).u255();
    for bytes in [&[255u8, 253][..], &[254, 0], &[253, 1, 250]] {
        assert_eq!(u255(bytes), Ok(506), "every encoding of 506 is accepted");
    }
    assert_eq!(u255(&[252]), Ok(252));
    for value in [0u16, 252, 253, 505, 506, 761, 762, 65535] {
        let mut bytes = Vec::new();
        write_u255(&mut bytes, value);
        assert_eq!(u255(&bytes), Ok(value));
    }
}

#[test]
fn the_triplet_table_of_the_standard() {
    // Rows of the table in section 5.2: index, data, then dx and dy.
    let rows: [(u8, &[u8], i32, i32); 14] = [
        (0, &[7], 0, -7),
        (1, &[7], 0, 7),
        (9, &[0], 0, 1024),
        (10, &[7], -7, 0),
        (19, &[0], 1024, 0),
        (20, &[0x00], -1, -1),
        (21, &[0x00], 1, -1),
        (23, &[0x23], 3, 4),
        (24, &[0x00], -1, -17),
        (36, &[0x00], -17, -1),
        (84, &[0, 0], -1, -1),
        (95, &[0, 0], 1, 513),
        (96, &[0, 0], -257, -1),
        (127, &[0x12, 0x34, 0x56, 0x78], 0x1234, 0x5678),
    ];
    for (flag, data, dx, dy) in rows {
        assert_eq!(glyf::decode_triplet_for_test(flag, data), (dx, dy), "row {flag}");
    }
    assert_eq!(glyf::decode_triplet_for_test(120, &[0xAB, 0xCD, 0xEF]), (-0xABC, -0xDEF));
}

/// A WOFF2 file with its header or stream patched.
fn patched(file: &[u8], patch: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut copy = file.to_vec();
    patch(&mut copy);
    copy
}

/// Where the compressed data of a file without a collection directory starts.
fn directory_end(file: &[u8]) -> usize {
    let mut reader = Reader::new(file, HEADER_SIZE);
    read_directory(&mut reader, file.u16_at(12).unwrap() as usize, &WebFontLimits::DEFAULT).unwrap();
    reader.position
}

#[test]
fn every_rejection_rule_of_the_header() {
    let good = encoded(&font(SMALL), ALL);
    assert!(decode(&good).is_ok());
    assert_eq!(decode(&patched(&good, |f| f[8] ^= 1)).err(), Some(malformed("WOFF2 length")));
    assert_eq!(decode(&patched(&good, |f| f[12..14].fill(0))).err(), Some(malformed("WOFF2 without tables")));
    assert!(decode(&patched(&good, |f| f[14] = 1)).is_ok(), "the reserved field is not a reason to refuse");
    assert!(decode(&patched(&good, |f| f[16..20].fill(0))).is_ok(), "totalSfntSize is only a hint");
    assert_eq!(
        decode(&patched(&good, |f| f[28..32].copy_from_slice(&64u32.to_be_bytes()))).err(),
        Some(malformed("WOFF2 block with an offset or a length alone"))
    );
    let extra = patched(&good, |f| {
        f.extend_from_slice(&[0; 8]);
        let length = f.len() as u32;
        f[8..12].copy_from_slice(&length.to_be_bytes());
    });
    assert_eq!(decode(&extra).err(), Some(malformed("WOFF2 data after the last block")));
    // The padding of the compressed data is required, and must be zeros.
    let unpadded_font = encode_woff2(&font(SMALL), ALL, &|stream| {
        let mut changed = stream.to_vec();
        changed.resize(stream.len() + (4 - (stream.len() + 48) % 4) % 4 + 1, 0);
        changed
    });
    let padding = unpadded_font.len() - (unpadded_font.u32_at(20).unwrap() as usize + directory_end(&unpadded_font));
    if padding > 0 {
        let cut = patched(&unpadded_font, |f| {
            let length = f.len() - padding;
            f.truncate(length);
            f[8..12].copy_from_slice(&(length as u32).to_be_bytes());
        });
        assert_eq!(decode(&cut).err(), Some(malformed("WOFF2 compressed data without its padding")));
        let dirty = patched(&unpadded_font, |f| *f.last_mut().unwrap() = 1);
        assert_eq!(decode(&dirty).err(), Some(malformed("WOFF2 data after the last block")));
    }
    let longer = patched(&good, |f| {
        let compressed = f.len() as u32;
        f[20..24].copy_from_slice(&compressed.to_be_bytes());
    });
    assert_eq!(decode(&longer).err(), Some(FontDecodeError::Truncated));
}

/// Rebuilds a file from a font with its transformed `glyf` changed.
fn with_glyf_stream(data: &[u8], change: impl Fn(&mut Vec<u8>)) -> Vec<u8> {
    let original = encoded(data, EncodeOptions { transform_glyf: true, transform_hmtx: false });
    let glyf = transform_glyf(data);
    let mut changed = glyf.clone();
    change(&mut changed);
    assert_eq!(glyf.len(), changed.len(), "the change keeps the length");
    let at = original.windows(glyf.len()).position(|window| window == glyf).unwrap();
    let mut file = original;
    file[at..at + changed.len()].copy_from_slice(&changed);
    file
}

#[test]
fn transformed_glyphs_that_lie_are_refused() {
    let data = font(SMALL);
    const BAD: FontDecodeError = FontDecodeError::Malformed("WOFF2 glyf transform");
    assert_eq!(decode(&with_glyf_stream(&data, |g| g[1] = 1)).err(), Some(BAD), "reserved field");
    assert_eq!(decode(&with_glyf_stream(&data, |g| g[5] ^= 1)).err(), Some(BAD), "numGlyphs differs from maxp");
    assert_eq!(decode(&with_glyf_stream(&data, |g| g[7] ^= 1)).err(), Some(BAD), "indexFormat differs from head");
    assert_eq!(
        decode(&with_glyf_stream(&data, |g| g[8..12].copy_from_slice(&u32::MAX.to_be_bytes()))).err(),
        Some(BAD),
        "a substream beyond the table"
    );
}

#[test]
fn an_empty_glyph_with_a_bounding_box_is_refused() {
    let data = font(SMALL);
    let glyphs = outlines(&data);
    let empty = glyphs.iter().position(|glyph| *glyph == Outline::Empty).expect("an empty glyph");
    let changed = with_glyf_stream(&data, |g| {
        let sizes: Vec<usize> = (0..7).map(|index| g.u32_at(8 + index * 4).unwrap() as usize).collect();
        let bitmap = 36 + sizes[..5].iter().sum::<usize>();
        g[bitmap + (empty >> 3)] |= 0x80 >> (empty & 7);
    });
    assert_eq!(decode(&changed).err(), Some(FontDecodeError::Malformed("WOFF2 glyf transform")));
}

#[test]
fn a_stream_of_the_wrong_length_is_refused() {
    let data = font(SMALL);
    let short = encode_woff2(&data, ALL, &|stream| stream[..stream.len() - 1].to_vec());
    assert!(decode(&short).is_err());
    let long = encode_woff2(&data, ALL, &|stream| {
        let mut longer = stream.to_vec();
        longer.push(0);
        longer
    });
    assert!(decode(&long).is_err());
}

#[test]
fn limits_are_enforced_before_allocating() {
    let file = encoded(&font(SMALL), ALL);
    let limits = WebFontLimits { max_decoded_size: 256, ..WebFontLimits::DEFAULT };
    let options = SanitizeOptions { limits, ..Default::default() };
    assert_eq!(
        decode_font_with_brotli(&file, &options, Some(plain)).err(),
        Some(FontDecodeError::LimitExceeded("decoded size"))
    );
}

#[test]
fn truncation_and_corruption_never_panic() {
    let file = encoded(&font(SMALL), ALL);
    for length in 0..file.len().min(4096) {
        let _ = decode(&file[..length]);
    }
    let mut state = 0xD1B5_4A32_D192_ED03u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state as usize
    };
    for _ in 0..4000 {
        let mut copy = file.clone();
        for _ in 0..4 {
            let range = if next() % 2 == 0 { copy.len().min(1024) } else { copy.len() };
            let position = next() % range;
            copy[position] = next() as u8;
        }
        let _ = decode(&copy);
    }
}
