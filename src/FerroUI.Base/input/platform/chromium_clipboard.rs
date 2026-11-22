//! The native format map used by Chromium for custom ClipboardItem data.
//! Platform backends read the map and slots from their native clipboard.

use super::web_clipboard_format::{from_web_clipboard_format, to_web_clipboard_format, WEB_PREFIX};
use crate::input::{DataFormat, DataFormatKind, DataFormatOf};
use std::any::{Any, TypeId};
use std::collections::BTreeMap;
use std::rc::Rc;

const SLOT_COUNT: u8 = 100;

/// Maximum metadata size before decoding the custom format map.
pub const MAX_MAP_BYTES: usize = 64 * 1024;

/// Whether this build publishes and reads the map (the `chromium-clipboard-map` feature).
pub(super) const MAP_ENABLED: bool = cfg!(feature = "chromium-clipboard-map");

#[cfg(feature = "chromium-clipboard-map")]
fn encode_map(map: &BTreeMap<String, String>) -> Option<Vec<u8>> {
    serde_json::to_vec(map).ok()
}

#[cfg(not(feature = "chromium-clipboard-map"))]
fn encode_map(_: &BTreeMap<String, String>) -> Option<Vec<u8>> {
    None
}

/// The entries of a map whose value is a string; malformed JSON has none.
#[cfg(feature = "chromium-clipboard-map")]
fn decode_map(data: &[u8]) -> Vec<(String, String)> {
    let Ok(map) = serde_json::from_slice::<BTreeMap<String, serde_json::Value>>(data) else {
        return Vec::new();
    };
    map.into_iter()
        .filter_map(|(mime, slot)| match slot {
            serde_json::Value::String(slot) => Some((mime, slot)),
            _ => None,
        })
        .collect()
}

#[cfg(not(feature = "chromium-clipboard-map"))]
fn decode_map(_: &[u8]) -> Vec<(String, String)> {
    Vec::new()
}

/// Native names used by Chromium for its map and fixed set of data slots.
#[derive(Clone, Copy)]
pub enum ChromiumClipboard {
    Windows,
    Linux,
    MacOs,
}

/// A transfer's JSON metadata and corresponding payload slots.
pub struct ClipboardMap {
    pub bytes: Rc<[u8]>,
    pub aliases: Vec<FormatAlias>,
}

/// A typed application format transported via a platform byte format.
#[derive(Clone)]
pub struct FormatAlias {
    pub application: DataFormat,
    pub slot: DataFormatOf<Rc<[u8]>>,
}

impl ChromiumClipboard {
    pub fn map_format(self) -> DataFormatOf<Rc<[u8]>> {
        DataFormat::create_bytes_platform_format(self.map_name())
    }

    fn map_name(self) -> &'static str {
        match self {
            Self::Windows => "Web Custom Format Map",
            Self::Linux => r#"application/web;type="custom/formatmap""#,
            Self::MacOs => "org.w3.web-custom-format.map",
        }
    }

    fn slot_name_parts(self) -> (&'static str, &'static str) {
        match self {
            Self::Windows => ("Web Custom Format", ""),
            Self::Linux => (r#"application/web;type="custom/format"#, "\""),
            Self::MacOs => ("org.w3.web-custom-format.type-", ""),
        }
    }

    fn find_platform_format<'a>(
        self,
        formats: &'a [DataFormat],
        name: &str,
    ) -> Option<&'a DataFormat> {
        formats.iter().find(|format| {
            format.kind() == DataFormatKind::Platform
                && match self {
                    Self::Windows => format.identifier().eq_ignore_ascii_case(name),
                    Self::Linux | Self::MacOs => format.identifier() == name,
                }
        })
    }

    /// Finds the map in the formats actually advertised by the source.
    pub fn find_map(self, formats: &[DataFormat]) -> Option<DataFormatOf<Rc<[u8]>>> {
        if !MAP_ENABLED {
            return None;
        }
        let map = self.find_platform_format(formats, self.map_name())?;
        Some(DataFormat::create_bytes_platform_format(map.identifier()))
    }

    /// Assigns unused native slots to the transfer's supported application formats.
    pub fn create_map(self, formats: &[DataFormat]) -> Option<ClipboardMap> {
        if !MAP_ENABLED || self.find_map(formats).is_some() {
            return None;
        }
        let (prefix, suffix) = self.slot_name_parts();
        let mut slots = (0..SLOT_COUNT)
            .map(|index| format!("{prefix}{index}{suffix}"))
            .filter(|name| self.find_platform_format(formats, name).is_none());
        let mut map = BTreeMap::new();
        let mut aliases = Vec::new();
        for application in formats
            .iter()
            .filter(|format| format.kind() == DataFormatKind::Application)
        {
            let content_type = application.data_type();
            let text = content_type == Some(TypeId::of::<String>());
            if !text && content_type != Some(TypeId::of::<Rc<[u8]>>()) {
                continue;
            }
            let Some(slot) = slots.next() else { break };
            let web_name = to_web_clipboard_format(application.identifier(), text);
            let mime = &web_name[WEB_PREFIX.len()..];
            aliases.push(FormatAlias {
                application: application.clone(),
                slot: DataFormat::create_bytes_platform_format(&slot),
            });
            map.insert(mime.to_owned(), slot);
        }
        if aliases.is_empty() {
            return None;
        }
        let bytes = encode_map(&map)?;
        (bytes.len() <= MAX_MAP_BYTES).then(|| ClipboardMap {
            bytes: bytes.into(),
            aliases,
        })
    }

    /// Decodes links to existing slots and preserves formats already present
    /// under their native application names.
    pub fn parse_map(self, data: &[u8], formats: &[DataFormat]) -> Vec<FormatAlias> {
        if !MAP_ENABLED || data.len() > MAX_MAP_BYTES {
            return Vec::new();
        }
        let end = data
            .iter()
            .rposition(|byte| *byte != 0)
            .map_or(0, |last| last + 1);
        let (prefix, suffix) = self.slot_name_parts();
        let mut aliases = Vec::<FormatAlias>::new();
        for (mime, name) in decode_map(&data[..end]) {
            let Some(index) = name
                .strip_prefix(prefix)
                .and_then(|name| name.strip_suffix(suffix))
            else {
                continue;
            };
            let canonical_index = (index.len() == 1 || index.len() == 2 && !index.starts_with('0'))
                && index.bytes().all(|byte| byte.is_ascii_digit());
            if !canonical_index {
                continue;
            }
            let Some(slot) = self.find_platform_format(formats, &name) else {
                continue;
            };
            let Some(application) = from_web_clipboard_format(&format!("{WEB_PREFIX}{mime}"))
            else {
                continue;
            };
            if formats.contains(&application)
                || aliases
                    .iter()
                    .any(|alias| alias.application == application || *alias.slot == *slot)
            {
                continue;
            }
            aliases.push(FormatAlias {
                application,
                slot: DataFormat::create_bytes_platform_format(slot.identifier()),
            });
        }
        aliases
    }
}

