//! Paragraph styles use whole, disjoint UTF-8 ranges. Character styling and
//! paragraph styling share the normal text replacement/history boundary.
use crate::{
    DocumentError, Editor, Node, NodeId, NodeKind, TextAlign, TextDirection, TextStyleChange,
    TextStyleError,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextParagraph {
    pub start: usize,
    pub end: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<TextAlign>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<TextDirection>,
    /// Multiplier of the layer's base font size, like Node::line_height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub letter_spacing: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_before: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub space_after: Option<f64>,
    /// Logical first-line indentation, measured from the paragraph start edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<f64>,
}

impl TextParagraph {
    pub fn has_style(&self) -> bool {
        self.alignment.is_some()
            || self.direction.is_some()
            || self.line_height.is_some()
            || self.letter_spacing.is_some()
            || self.space_before.is_some()
            || self.space_after.is_some()
            || self.indent.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParagraphStylePatch {
    pub alignment: TextStyleChange<TextAlign>,
    pub direction: TextStyleChange<TextDirection>,
    pub line_height: TextStyleChange<f64>,
    pub letter_spacing: TextStyleChange<f64>,
    pub space_before: TextStyleChange<f64>,
    pub space_after: TextStyleChange<f64>,
    pub indent: TextStyleChange<f64>,
}

impl ParagraphStylePatch {
    pub fn inherit_all() -> Self {
        Self {
            alignment: TextStyleChange::Inherit,
            direction: TextStyleChange::Inherit,
            line_height: TextStyleChange::Inherit,
            letter_spacing: TextStyleChange::Inherit,
            space_before: TextStyleChange::Inherit,
            space_after: TextStyleChange::Inherit,
            indent: TextStyleChange::Inherit,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self.alignment, TextStyleChange::Keep)
            && matches!(self.direction, TextStyleChange::Keep)
            && matches!(self.line_height, TextStyleChange::Keep)
            && matches!(self.letter_spacing, TextStyleChange::Keep)
            && matches!(self.space_before, TextStyleChange::Keep)
            && matches!(self.space_after, TextStyleChange::Keep)
            && matches!(self.indent, TextStyleChange::Keep)
    }

    fn apply(&self, value: &mut TextParagraph) {
        fn apply<T: Clone>(target: &mut Option<T>, change: &TextStyleChange<T>) {
            match change {
                TextStyleChange::Keep => {}
                TextStyleChange::Set(value) => *target = Some(value.clone()),
                TextStyleChange::Inherit => *target = None,
            }
        }
        apply(&mut value.alignment, &self.alignment);
        apply(&mut value.direction, &self.direction);
        apply(&mut value.line_height, &self.line_height);
        apply(&mut value.letter_spacing, &self.letter_spacing);
        apply(&mut value.space_before, &self.space_before);
        apply(&mut value.space_after, &self.space_after);
        apply(&mut value.indent, &self.indent);
    }

    fn validate(&self) -> Result<(), TextStyleError> {
        let mut value = TextParagraph::default();
        self.apply(&mut value);
        validate_values(&value).map_err(TextStyleError::InvalidStyle)
    }
}

fn validate_values(value: &TextParagraph) -> Result<(), &'static str> {
    if value
        .line_height
        .is_some_and(|v| !v.is_finite() || v <= 0.0 || v > 100.0)
    {
        return Err("Paragraph line height must be greater than zero and at most 100.");
    }
    if value
        .letter_spacing
        .is_some_and(|v| !v.is_finite() || v.abs() > 1_000_000.0)
    {
        return Err(
            "Paragraph letter spacing must be a finite value between -1000000 and 1000000.",
        );
    }
    if [value.space_before, value.space_after, value.indent]
        .into_iter()
        .flatten()
        .any(|v| !v.is_finite() || !(0.0..=1_000_000.0).contains(&v))
    {
        return Err("Paragraph spacing and indentation must be between 0 and 1000000.");
    }
    Ok(())
}

/// Required line breaks define paragraphs. CRLF is one break. The terminator
/// belongs to the preceding paragraph; text ending in a break has a final empty
/// paragraph so its caret and future typing can retain a style.
pub fn text_paragraph_ranges(text: &str) -> Vec<Range<usize>> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((at, ch)) = chars.next() {
        if !matches!(ch, '\r' | '\n' | '\u{0085}' | '\u{2028}' | '\u{2029}') {
            continue;
        }
        let mut end = at + ch.len_utf8();
        if ch == '\r' && chars.peek().is_some_and(|(_, ch)| *ch == '\n') {
            end = chars.next().unwrap().0 + 1;
        }
        result.push(start..end);
        start = end;
    }
    result.push(start..text.len());
    result
}

pub fn validate_text_paragraphs(node: &Node) -> Result<(), DocumentError> {
    if node.text_paragraphs.is_empty() {
        return Ok(());
    }
    if node.kind != NodeKind::Text || node.text_paragraphs.len() > 100_000 {
        return Err(DocumentError::Invalid(
            "Paragraph styles require a text layer and at most 100000 ranges.".into(),
        ));
    }
    let ranges = text_paragraph_ranges(&node.text);
    let mut previous = None;
    for style in &node.text_paragraphs {
        if previous.is_some_and(|start| start >= style.start)
            || ranges
                .binary_search_by_key(&style.start, |range| range.start)
                .ok()
                .is_none_or(|index| ranges[index].end != style.end)
        {
            return Err(DocumentError::Invalid(
                "Paragraph ranges must be ordered, disjoint complete UTF-8 paragraphs.".into(),
            ));
        }
        validate_values(style).map_err(|message| DocumentError::Invalid(message.into()))?;
        previous = Some(style.start);
    }
    Ok(())
}

fn at(ranges: &[Range<usize>], offset: usize) -> usize {
    ranges
        .partition_point(|range| range.start <= offset)
        .saturating_sub(1)
}

impl Node {
    pub fn apply_paragraph_style(
        &mut self,
        range: Range<usize>,
        patch: &ParagraphStylePatch,
    ) -> Result<usize, TextStyleError> {
        if self.kind != NodeKind::Text {
            return Err(TextStyleError::NotText);
        }
        if range.start > range.end
            || range.end > self.text.len()
            || !self.text.is_char_boundary(range.start)
            || !self.text.is_char_boundary(range.end)
        {
            return Err(TextStyleError::InvalidRange);
        }
        patch.validate()?;
        validate_text_paragraphs(self).map_err(|_| TextStyleError::InvalidExistingRun)?;
        let ranges = text_paragraph_ranges(&self.text);
        let existing: BTreeMap<_, _> = self
            .text_paragraphs
            .iter()
            .map(|style| (style.start, style))
            .collect();
        let caret = at(&ranges, range.start);
        let mut next = Vec::with_capacity(ranges.len());
        let mut changed = 0;
        for (index, paragraph) in ranges.iter().enumerate() {
            let mut value = existing
                .get(&paragraph.start)
                .map(|style| (**style).clone())
                .unwrap_or_else(|| TextParagraph {
                    start: paragraph.start,
                    end: paragraph.end,
                    ..Default::default()
                });
            let old = value.clone();
            let touched = if range.is_empty() {
                index == caret
            } else {
                paragraph.start < range.end && paragraph.end > range.start
            };
            if touched {
                patch.apply(&mut value);
            }
            if value != old {
                changed += 1;
            }
            if value.has_style() {
                next.push(value);
            }
        }
        if changed > 0 {
            self.text_paragraphs = next;
        }
        Ok(changed)
    }
}

impl Editor {
    pub fn style_text_paragraphs(
        &mut self,
        id: NodeId,
        range: Range<usize>,
        patch: &ParagraphStylePatch,
    ) -> Result<usize, TextStyleError> {
        if self.document.effective_locked(id) {
            return Err(TextStyleError::Locked);
        }
        self.update_node(id, "Style paragraphs", |node| {
            let changed = node.apply_paragraph_style(range, patch)?;
            if changed > 0 && node.component_source.is_some() {
                node.overridden_properties.insert("text_paragraphs".into());
            }
            Ok(changed)
        })
        .unwrap_or(Err(TextStyleError::MissingNode(id)))
    }
}

/// Map actual new paragraph starts through the same exact changed byte interval
/// used for character styles. Splits inherit the source paragraph; deleting a
/// break keeps the surviving first paragraph's style. Unchanged later paragraphs
/// retain their own style and shift only by the replacement's byte delta.
pub(crate) fn remap(
    node: &Node,
    text: &str,
    prefix: usize,
    old_end: usize,
    new_end: usize,
) -> Vec<TextParagraph> {
    if node.text_paragraphs.is_empty() {
        return Vec::new();
    }
    let old_ranges = text_paragraph_ranges(&node.text);
    let styles: BTreeMap<_, _> = node
        .text_paragraphs
        .iter()
        .map(|style| (style.start, style))
        .collect();
    text_paragraph_ranges(text)
        .into_iter()
        .filter_map(|range| {
            // At a deletion boundary the new paragraph starts with the
            // surviving suffix, even when the removed text was a complete
            // paragraph (or only the LF half of CRLF). A paragraph that
            // survives before the cut still owns a separator-only merge.
            let source = if range.start < prefix {
                range.start
            } else if range.start >= new_end {
                old_end + range.start - new_end
            } else {
                prefix
            };
            let original = old_ranges.get(at(&old_ranges, source.min(node.text.len())))?;
            let mut style = (**styles.get(&original.start)?).clone();
            style.start = range.start;
            style.end = range.end;
            style.has_style().then_some(style)
        })
        .collect()
}

#[cfg(test)]
#[path = "paragraphs_tests.rs"]
mod tests;
