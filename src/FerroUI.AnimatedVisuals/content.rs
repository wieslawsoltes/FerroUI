//! What image and text layers draw: the image assets of a document and its text documents, with
//! the glyph shapes a document can carry.

use crate::model::{shape_items, ShapeItem};
use crate::value::Components;
use serde_json::Value;
use std::collections::HashMap;
use std::rc::Rc;

/// An image of a document: its bytes, from the document or from the loader of the source.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageAsset {
    pub id: String,
    /// The size it is drawn at, in the space of its layer.
    pub width: f64,
    pub height: f64,
    /// The encoded image. `None` until a loader gave the bytes of an image in a file.
    pub bytes: Option<Rc<[u8]>>,
    /// Where the file is, for an image that is not in the document.
    pub path: Option<String>,
}

fn base64_value(byte: u8) -> Option<u32> {
    match byte {
        b'A'..=b'Z' => Some(u32::from(byte - b'A')),
        b'a'..=b'z' => Some(u32::from(byte - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(byte - b'0') + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    }
}

/// The bytes of base64 text. White space is skipped; anything else that is not base64 is an error.
pub(crate) fn decode_base64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut bits, mut count) = (0u32, 0u32);
    for byte in text.bytes() {
        if byte == b'=' {
            break;
        }
        if byte.is_ascii_whitespace() {
            continue;
        }
        bits = (bits << 6) | base64_value(byte)?;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    Some(out)
}

impl ImageAsset {
    /// The image an entry of `assets` describes, if it describes one.
    pub(crate) fn parse(asset: &Value) -> Option<Self> {
        if asset.get("layers").is_some() {
            return None;
        }
        let id = asset.get("id")?.as_str()?.to_owned();
        let name = asset.get("p")?.as_str()?;
        let number = |key: &str| asset.get(key).and_then(Value::as_f64).unwrap_or(0.0);
        let (bytes, path) = match name.strip_prefix("data:") {
            Some(data) => {
                let (kind, payload) = data.split_once(',')?;
                if !kind.ends_with(";base64") {
                    return None;
                }
                (Some(Rc::from(decode_base64(payload)?)), None)
            }
            None => {
                let folder = asset.get("u").and_then(Value::as_str).unwrap_or("");
                (None, Some(format!("{folder}{name}")))
            }
        };
        Some(Self { id, width: number("w"), height: number("h"), bytes, path })
    }
}

/// The text of a layer from a time on.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextDocument {
    pub(crate) text: String,
    pub(crate) font: String,
    pub(crate) size: f64,
    pub(crate) fill: Option<Components>,
    pub(crate) stroke: Option<Components>,
    pub(crate) stroke_width: f64,
    pub(crate) stroke_over_fill: bool,
    /// 0 left, 1 right, 2 centre.
    pub(crate) justify: u8,
    pub(crate) line_height: f64,
    /// In thousandths of the size.
    pub(crate) tracking: f64,
    pub(crate) baseline_shift: f64,
}

fn text_color(value: Option<&Value>) -> Option<Components> {
    let items = value?.as_array()?;
    let at = |index: usize| items.get(index).and_then(Value::as_f64);
    Some([at(0)?, at(1)?, at(2)?, at(3).unwrap_or(1.0)])
}

impl TextDocument {
    fn parse(value: &Value) -> Option<Self> {
        let number = |key: &str, default: f64| value.get(key).and_then(Value::as_f64).unwrap_or(default);
        let size = number("s", 10.0);
        Some(Self {
            text: value.get("t")?.as_str()?.to_owned(),
            font: value.get("f").and_then(Value::as_str).unwrap_or("").to_owned(),
            size,
            fill: text_color(value.get("fc")),
            stroke: text_color(value.get("sc")),
            stroke_width: number("sw", 0.0),
            stroke_over_fill: value.get("of").and_then(Value::as_bool).unwrap_or(false),
            justify: value.get("j").and_then(Value::as_u64).unwrap_or(0) as u8,
            line_height: number("lh", size * 1.2),
            tracking: number("tr", 0.0),
            baseline_shift: number("ls", 0.0),
        })
    }
}

/// The documents of a text layer (`t.d.k`), each held until the next.
pub(crate) fn text_documents(layer: &Value) -> Vec<(f64, TextDocument)> {
    let keys = layer.get("t").and_then(|text| text.get("d")).and_then(|data| data.get("k")).and_then(Value::as_array);
    keys.into_iter()
        .flatten()
        .filter_map(|key| {
            let time = key.get("t").and_then(Value::as_f64).unwrap_or(0.0);
            Some((time, TextDocument::parse(key.get("s")?)?))
        })
        .collect()
}