impl FormatAlias {
    /// Encodes the application value for its native byte slot.
    pub fn encode(&self, value: &dyn Any) -> Option<Rc<[u8]>> {
        if self.application.data_type() == Some(TypeId::of::<String>()) {
            Some(Rc::from(value.downcast_ref::<String>()?.as_bytes()))
        } else {
            value.downcast_ref::<Rc<[u8]>>().cloned()
        }
    }

    /// Converts a native byte slot to the application's declared type.
    pub fn decode(&self, value: Rc<dyn Any>) -> Option<Rc<dyn Any>> {
        let bytes = value.downcast_ref::<Rc<[u8]>>()?;
        if self.application.data_type() == Some(TypeId::of::<String>()) {
            let text = std::str::from_utf8(bytes).ok()?;
            Some(Rc::new(text.to_owned()))
        } else {
            Some(value)
        }
    }
}

#[cfg(all(test, not(feature = "chromium-clipboard-map")))]
mod disabled_tests {
    use super::*;

    #[test]
    fn without_the_feature_no_map_is_found_published_or_read() {
        let clipboard = ChromiumClipboard::MacOs;
        let marker = DataFormat::create_string_application_format("Box");
        let map = DataFormat::create_bytes_platform_format("org.w3.web-custom-format.map");
        let slot = DataFormat::create_bytes_platform_format("org.w3.web-custom-format.type-0");
        assert!(clipboard.create_map(&[marker.into()]).is_none());
        assert!(clipboard.find_map(&[map.into()]).is_none());
        let json = br#"{"application/frn-fmt.text.426f78":"org.w3.web-custom-format.type-0"}"#;
        assert!(clipboard.parse_map(json, &[slot.into()]).is_empty());
    }
}

#[cfg(all(test, feature = "chromium-clipboard-map"))]
mod tests {
    use super::*;

    #[test]
    fn macos_pasteboard_map_keeps_typed_utf8_in_an_unoccupied_slot() {
        let clipboard = ChromiumClipboard::MacOs;
        let marker = DataFormat::create_string_application_format("Box");
        let occupied = DataFormat::create_bytes_platform_format("org.w3.web-custom-format.type-0");
        let map = clipboard
            .create_map(&[marker.clone().into(), occupied.into()])
            .unwrap();
        assert_eq!(
            clipboard.map_format().identifier(),
            "org.w3.web-custom-format.map"
        );
        assert_eq!(
            &*map.bytes,
            br#"{"application/frn-fmt.text.426f78":"org.w3.web-custom-format.type-1"}"#
        );
        let slot = &map.aliases[0].slot;
        let aliases = clipboard.parse_map(&map.bytes, &[slot.clone().into()]);
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].application, *marker);
        let text = "naïve 😀".to_owned();
        let encoded = map.aliases[0].encode(&text).unwrap();
        assert_eq!(&*encoded, text.as_bytes());
        assert_eq!(
            aliases[0]
                .decode(Rc::new(encoded))
                .unwrap()
                .downcast_ref::<String>(),
            Some(&text)
        );
    }

    #[test]
    fn macos_map_resolves_only_advertised_canonical_slots() {
        let clipboard = ChromiumClipboard::MacOs;
        let map = br#"{
            "application/frn-fmt.text.426f78":"org.w3.web-custom-format.type-1",
            "application/frn-fmt.text.4d697373":"org.w3.web-custom-format.type-2",
            "application/frn-fmt.text.48696464656e":"public.utf8-plain-text",
            "application/frn-fmt.bytes.426c6f62":"org.w3.web-custom-format.type-3"
        }"#;
        let slot = DataFormat::create_bytes_platform_format("org.w3.web-custom-format.type-3");
        let aliases = clipboard.parse_map(map, &[slot.into(), DataFormat::text().into()]);
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].application.identifier(), "Blob");
        let data: Rc<[u8]> = Rc::from([0, 255, 128, 0]);
        let value = aliases[0].decode(Rc::new(data.clone())).unwrap();
        assert_eq!(value.downcast_ref::<Rc<[u8]>>(), Some(&data));
    }
}
