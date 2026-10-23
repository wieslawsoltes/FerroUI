//! Typed paragraph direction and whole-layer OpenType authoring. UTF-8 range
//! styles remain in `text_edit`; native text controls use a UTF-16 projection.
use crate::{DocumentError, Editor, NodeId, NodeKind, TextStyleChange};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDirection {
    /// Resolve the first strong Unicode character using the renderer's Unicode
    /// bidi data; an empty or neutral-only paragraph falls back to LTR.
    Auto,
    #[serde(rename = "ltr")]
    LeftToRight,
    #[serde(rename = "rtl")]
    RightToLeft,
}

/// An OpenType tag is exactly four printable ASCII bytes. Values are unsigned
/// feature parameters, including alternate indices; zero disables the feature.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontFeatureSetting {
    pub tag: String,
    pub value: u32,
}

pub const MAX_FONT_FEATURES: usize = 128;

fn invalid(message: &str) -> DocumentError { DocumentError::Invalid(message.into()) }

pub fn validate_font_features(features: &[FontFeatureSetting]) -> Result<(), DocumentError> {
    if features.len() > MAX_FONT_FEATURES {
        return Err(invalid("Use at most 128 OpenType feature settings."));
    }
    let mut tags = BTreeSet::new();
    for feature in features {
        if feature.tag.len() != 4 || !feature.tag.bytes().all(|byte| (0x20..=0x7e).contains(&byte)) {
            return Err(invalid("An OpenType tag must contain exactly four printable ASCII characters."));
        }
        if !tags.insert(feature.tag.as_str()) {
            return Err(invalid("Each OpenType feature tag may appear only once."));
        }
    }
    Ok(())
}

/// Authoring syntax: `kern=1, liga=0, ss01=2`, `+kern`, or `-liga`. Quoted CSS
/// tags (`"liga" off`) are accepted too. Invalid/duplicate settings are rejected
/// as a whole. Blank text or `normal` restores the font's defaults.
pub fn parse_font_features(value: &str) -> Result<Vec<FontFeatureSetting>, DocumentError> {
    let value = value.trim();
    if value.is_empty() || value == "normal" { return Ok(Vec::new()); }
    let mut features = Vec::new();
    for item in feature_items(value)? {
        if features.len() >= MAX_FONT_FEATURES { return Err(invalid("Use at most 128 OpenType feature settings.")); }
        let item = item.trim();
        let (tag, parameter) = if item.starts_with(['\'', '"']) {
            let quote = item.as_bytes()[0] as char;
            let mut tag = String::new();
            let mut escaped = false;
            let mut end = None;
            for (offset, ch) in item[1..].char_indices() {
                if escaped { tag.push(ch); escaped = false; }
                else if ch == '\\' { escaped = true; }
                else if ch == quote { end = Some(offset + 2); break; }
                else { tag.push(ch); }
            }
            let end = end.ok_or_else(|| invalid("Close the quoted OpenType feature tag."))?;
            let rest = item[end..].trim();
            let parameter = if rest.is_empty() { None }
                else { Some(rest.strip_prefix('=').unwrap_or(rest).trim()) };
            (tag, parameter)
        } else {
            let (sign, item) = if let Some(rest) = item.strip_prefix('+') { (Some("1"), rest) }
                else if let Some(rest) = item.strip_prefix('-') { (Some("0"), rest) }
                else { (None, item) };
            if let Some(sign) = sign {
                if item.contains(['=', ' ', '\t']) { return Err(invalid("Use a sign or a value for an OpenType feature, not both.")); }
                (item.to_string(), Some(sign))
            } else if let Some((tag, parameter)) = item.split_once('=') {
                (tag.trim().to_string(), Some(parameter.trim()))
            } else { (item.to_string(), None) }
        };
        let parameter = match parameter {
            None | Some("on") => 1,
            Some("off") => 0,
            Some(value) if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) => value.parse::<u32>()
                .map_err(|_| invalid("OpenType feature values must be integers from 0 to 4294967295."))?,
            _ => return Err(invalid("OpenType feature values must be on, off or a nonnegative integer.")),
        };
        features.push(FontFeatureSetting { tag, value: parameter });
    }
    validate_font_features(&features)?;
    features.sort_by(|a, b| a.tag.cmp(&b.tag));
    Ok(features)
}

