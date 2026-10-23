//! RTF emission of common-model hyperlink fields. Field boundaries live in the
//! document's visible UTF-16 space, independently of paragraph-owned inline
//! projections. Formatting groups close at leaf boundaries so one field group
//! can cross spans, paragraphs and empty paragraph marks without becoming nested.

use super::{escape, Encoder, RtfError};
use crate::documents::{
    CharacterFormat, DocumentLinkField, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind,
    ParagraphSnapshot,
};
use ferroui_base::media::{FontStyle, FontWeight, TextDecorations};
use std::collections::HashMap;
use std::ops::Range;

struct Field<'a> {
    range: Range<usize>,
    target: &'a str,
    target_name: Option<&'a str>,
}

fn fields(document: &DocumentSnapshot) -> Result<Vec<Field<'_>>, RtfError> {
    let mut result = Vec::with_capacity(document.link_fields.len());
    let mut registered = HashMap::new();
    for field in &document.link_fields {
        if field.has_sentinel {
            return Err(RtfError::UnsupportedFeature(
                "hyperlink sentinel RTF encoding",
            ));
        }
        registered.insert(field.id, field);
        result.push(Field {
            range: field.range.clone(),
            target: &field.target,
            target_name: field.target_name.as_deref(),
        });
    }
    let mut position = 0;
    for (index, paragraph) in document.paragraphs.iter().enumerate() {
        if index != 0 {
            position += 1;
        }
        position = collect_legacy_fields(&paragraph.inlines, position, &registered, &mut result)?;
    }
    result.sort_by_key(|field| field.range.start);
    let mut previous_end = 0;
    for field in &result {
        if field.range.start >= field.range.end || field.range.start < previous_end {
            return Err(RtfError::UnsupportedFeature(
                "empty or nested hyperlink fields",
            ));
        }
        if field.target.contains(['"', '\0']) {
            return Err(RtfError::InvalidParameter("hyperlink URI"));
        }
        if field.target_name.is_some_and(|name| name.contains('"')) {
            return Err(RtfError::InvalidParameter("hyperlink target"));
        }
        previous_end = field.range.end;
    }
    Ok(result)
}

/// Legacy inline-only hyperlinks still export as independent fields. Explicit
/// projections are suppressed only after checking their associated record;
/// matching target text alone never merges adjacent field identities.
fn collect_legacy_fields<'a>(
    inlines: &'a [InlineSnapshot],
    mut position: usize,
    registered: &HashMap<u64, &'a DocumentLinkField>,
    fields: &mut Vec<Field<'a>>,
) -> Result<usize, RtfError> {
    for inline in inlines {
        let start = position;
        position = match inline.children() {
            Some(children) => collect_legacy_fields(children, position, registered, fields)?,
            None => position + inline.text_length(),
        };
        if let InlineSnapshotKind::Hyperlink {
            field_id,
            navigate_uri,
            target_name,
            ..
        } = &inline.kind
        {
            if let Some(id) = field_id {
                let record = registered
                    .get(id)
                    .ok_or(RtfError::InvalidParameter("hyperlink field projection"))?;
                if start < record.range.start
                    || position > record.range.end
                    || navigate_uri.as_deref() != Some(record.target.as_str())
                    || target_name != &record.target_name
                {
                    return Err(RtfError::InvalidParameter("hyperlink field projection"));
                }
            } else if let Some(target) = navigate_uri {
                fields.push(Field {
                    range: start..position,
                    target,
                    target_name: target_name.as_deref(),
                });
            }
        }
    }
    Ok(position)
}

impl Encoder {
    pub(super) fn link_field_document(
        &mut self,
        document: &DocumentSnapshot,
    ) -> Result<(), RtfError> {
        let Some(first) = document.paragraphs.first() else {
            return Err(RtfError::InvalidParameter("hyperlink field range"));
        };
        let mut writer = FieldEncoder {
            encoder: self,
            fields: fields(document)?,
            next: 0,
            active: false,
            position: 0,
            paragraph: first,
        };
        for (index, paragraph) in document.paragraphs.iter().enumerate() {
            if index != 0 {
                writer.boundary()?;
                writer.encoder.output.push_str("\\par\n");
                writer.position += 1;
            }
            writer.paragraph = paragraph;
            writer.paragraph_defaults()?;
            writer.boundary()?;
            writer.inlines(&paragraph.inlines, &CharacterFormat::default())?;
            writer.boundary()?;
        }
        if writer.active || writer.next != writer.fields.len() {
            return Err(RtfError::InvalidParameter("hyperlink field range"));
        }
        Ok(())
    }
}

