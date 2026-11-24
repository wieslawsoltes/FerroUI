//! Images embedded in a document as `data:` URLs. Nothing is ever fetched. The size is read from
//! the header before anything is decoded, so that a small file that expands to a huge bitmap is
//! refused by the limits of the document.

/// An image of a `data:` URL: its encoded bytes and its size in pixels.
#[derive(Debug, PartialEq)]
pub(crate) struct EmbeddedData {
    pub(crate) bytes: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

/// Why a reference is not an embedded image that can be drawn.
#[derive(Debug, PartialEq)]
pub(crate) enum EmbeddedError {
    /// The reference is not a `data:` URL.
    NotData,
    /// A media type other than PNG or JPEG.
    UnsupportedType(String),
    /// Text that is not base64 encoded.
    UnsupportedEncoding,
    /// The data is not a PNG or JPEG with a readable size.
    Invalid,
}

/// Reads a `data:image/png;base64,...` or `data:image/jpeg;base64,...` URL.
pub(crate) fn decode_data_url(text: &str) -> Result<EmbeddedData, EmbeddedError> {
    let text = text.trim();
    let scheme = text.get(..5).filter(|scheme| scheme.eq_ignore_ascii_case("data:")).ok_or(EmbeddedError::NotData)?;
    let rest = &text[scheme.len()..];
    let (meta, payload) = rest.split_once(',').ok_or(EmbeddedError::Invalid)?;
    let mut parts = meta.split(';');
    let media_type = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    if !matches!(media_type.as_str(), "image/png" | "image/jpeg" | "image/jpg") {
        return Err(EmbeddedError::UnsupportedType(media_type.chars().take(32).collect()));
    }
    if !parts.any(|part| part.trim().eq_ignore_ascii_case("base64")) {
        return Err(EmbeddedError::UnsupportedEncoding);
    }
    let bytes = decode_base64(payload).ok_or(EmbeddedError::Invalid)?;
    let (width, height) = png_size(&bytes).or_else(|| jpeg_size(&bytes)).ok_or(EmbeddedError::Invalid)?;
    if width == 0 || height == 0 {
        return Err(EmbeddedError::Invalid);
    }
    Ok(EmbeddedData { bytes, width, height })
}

fn base64_value(byte: u8) -> Option<u32> {
    match byte {
        b'A'..=b'Z' => Some(u32::from(byte - b'A')),
        b'a'..=b'z' => Some(u32::from(byte - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(byte - b'0') + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Standard base64, with whitespace allowed anywhere and the padding optional. `None` for any
/// other character, for data after the padding and for a length no encoder writes.
fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(text.len() / 4 * 3 + 3);
    let (mut buffer, mut bits, mut digits, mut padding) = (0u32, 0u32, 0usize, 0usize);
    for byte in text.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            padding += 1;
            if padding > 2 {
                return None;
            }
            continue;
        }
        if padding > 0 {
            return None;
        }
        buffer = (buffer << 6) | base64_value(byte)?;
        bits += 6;
        digits += 1;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    if digits % 4 == 1 || (padding > 0 && (digits + padding) % 4 != 0) {
        return None;
    }
    Some(bytes)
}

fn big_endian_u16(bytes: &[u8], at: usize) -> Option<u32> {
    let part = bytes.get(at..at + 2)?;
    Some(u32::from(part[0]) << 8 | u32::from(part[1]))
}

/// The size in the header chunk of a PNG.
fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if !bytes.starts_with(SIGNATURE) || bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    let number = |at: usize| Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    Some((number(16)?, number(20)?))
}

/// The size in the first start-of-frame segment of a JPEG.
fn jpeg_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut at = 2;
    loop {
        if *bytes.get(at)? != 0xff {
            return None;
        }
        // Any number of fill bytes may come before a marker.
        while *bytes.get(at)? == 0xff {
            at += 1;
        }
        let marker = *bytes.get(at)?;
        at += 1;
        match marker {
            // Markers without a segment.
            0x01 | 0xd0..=0xd8 => continue,
            // The image data or its end, with no frame before it.
            0xd9 | 0xda => return None,
            // Every start of frame; 0xc4, 0xc8 and 0xcc share the range and are not frames.
            0xc0..=0xcf if !matches!(marker, 0xc4 | 0xc8 | 0xcc) => {
                let height = big_endian_u16(bytes, at + 3)?;
                let width = big_endian_u16(bytes, at + 5)?;
                return Some((width, height));
            }
            _ => {
                let length = big_endian_u16(bytes, at)? as usize;
                if length < 2 {
                    return None;
                }
                at += length;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut text = String::new();
        for chunk in bytes.chunks(3) {
            let group = u32::from(chunk[0]) << 16
                | u32::from(*chunk.get(1).unwrap_or(&0)) << 8
                | u32::from(*chunk.get(2).unwrap_or(&0));
            for index in 0..4 {
                if index <= chunk.len() {
                    text.push(ALPHABET[(group >> (18 - 6 * index) & 63) as usize] as char);
                } else {
                    text.push('=');
                }
            }
        }
        text
    }

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes
    }

    #[test]
    fn base64_round_trips_with_whitespace_and_without_padding() {
        for text in [&b"a"[..], b"ab", b"abc", b"abcd", b"\x00\xff\x10"] {
            let encoded = encode(text);
            assert_eq!(decode_base64(&encoded).as_deref(), Some(text), "{encoded}");
            assert_eq!(decode_base64(encoded.trim_end_matches('=')).as_deref(), Some(text), "{encoded}");
            let spaced: String = encoded.chars().flat_map(|c| [c, '\n']).collect();
            assert_eq!(decode_base64(&spaced).as_deref(), Some(text), "{encoded}");
        }
    }

    #[test]
    fn base64_refuses_what_no_encoder_writes() {
        for text in ["a", "ab!d", "ab=d", "a===", "abc=="] {
            assert_eq!(decode_base64(text), None, "{text}");
        }
    }

    #[test]
    fn the_size_of_a_png_is_read_from_its_header() {
        let url = format!("data:image/png;base64,{}", encode(&png_header(640, 480)));
        let data = decode_data_url(&url).unwrap();
        assert_eq!((data.width, data.height), (640, 480));
    }

    #[test]
    fn the_size_of_a_jpeg_is_read_past_its_other_segments() {
        let mut bytes = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x04, 0x00, 0x00];
        // A table segment in the range of the frames, which is not a frame.
        bytes.extend_from_slice(&[0xff, 0xc4, 0x00, 0x03, 0x00]);
        bytes.extend_from_slice(&[0xff, 0xff, 0xc2, 0x00, 0x0b, 0x08, 0x01, 0x02, 0x03, 0x04]);
        let url = format!("data:image/jpeg;base64,{}", encode(&bytes));
        let data = decode_data_url(&url).unwrap();
        assert_eq!((data.width, data.height), (0x0304, 0x0102));
    }

    #[test]
    fn other_references_and_forms_are_refused() {
        assert_eq!(decode_data_url("https://example.invalid/a.png"), Err(EmbeddedError::NotData));
        assert_eq!(decode_data_url("a.png"), Err(EmbeddedError::NotData));
        assert_eq!(
            decode_data_url("data:image/svg+xml;base64,AAAA"),
            Err(EmbeddedError::UnsupportedType("image/svg+xml".to_owned()))
        );
        assert_eq!(decode_data_url("data:image/png,abc"), Err(EmbeddedError::UnsupportedEncoding));
        assert_eq!(decode_data_url("data:image/png;base64,AAAA"), Err(EmbeddedError::Invalid));
        let empty = format!("data:image/png;base64,{}", encode(&png_header(0, 10)));
        assert_eq!(decode_data_url(&empty), Err(EmbeddedError::Invalid));
    }

    #[test]
    fn a_truncated_header_is_refused_without_a_panic() {
        let header = png_header(10, 10);
        for length in 0..header.len() {
            assert_eq!(png_size(&header[..length]), None, "{length}");
        }
        let jpeg = [0xff, 0xd8, 0xff, 0xc0, 0x00, 0x0b, 0x08, 0x01, 0x02, 0x03, 0x04];
        for length in 0..jpeg.len() {
            assert_eq!(jpeg_size(&jpeg[..length]), None, "{length}");
        }
    }
}