fn feature_items(value: &str) -> Result<Vec<&str>, DocumentError> {
    let mut result = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    for (index, ch) in value.char_indices() {
        if escaped { escaped = false; continue; }
        if let Some(current) = quote {
            if ch == '\\' { escaped = true; }
            else if ch == current { quote = None; }
        } else if ch == '\'' || ch == '"' { quote = Some(ch); }
        else if ch == ',' { result.push(&value[start..index]); start = index + 1; }
    }
    if quote.is_some() { return Err(invalid("Close the quoted OpenType feature tag.")); }
    result.push(&value[start..]);
    Ok(result)
}

pub fn format_font_features(features: &[FontFeatureSetting]) -> String {
    features.iter().map(|feature| {
        if feature.tag.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            format!("{}={}", feature.tag, feature.value)
        } else { format!("{}={}", serde_json::to_string(&feature.tag).expect("ASCII tag"), feature.value) }
    }).collect::<Vec<_>>().join(", ")
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextTypographyPatch {
    pub direction: TextStyleChange<TextDirection>,
    pub font_features: TextStyleChange<Vec<FontFeatureSetting>>,
}

impl Editor {
    /// Validate every target before beginning an edit. A missing, non-text or
    /// locked target fails the whole batch without changing history, redo,
    /// selection or revision. Each successful batch is one undoable operation.
    pub fn edit_text_typography(&mut self, ids: &[NodeId], patch: &TextTypographyPatch) -> Result<usize, DocumentError> {
        if let TextStyleChange::Set(features) = &patch.font_features { validate_font_features(features)?; }
        let mut prepared = BTreeMap::new();
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            let node = self.document.nodes.get(&id).ok_or(DocumentError::MissingNode(id))?;
            if node.kind != NodeKind::Text { return Err(invalid("Select text layers to change typography.")); }
            if self.document.effective_locked(id) { return Err(invalid("Unlock the text layer and its parents before editing.")); }
            let direction = match patch.direction {
                TextStyleChange::Keep => node.text_direction,
                TextStyleChange::Set(direction) => Some(direction),
                TextStyleChange::Inherit => None,
            };
            let features = match &patch.font_features {
                TextStyleChange::Keep => node.font_features.clone(),
                TextStyleChange::Set(features) => features.clone(),
                TextStyleChange::Inherit => Vec::new(),
            };
            if direction != node.text_direction || features != node.font_features {
                prepared.insert(id, (direction, features));
            }
        }
        let count = prepared.len();
        if count == 0 { return Ok(0); }
        let ids: Vec<_> = prepared.keys().copied().collect();
        self.update_nodes(&ids, "Change typography", |node| {
            let (direction, features) = prepared.remove(&node.id).expect("validated typography target");
            if node.text_direction != direction {
                node.text_direction = direction;
                node.overridden_properties.insert("text_direction".into());
            }
            if node.font_features != features {
                node.font_features = features;
                node.overridden_properties.insert("font_features".into());
            }
        });
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Node};

    fn fixture() -> (Editor, NodeId, NodeId, NodeId) {
        let mut document = Document::new("Typography");
        let page = document.pages[0].id;
        let frame = document.insert_node(page, Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 400.0, 100.0)).unwrap();
        let mut text = Node::new(NodeKind::Text, "Text", 0.0, 0.0, 150.0, 40.0);
        text.text = "office 東京 👩🏽‍💻 שלום".into();
        let a = document.insert_node(frame, text.clone()).unwrap();
        let b = document.insert_node(frame, text).unwrap();
        let mut editor = Editor::new(document);
        editor.selection = vec![a, b];
        (editor, frame, a, b)
    }

    #[test]
    fn legacy_documents_keep_their_shape_and_explicit_direction_round_trips() {
        let (mut editor, _, a, _) = fixture();
        let legacy = editor.document.to_json().unwrap();
        assert!(!legacy.contains("text_direction"));
        assert!(!legacy.contains("font_features"));
        assert_eq!(Document::from_json(&legacy).unwrap().to_json().unwrap(), legacy);
        for direction in [TextDirection::Auto, TextDirection::LeftToRight, TextDirection::RightToLeft] {
            editor.edit_text_typography(&[a], &TextTypographyPatch { direction: TextStyleChange::Set(direction), ..Default::default() }).unwrap();
            let json = editor.document.to_json().unwrap();
            assert_eq!(Document::from_json(&json).unwrap().nodes[&a].text_direction, Some(direction));
        }
    }

    #[test]
    fn feature_authoring_is_typed_strict_and_round_trippable() {
        let features = parse_font_features(" +kern, -liga, \"ss01\" 2, 'aalt'=4294967295 ").unwrap();
        assert_eq!(features.len(), 4);
        assert_eq!(features[0], FontFeatureSetting { tag: "aalt".into(), value: u32::MAX });
        assert_eq!(parse_font_features(&format_font_features(&features)).unwrap(), features);
        assert!(parse_font_features("normal").unwrap().is_empty());
        for invalid in ["", "kern=", "kern=1,"] {
            // A blank complete field is the explicit reset; an incomplete entry
            // never silently enables a feature.
            if !invalid.is_empty() { assert!(parse_font_features(invalid).is_err(), "{invalid}"); }
        }
        for invalid in ["kern,,liga", "kern, kern=0", "керн=1", "abc=1", "abcde=1", "kern=-1", "kern=1.5", "kern=4294967296", "kern[0:2]=1", "+kern=0", "\"kern=1"] {
            assert!(parse_font_features(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn direction_and_features_batch_is_one_undo_and_preserves_unicode_ranges() {
        let (mut editor, _, a, b) = fixture();
        let before = editor.document.to_json().unwrap();
        let patch = TextTypographyPatch {
            direction: TextStyleChange::Set(TextDirection::RightToLeft),
            font_features: TextStyleChange::Set(parse_font_features("kern=0, liga=0").unwrap()),
        };
        assert_eq!(editor.edit_text_typography(&[a, b, a], &patch).unwrap(), 2);
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.selection, [a, b]);
        let after = editor.document.to_json().unwrap();
        editor.undo();
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.edit_text_typography(&[a], &TextTypographyPatch::default()).unwrap(), 0);
        editor.redo();
        assert_eq!(editor.document.to_json().unwrap(), after);
        assert_eq!(editor.edit_text_typography(&[a, b], &patch).unwrap(), 0);
        assert_eq!(editor.history_len(), 1);
    }

    #[test]
    fn invalid_locked_and_stale_batches_are_atomic() {
        let (mut editor, frame, a, b) = fixture();
        let before = editor.document.to_json().unwrap();
        let invalid = TextTypographyPatch { font_features: TextStyleChange::Set(vec![FontFeatureSetting { tag: "bad".into(), value: 1 }]), ..Default::default() };
        assert!(editor.edit_text_typography(&[a, b], &invalid).is_err());
        let patch = TextTypographyPatch { direction: TextStyleChange::Set(TextDirection::Auto), ..Default::default() };
        assert!(editor.edit_text_typography(&[a, u64::MAX], &patch).is_err());
        assert!(editor.edit_text_typography(&[a, frame], &patch).is_err());
        assert_eq!(editor.document.to_json().unwrap(), before);
        assert_eq!(editor.history_len(), 0);
        editor.document.nodes.get_mut(&frame).unwrap().locked = true;
        let locked = editor.document.to_json().unwrap();
        assert!(editor.edit_text_typography(&[a, b], &patch).is_err());
        assert_eq!(editor.document.to_json().unwrap(), locked);
        assert_eq!(editor.history_len(), 0);
    }

    #[test]
    fn imported_invalid_features_fail_document_validation() {
        let (mut editor, _, a, _) = fixture();
        for features in [vec![FontFeatureSetting { tag: "🦀".into(), value: 1 }],
            vec![FontFeatureSetting { tag: "kern".into(), value: 1 }; 2],
            vec![FontFeatureSetting { tag: "kern".into(), value: 1 }; MAX_FONT_FEATURES + 1]] {
            editor.document.nodes.get_mut(&a).unwrap().font_features = features;
            assert!(Document::from_json(&editor.document.to_json().unwrap()).is_err());
        }
    }
}