struct FieldEncoder<'a, 'e> {
    encoder: &'e mut Encoder,
    fields: Vec<Field<'a>>,
    next: usize,
    active: bool,
    position: usize,
    paragraph: &'a ParagraphSnapshot,
}

impl FieldEncoder<'_, '_> {
    fn paragraph_defaults(&mut self) -> Result<(), RtfError> {
        self.encoder.output.push_str("\\pard\\plain");
        self.encoder.paragraph_format(&self.paragraph.format)?;
        self.encoder.format(&self.paragraph.format.character)?;
        self.encoder.output.push(' ');
        Ok(())
    }

    fn boundary(&mut self) -> Result<(), RtfError> {
        if self.active && self.fields[self.next].range.end == self.position {
            self.encoder.output.push_str("}}");
            self.active = false;
            self.next += 1;
            // Closing fldrslt restores its opening state. A field may have
            // crossed paragraphs, so restore the current paragraph before its
            // suffix or following mark is emitted outside that group.
            self.paragraph_defaults()?;
        }
        if !self.active {
            if let Some(field) = self.fields.get(self.next) {
                if field.range.start < self.position {
                    return Err(RtfError::InvalidParameter("hyperlink field boundary"));
                }
                if field.range.start == self.position {
                    self.encoder
                        .output
                        .push_str("{\\field{\\*\\fldinst HYPERLINK \"");
                    escape(field.target, &mut self.encoder.output);
                    self.encoder.output.push('"');
                    if let Some(name) = field.target_name {
                        self.encoder.output.push_str(" \\\\t \"");
                        escape(name, &mut self.encoder.output);
                        self.encoder.output.push('"');
                    }
                    self.encoder.output.push_str("}{\\fldrslt ");
                    self.active = true;
                }
            }
        }
        Ok(())
    }

    fn next_boundary(&self) -> usize {
        self.fields.get(self.next).map_or(usize::MAX, |field| {
            if self.active {
                field.range.end
            } else {
                field.range.start
            }
        })
    }

    fn start_format(&mut self, format: &CharacterFormat) -> Result<(), RtfError> {
        self.encoder.output.push('{');
        let start = self.encoder.output.len();
        self.encoder.format(format)?;
        if self.encoder.output.len() != start {
            self.encoder.output.push(' ');
        }
        Ok(())
    }

    fn text(&mut self, text: &str, format: &CharacterFormat) -> Result<(), RtfError> {
        let total = text.encode_utf16().count();
        let mut consumed = 0;
        let mut byte = 0;
        while consumed < total {
            self.boundary()?;
            let count = (self.next_boundary() - self.position).min(total - consumed);
            let mut length = 0;
            let mut end = byte;
            for (relative, character) in text[byte..].char_indices() {
                length += character.len_utf16();
                if length > count {
                    return Err(RtfError::InvalidUnicode);
                }
                end = byte + relative + character.len_utf8();
                if length == count {
                    break;
                }
            }
            if count == 0 || length != count {
                return Err(RtfError::InvalidParameter("hyperlink field boundary"));
            }
            self.start_format(format)?;
            escape(&text[byte..end], &mut self.encoder.output);
            self.encoder.output.push('}');
            self.position += count;
            consumed += count;
            byte = end;
        }
        Ok(())
    }

    fn inlines(
        &mut self,
        inlines: &[InlineSnapshot],
        inherited: &CharacterFormat,
    ) -> Result<(), RtfError> {
        for inline in inlines {
            let mut format = inherited.clone();
            match &inline.kind {
                InlineSnapshotKind::Bold(_) => format.font_weight = Some(FontWeight::Bold),
                InlineSnapshotKind::Italic(_) => format.font_style = Some(FontStyle::Italic),
                InlineSnapshotKind::Underline(_) => {
                    format.text_decorations = Some(Some(TextDecorations::underline()));
                }
                _ => {}
            }
            format = format.overlay(&inline.format);
            match &inline.kind {
                InlineSnapshotKind::Run(text) => self.text(text, &format)?,
                InlineSnapshotKind::LineBreak => {
                    self.boundary()?;
                    let end = self.position + inline.text_length();
                    if self.next_boundary() < end {
                        return Err(RtfError::UnsupportedFeature(
                            "hyperlink boundary within a line break",
                        ));
                    }
                    self.start_format(&format)?;
                    self.encoder.output.push_str("\\line }");
                    self.position = end;
                }
                InlineSnapshotKind::Embedded(_) => {
                    return Err(RtfError::UnsupportedFeature("embedded controls"));
                }
                _ => self.inlines(inline.children().unwrap_or_default(), &format)?,
            }
        }
        Ok(())
    }
}
