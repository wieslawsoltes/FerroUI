//! Read embedded raster dimensions without decoding pixels or allocating a
//! second copy of a potentially large image. SVG needs the source aspect ratio
//! to express the same source-window and stretch rules as FerroUI ImageBrush.
use crate::Point;

struct EncodedImage<'a> { bytes: &'a [u8], len: usize }
impl EncodedImage<'_> {
    fn byte(&self, index: usize) -> Option<u8> {
        if index >= self.len { return None; }
        let block = index / 3 * 4;
        let slot = index % 3;
        let a = sextet(*self.bytes.get(block + slot)?)?;
        let b = sextet(*self.bytes.get(block + slot + 1)?)?;
        Some(match slot { 0 => (a << 2) | (b >> 4), 1 => (a << 4) | (b >> 2), _ => (a << 6) | b })
    }
    fn matches(&self, offset: usize, value: &[u8]) -> bool {
        value.iter().enumerate().all(|(i, &value)| self.byte(offset + i) == Some(value))
    }
    fn number(&self, offset: usize, count: usize, little: bool) -> Option<u32> {
        let mut value = 0;
        for i in 0..count {
            let shift = if little { i * 8 } else { (count - 1 - i) * 8 };
            value |= u32::from(self.byte(offset + i)?) << shift;
        }
        Some(value)
    }
}
fn sextet(value: u8) -> Option<u8> {
    match value {
        b'A'..=b'Z' => Some(value - b'A'), b'a'..=b'z' => Some(value - b'a' + 26),
        b'0'..=b'9' => Some(value - b'0' + 52), b'+' => Some(62), b'/' => Some(63), _ => None,
    }
}
fn dimensions(width: u32, height: u32) -> Option<Point> {
    (width > 0 && height > 0).then(|| Point::new(f64::from(width), f64::from(height)))
}

/// Inspect PNG, JPEG, GIF or WebP data URLs. This reads container metadata; the
/// renderer still validates and decodes the complete image through FerroUI.
pub fn embedded_image_size(href: &str) -> Option<Point> {
    let (header, payload) = href.split_once(',')?;
    if !header.starts_with("data:image/") || !header.ends_with(";base64") { return None; }
    let bytes = payload.trim().as_bytes();
    if bytes.len() < 4 || bytes.len() > 64 * 1024 * 1024 || bytes.len() % 4 == 1 { return None; }
    let padding = bytes.iter().rev().take_while(|&&v| v == b'=').count();
    if padding > 2 { return None; }
    let image = EncodedImage { bytes, len: (bytes.len() * 3 / 4).checked_sub(padding)? };
    if image.matches(0, b"\x89PNG\r\n\x1a\n") && image.matches(12, b"IHDR") {
        return dimensions(image.number(16, 4, false)?, image.number(20, 4, false)?);
    }
    if image.matches(0, b"GIF87a") || image.matches(0, b"GIF89a") {
        return dimensions(image.number(6, 2, true)?, image.number(8, 2, true)?);
    }
    if image.matches(0, &[0xff, 0xd8]) {
        let mut offset = 2;
        while offset < image.len {
            if image.byte(offset)? != 0xff { return None; }
            while image.byte(offset) == Some(0xff) { offset += 1; }
            let marker = image.byte(offset)?;
            offset += 1;
            if matches!(marker, 0xd9 | 0xda | 0x00) { return None; }
            if marker == 0x01 || (0xd0..=0xd7).contains(&marker) { continue; }
            let size = image.number(offset, 2, false)? as usize;
            if size < 2 || offset.checked_add(size)? > image.len { return None; }
            if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
                if size < 7 { return None; }
                return dimensions(image.number(offset + 5, 2, false)?, image.number(offset + 3, 2, false)?);
            }
            offset += size;
        }
    }
    if image.matches(0, b"RIFF") && image.matches(8, b"WEBP") {
        let mut offset: usize = 12;
        while offset.checked_add(8)? <= image.len {
            let size = image.number(offset + 4, 4, true)? as usize;
            let data = offset + 8;
            if data.checked_add(size)? > image.len { return None; }
            if image.matches(offset, b"VP8X") && size >= 10 {
                return dimensions(image.number(data + 4, 3, true)? + 1, image.number(data + 7, 3, true)? + 1);
            }
            if image.matches(offset, b"VP8L") && size >= 5 && image.byte(data)? == 0x2f {
                let bits = image.number(data + 1, 4, true)?;
                return dimensions((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1);
            }
            if image.matches(offset, b"VP8 ") && size >= 10 && image.matches(data + 3, &[0x9d, 0x01, 0x2a]) {
                return dimensions(image.number(data + 6, 2, true)? & 0x3fff, image.number(data + 8, 2, true)? & 0x3fff);
            }
            offset = data.checked_add(size)?.checked_add(size % 2)?;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn url(bytes: &[u8]) -> String {
        const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::from("data:image/test;base64,");
        for part in bytes.chunks(3) {
            let bits = (u32::from(part[0]) << 16) | (u32::from(*part.get(1).unwrap_or(&0)) << 8) | u32::from(*part.get(2).unwrap_or(&0));
            for i in 0..4 { result.push(if i > part.len() { '=' } else { TABLE[((bits >> (18 - i * 6)) & 63) as usize] as char }); }
        }
        result
    }
    #[test]
    fn reads_raster_container_dimensions_without_pixel_decoding() {
        let png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x01\x00\x00\x00\x00\x80";
        assert_eq!(embedded_image_size(&url(png)), Some(Point::new(256.0, 128.0)));
        assert_eq!(embedded_image_size(&url(b"GIF89a\x40\x01\xf0\x00")), Some(Point::new(320.0, 240.0)));
        assert_eq!(embedded_image_size(&url(&[0xff,0xd8,0xff,0xe0,0,4,0,0,0xff,0xc2,0,8,8,0,90,0,160,3])), Some(Point::new(160.0,90.0)));
        let mut webp = b"RIFF\0\0\0\0WEBPVP8X\x0a\0\0\0\0\0\0\0".to_vec();
        webp.extend([255,0,0,127,0,0]);
        assert_eq!(embedded_image_size(&url(&webp)), Some(Point::new(256.0,128.0)));
    }
    #[test]
    fn truncated_or_untrusted_headers_fail_without_allocating_image_data() {
        for value in ["", "https://example.invalid/photo.png", "data:image/png;base64,!!!!", "data:image/png;base64,R0lGODlhAA=="] {
            assert_eq!(embedded_image_size(value), None);
        }
        assert_eq!(embedded_image_size(&url(&[0xff,0xd8,0xff,0xe0,0xff,0xff])), None);
    }
}