/// The document shown at `frame`.
pub(crate) fn document_at(documents: &[(f64, TextDocument)], frame: f64) -> Option<&TextDocument> {
    let after = documents.partition_point(|(time, _)| *time <= frame);
    documents.get(after.saturating_sub(1)).map(|(_, document)| document)
}

/// The lines of a text: Lottie ends them with a carriage return, a line feed or `U+0003`.
pub(crate) fn lines_of(text: &str) -> impl Iterator<Item = &str> {
    text.split(['\r', '\n', '\u{3}'])
}

/// The shape of a character a document carries, its units a hundredth of the font size.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Glyph {
    pub(crate) advance: f64,
    pub(crate) shapes: Vec<ShapeItem>,
}

/// The fonts of a document and the glyphs it carries.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct DocumentFonts {
    /// The family of each font name.
    pub(crate) families: HashMap<String, String>,
    /// By family, then character.
    pub(crate) glyphs: HashMap<String, HashMap<char, Glyph>>,
}

impl DocumentFonts {
    pub(crate) fn parse(root: &Value) -> Self {
        let mut fonts = Self::default();
        let list = root.get("fonts").and_then(|fonts| fonts.get("list")).and_then(Value::as_array);
        for font in list.into_iter().flatten() {
            if let (Some(name), Some(family)) =
                (font.get("fName").and_then(Value::as_str), font.get("fFamily").and_then(Value::as_str))
            {
                fonts.families.insert(name.to_owned(), family.to_owned());
            }
        }
        for item in root.get("chars").and_then(Value::as_array).into_iter().flatten() {
            let Some(character) = item.get("ch").and_then(Value::as_str).and_then(|text| text.chars().next()) else {
                continue;
            };
            let family = item.get("fFamily").and_then(Value::as_str).unwrap_or("").to_owned();
            let advance = item.get("w").and_then(Value::as_f64).unwrap_or(0.0);
            let shapes = shape_items(item.get("data").and_then(|data| data.get("shapes")));
            fonts.glyphs.entry(family).or_default().insert(character, Glyph { advance, shapes });
        }
        fonts
    }

    /// The glyphs of the font `name`, when the document carries every character of `text`.
    pub(crate) fn glyphs_for(&self, name: &str, text: &str) -> Option<&HashMap<char, Glyph>> {
        let family = self.families.get(name).map_or(name, String::as_str);
        let glyphs = self.glyphs.get(family)?;
        lines_of(text).flat_map(str::chars).all(|character| glyphs.contains_key(&character)).then_some(glyphs)
    }

    /// The family to ask the system for.
    pub(crate) fn family_of<'a>(&'a self, name: &'a str) -> &'a str {
        self.families.get(name).map_or(name, String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn base64_is_decoded_and_rejects_what_is_not_base64() {
        assert_eq!(decode_base64("TWFu"), Some(b"Man".to_vec()));
        assert_eq!(decode_base64("TWE="), Some(b"Ma".to_vec()));
        assert_eq!(decode_base64("T Q\n=="), Some(b"M".to_vec()));
        assert_eq!(decode_base64("T*Q="), None);
    }

    #[test]
    fn an_image_is_in_the_document_or_in_a_file() {
        let inside = ImageAsset::parse(&json!({"id": "a", "w": 2, "h": 3, "p": "data:image/png;base64,TWFu"})).unwrap();
        assert_eq!(inside.bytes.as_deref(), Some(&b"Man"[..]));
        let file = ImageAsset::parse(&json!({"id": "b", "w": 2, "h": 3, "u": "images/", "p": "b.png"})).unwrap();
        assert_eq!((file.bytes, file.path.as_deref()), (None, Some("images/b.png")));
        assert!(ImageAsset::parse(&json!({"id": "c", "layers": []})).is_none());
    }

    #[test]
    fn the_document_at_a_frame_is_held_until_the_next() {
        let layer = json!({"t": {"d": {"k": [{"t": 0, "s": {"t": "one", "s": 12}}, {"t": 10, "s": {"t": "two", "s": 12}}]}}});
        let documents = text_documents(&layer);
        assert_eq!(document_at(&documents, 9.0).unwrap().text, "one");
        assert_eq!(document_at(&documents, 10.0).unwrap().text, "two");
        assert_eq!(lines_of("a\rb\u{3}c").collect::<Vec<_>>(), ["a", "b", "c"]);
    }
}
