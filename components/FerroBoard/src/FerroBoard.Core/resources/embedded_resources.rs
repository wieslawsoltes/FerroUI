// Derived from Resources/EmbeddedResources.cs; see ../NOTICE.md.
use super::resource_data::RESOURCES;
use crate::text::Utf16Text;
use flate2::read::MultiGzDecoder;
use std::{
    collections::HashMap,
    io::{self, Cursor, Read},
    sync::LazyLock,
};
pub(super) struct Resource {
    pub path: &'static str,
    pub logical_name: &'static str,
    pub bytes: &'static [u8],
}
static INDEX: LazyLock<HashMap<&'static str, &'static Resource>> =
    LazyLock::new(|| RESOURCES.iter().map(|r| (r.path, r)).collect());
pub struct EmbeddedResources;
impl EmbeddedResources {
    pub fn paths() -> impl Iterator<Item = &'static str> {
        RESOURCES.iter().map(|r| r.path)
    }
    fn resolve(path: &str) -> Option<&'static Resource> {
        let path = path.replace('\\', "/");
        let path = path.trim_start_matches('/');
        INDEX
            .get(path)
            .or_else(|| INDEX.get(format!("{path}.gz").as_str()))
            .copied()
    }
    pub fn exists(path: &str) -> bool {
        Self::resolve(path).is_some()
    }
    /// Rust stream bridge: each call owns a fresh cursor/decompressor over static shared bytes.
    pub fn open(path: &str) -> Option<Box<dyn Read + Send>> {
        let resource = Self::resolve(path)?;
        let cursor = Cursor::new(resource.bytes);
        if resource.path.ends_with(".gz") {
            Some(Box::new(MultiGzDecoder::new(cursor)))
        } else {
            Some(Box::new(cursor))
        }
    }
    pub fn read_text(path: &str) -> io::Result<Option<Utf16Text>> {
        let Some(mut stream) = Self::open(path) else {
            return Ok(None);
        };
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes)?;
        Ok(Some(decode_text(&bytes)))
    }
    /// Manifest identity for diagnostics and resource integrity checks.
    pub fn logical_name(path: &str) -> Option<&'static str> {
        Self::resolve(path).map(|r| r.logical_name)
    }
}
// StreamReader defaults to UTF-8 with BOM detection and replacement fallback.
fn decode_text(bytes: &[u8]) -> Utf16Text {
    let (width, big, skip) = if bytes.starts_with(&[0xff, 0xfe, 0, 0]) {
        (4, false, 4)
    } else if bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        (4, true, 4)
    } else if bytes.starts_with(&[0xff, 0xfe]) {
        (2, false, 2)
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (2, true, 2)
    } else {
        (
            1,
            false,
            if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
                3
            } else {
                0
            },
        )
    };
    let bytes = &bytes[skip..];
    if width == 1 {
        return String::from_utf8_lossy(bytes).as_ref().into();
    }
    let mut result = String::new();
    if width == 2 {
        let (chunks, remainder) = bytes.as_chunks::<2>();
        let units: Vec<_> = chunks
            .iter()
            .map(|b| {
                if big {
                    u16::from_be_bytes([b[0], b[1]])
                } else {
                    u16::from_le_bytes([b[0], b[1]])
                }
            })
            .collect();
        result = String::from_utf16_lossy(&units);
        if !remainder.is_empty() {
            result.push('\u{fffd}');
        }
    } else {
        let (chunks, remainder) = bytes.as_chunks::<4>();
        for b in chunks {
            let array = [b[0], b[1], b[2], b[3]];
            let cp = if big {
                u32::from_be_bytes(array)
            } else {
                u32::from_le_bytes(array)
            };
            result.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
        }
        if !remainder.is_empty() {
            result.push('\u{fffd}');
        }
    }
    result.as_str().into()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stream_reader_reference_decoding() {
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../../tests/fixtures/stream-text.json")).unwrap();
        for row in rows {
            let bytes: Vec<u8> = row["bytes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect();
            let expected: Vec<u16> = row["units"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect();
            assert_eq!(decode_text(&bytes).units(), expected, "{bytes:?}");
        }
    }
    #[test]
    fn bom_and_replacement_decoding() {
        for bytes in [
            vec![0xef, 0xbb, 0xbf, 65],
            vec![0xff, 0xfe, 65, 0],
            vec![0xfe, 0xff, 0, 65],
            vec![0xff, 0xfe, 0, 0, 65, 0, 0, 0],
            vec![0, 0, 0xfe, 0xff, 0, 0, 0, 65],
        ] {
            assert_eq!(decode_text(&bytes), "A".into());
        }
        assert_eq!(decode_text(&[0xff]), "�".into());
        assert_eq!(decode_text(&[0xff, 0xfe, 0, 0xd8]), "�".into());
    }
}
