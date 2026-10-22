use super::{
    HighlightedSection, HighlightingColor, HighlightingError, HighlightingFontStyle, HtmlOptions,
    RichTextWriter,
};
use crate::document::{
    AnchorMovementType, DocumentChangeEventArgs, DocumentError, OffsetChangeMap,
    OffsetChangeMapEntry, TextChangeEventArgs,
};
use std::fmt;

/// Errors from rich-text ranges, formatting, writers, and HTML export.
#[derive(Debug)]
pub enum RichTextError {
    InvalidRange {
        offset: usize,
        length: usize,
        text_length: usize,
    },
    InvalidLineRange {
        start: usize,
        end: usize,
        line_start: usize,
        line_end: usize,
    },
    RangeOverflow {
        offset: usize,
        length: usize,
    },
    InvalidSections {
        index: usize,
        reason: &'static str,
    },
    DifferentLine,
    SpanUnderflow,
    UnclosedSpans(usize),
    IndentationUnderflow,
    InvalidOption(&'static str),
    InvalidCssValue {
        property: &'static str,
        value: String,
    },
    InvalidHyperlink(String),
    Document(DocumentError),
    Highlighting(HighlightingError),
}

impl fmt::Display for RichTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange {
                offset,
                length,
                text_length,
            } => write!(
                f,
                "UTF-16 range ({offset}, {length}) is outside text length {text_length}"
            ),
            Self::InvalidLineRange {
                start,
                end,
                line_start,
                line_end,
            } => {
                write!(
                    f,
                    "UTF-16 range {start}..{end} is outside line {line_start}..{line_end}"
                )
            }
            Self::RangeOverflow { offset, length } => {
                write!(f, "UTF-16 range ({offset}, {length}) overflows")
            }
            Self::InvalidSections { index, reason } => {
                write!(f, "Invalid highlighted section {index}: {reason}")
            }
            Self::DifferentLine => f.write_str("Highlighted lines refer to different line numbers"),
            Self::SpanUnderflow => f.write_str("No rich-text span is open"),
            Self::UnclosedSpans(count) => write!(f, "{count} rich-text spans are still open"),
            Self::IndentationUnderflow => f.write_str("Indentation is already zero"),
            Self::InvalidOption(reason) => write!(f, "Invalid rich-text option: {reason}"),
            Self::InvalidCssValue { property, value } => {
                write!(f, "Invalid CSS {property} value {value:?}")
            }
            Self::InvalidHyperlink(value) => write!(f, "Unsupported hyperlink {value:?}"),
            Self::Document(error) => error.fmt(f),
            Self::Highlighting(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for RichTextError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Document(error) => Some(error),
            Self::Highlighting(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DocumentError> for RichTextError {
    fn from(value: DocumentError) -> Self {
        Self::Document(value)
    }
}

impl From<HighlightingError> for RichTextError {
    fn from(value: HighlightingError) -> Self {
        Self::Highlighting(value)
    }
}

pub(super) fn checked_end(offset: usize, length: usize) -> Result<usize, RichTextError> {
    offset
        .checked_add(length)
        .ok_or(RichTextError::RangeOverflow { offset, length })
}

pub(super) fn check_range(
    offset: usize,
    length: usize,
    text_length: usize,
) -> Result<usize, RichTextError> {
    offset
        .checked_add(length)
        .filter(|end| offset <= text_length && *end <= text_length)
        .ok_or(RichTextError::InvalidRange {
            offset,
            length,
            text_length,
        })
}

/// Mutable formatting indexed by UTF-16 offsets, independent of any text source.
///
/// The last format extends indefinitely. Ranges are checked for arithmetic
/// overflow here; a `RichText` value additionally checks against its text length.
/// Adjacent equal formats are coalesced, and queries include unformatted gaps.
#[derive(Clone, Debug, PartialEq)]
pub struct RichTextModel {
    changes: Vec<(usize, HighlightingColor)>,
}

impl Default for RichTextModel {
    fn default() -> Self {
        Self {
            changes: vec![(0, HighlightingColor::default())],
        }
    }
}

impl RichTextModel {
    pub fn new() -> Self {
        Self::default()
    }

    fn index_at(&self, offset: usize) -> usize {
        self.changes.partition_point(|(start, _)| *start <= offset) - 1
    }

    fn split_at(&mut self, offset: usize) -> usize {
        match self
            .changes
            .binary_search_by_key(&offset, |(start, _)| *start)
        {
            Ok(index) => index,
            Err(index) => {
                let color = self.changes[index - 1].1.clone();
                self.changes.insert(index, (offset, color));
                index
            }
        }
    }

    fn normalize(&mut self) {
        self.changes.dedup_by(|right, left| right.1 == left.1);
    }

    fn normalize_range(&mut self, start: usize, end: usize) {
        // Only the edited entries and the adjoining formats can now be equal.
        // In particular, append-only writers must not rescan the whole document.
        let mut write = start + 1;
        for read in start + 1..end {
            if self.changes[read].1 != self.changes[write - 1].1 {
                if read != write {
                    self.changes.swap(read, write);
                }
                write += 1;
            }
        }
        drop(self.changes.drain(write..end));
    }

    fn edit_range(
        &mut self,
        offset: usize,
        length: usize,
        mut edit: impl FnMut(&mut HighlightingColor),
    ) -> Result<(), RichTextError> {
        let end = checked_end(offset, length)?;
        if length != 0 {
            let start_index = self.split_at(offset);
            let end_index = self.split_at(end);
            for (_, color) in &mut self.changes[start_index..end_index] {
                edit(color);
            }
            self.normalize_range(start_index.saturating_sub(1), end_index + 1);
        }
        Ok(())
    }

    pub fn get_highlighting_at(&self, offset: usize) -> &HighlightingColor {
        &self.changes[self.index_at(offset)].1
    }

    /// Overlays specified properties and preserves properties that are `None`.
    pub fn apply_highlighting(
        &mut self,
        offset: usize,
        length: usize,
        color: &HighlightingColor,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |existing| {
            *existing = existing.merge_with(color)
        })
    }

    /// Replaces every property in a range; `None` clears its formatting.
    pub fn set_highlighting(
        &mut self,
        offset: usize,
        length: usize,
        color: Option<&HighlightingColor>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |existing| {
            *existing = color.cloned().unwrap_or_default();
        })
    }

    pub fn set_foreground(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<&str>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| {
            color.foreground = value.map(str::to_owned)
        })
    }

    pub fn set_background(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<&str>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| {
            color.background = value.map(str::to_owned)
        })
    }

    pub fn set_bold(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<bool>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| {
            color.bold = value;
            color.font_weight = None;
        })
    }

    pub fn set_italic(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<bool>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| {
            color.italic = value;
            color.font_style = None;
        })
    }

    pub fn set_underline(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<bool>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| color.underline = value)
    }

    pub fn set_font_size(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<f64>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| color.font_size = value)
    }

    pub fn set_font_family(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<&str>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| {
            color.font_family = value.map(str::to_owned)
        })
    }

    pub fn set_font_weight(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<u16>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| {
            color.font_weight = value;
            color.bold = value.map(|weight| weight >= 700);
        })
    }

    pub fn set_font_style(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<HighlightingFontStyle>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| {
            color.font_style = value;
            color.italic = value.map(|style| style != HighlightingFontStyle::Normal);
        })
    }

    pub fn set_strikethrough(
        &mut self,
        offset: usize,
        length: usize,
        value: Option<bool>,
    ) -> Result<(), RichTextError> {
        self.edit_range(offset, length, |color| color.strikethrough = value)
    }

    /// Returns a complete partition of the requested range with absolute offsets.
    pub fn get_highlighted_sections(
        &self,
        offset: usize,
        length: usize,
    ) -> Result<Vec<HighlightedSection>, RichTextError> {
        let end = checked_end(offset, length)?;
        let mut index = self.index_at(offset);
        let mut position = offset;
        let mut sections = Vec::new();
        while position < end {
            let next = self
                .changes
                .get(index + 1)
                .map_or(end, |entry| entry.0.min(end));
            sections.push(HighlightedSection {
                offset: position,
                length: next - position,
                color: self.changes[index].1.clone(),
            });
            position = next;
            index += 1;
        }
        Ok(sections)
    }

    fn remap(
        &mut self,
        mut map: impl FnMut(usize) -> Result<usize, RichTextError>,
    ) -> Result<(), RichTextError> {
        // The first change always remains at zero. If later changes collapse
        // onto it, the last surviving format wins at the deletion boundary.
        let mut changes = vec![self.changes[0].clone()];
        for (offset, color) in self.changes.iter().skip(1) {
            let offset = map(*offset)?;
            let last = changes.last_mut().expect("initial change at zero");
            if offset < last.0 {
                return Err(DocumentError::InvalidOffsetMap.into());
            }
            if offset == last.0 {
                last.1 = color.clone();
            } else {
                changes.push((offset, color.clone()));
            }
        }
        self.changes = changes;
        self.normalize();
        Ok(())
    }

    /// Remaps formatting after an edit, using the map's default insertion affinity.
    pub fn update_offsets(&mut self, change: &OffsetChangeMap) -> Result<(), RichTextError> {
        for entry in change.entries() {
            checked_end(entry.offset, entry.removal_length)?;
            checked_end(entry.offset, entry.insertion_length)?;
        }
        self.remap(|offset| {
            change.entries().iter().try_fold(offset, |offset, entry| {
                let end = checked_end(entry.offset, entry.removal_length)?;
                if offset >= end && (entry.removal_length != 0 || offset != entry.offset) {
                    checked_end(offset - entry.removal_length, entry.insertion_length)?;
                }
                Ok(entry.get_new_offset(offset, AnchorMovementType::Default))
            })
        })
    }

    pub fn update_offsets_for_entry(
        &mut self,
        change: OffsetChangeMapEntry,
    ) -> Result<(), RichTextError> {
        self.update_offsets(&OffsetChangeMap::from_single_element(change))
    }

    pub fn update_offsets_for_document_change(
        &mut self,
        change: &DocumentChangeEventArgs,
    ) -> Result<(), RichTextError> {
        self.update_offsets(&change.offset_change_map)
    }

    pub fn update_offsets_for_text_change(
        &mut self,
        change: &TextChangeEventArgs,
    ) -> Result<(), RichTextError> {
        let end = checked_end(change.offset, change.removal_length())?;
        checked_end(change.offset, change.insertion_length())?;
        self.remap(|offset| {
            if offset > end {
                checked_end(offset - change.removal_length(), change.insertion_length())?;
            }
            Ok(change.get_new_offset(offset, AnchorMovementType::Default))
        })
    }

    fn clipped(&self, length: usize) -> Self {
        let mut changes: Vec<_> = self
            .changes
            .iter()
            .take_while(|(offset, _)| *offset < length)
            .cloned()
            .collect();
        if changes.is_empty() {
            changes.push((0, self.get_highlighting_at(0).clone()));
        }
        Self { changes }
    }
}

/// Immutable text with effective formatting, indexed in UTF-16 code units.
///
/// `text_utf16()` is lossless, including lone surrogates. `text()`, `Display`,
/// and HTML replace isolated surrogates with the Unicode replacement character.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RichText {
    text: Vec<u16>,
    model: RichTextModel,
}

impl RichText {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.encode_utf16().collect(),
            model: RichTextModel::new(),
        }
    }

    pub fn from_utf16(text: &[u16]) -> Self {
        Self {
            text: text.to_vec(),
            model: RichTextModel::new(),
        }
    }

    pub fn with_model(text: &str, model: &RichTextModel) -> Self {
        let text: Vec<_> = text.encode_utf16().collect();
        let model = model.clipped(text.len());
        Self { text, model }
    }

    pub fn from_utf16_with_model(text: &[u16], model: &RichTextModel) -> Self {
        Self {
            text: text.to_vec(),
            model: model.clipped(text.len()),
        }
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn text(&self) -> String {
        String::from_utf16_lossy(&self.text)
    }

    pub fn text_utf16(&self) -> &[u16] {
        &self.text
    }

    pub fn get_highlighting_at(&self, offset: usize) -> Result<&HighlightingColor, RichTextError> {
        check_range(offset, 0, self.len())?;
        Ok(self.model.get_highlighting_at(offset))
    }

    pub fn get_highlighted_sections(
        &self,
        offset: usize,
        length: usize,
    ) -> Result<Vec<HighlightedSection>, RichTextError> {
        check_range(offset, length, self.len())?;
        self.model.get_highlighted_sections(offset, length)
    }

    pub fn to_rich_text_model(&self) -> RichTextModel {
        self.model.clone()
    }

    pub fn substring(&self, offset: usize, length: usize) -> Result<Self, RichTextError> {
        let end = check_range(offset, length, self.len())?;
        let sections = self.model.get_highlighted_sections(offset, length)?;
        let mut changes: Vec<_> = sections
            .into_iter()
            .map(|section| (section.offset - offset, section.color))
            .collect();
        if changes.is_empty() {
            changes.push((0, self.model.get_highlighting_at(offset).clone()));
        }
        Ok(Self {
            text: self.text[offset..end].to_vec(),
            model: RichTextModel { changes },
        })
    }

    pub fn concat(texts: &[Self]) -> Result<Self, RichTextError> {
        let length = texts
            .iter()
            .try_fold(0, |length, text| checked_end(length, text.len()))?;
        let mut text = Vec::with_capacity(length);
        let mut changes = Vec::new();
        for item in texts.iter().filter(|item| !item.is_empty()) {
            let offset = text.len();
            changes.extend(
                item.model
                    .changes
                    .iter()
                    .map(|(start, color)| (offset + start, color.clone())),
            );
            text.extend_from_slice(&item.text);
        }
        if changes.is_empty() {
            return Ok(Self::default());
        }
        let mut model = RichTextModel { changes };
        model.normalize();
        Ok(Self { text, model })
    }

    pub fn write_to(
        &self,
        writer: &mut (impl RichTextWriter + ?Sized),
    ) -> Result<(), RichTextError> {
        writer.write_rich_text(self)
    }

    pub fn write_range_to(
        &self,
        writer: &mut (impl RichTextWriter + ?Sized),
        offset: usize,
        length: usize,
    ) -> Result<(), RichTextError> {
        writer.write_rich_text_range(self, offset, length)
    }

    pub fn to_html(&self, options: &HtmlOptions) -> Result<String, RichTextError> {
        let mut writer = super::HtmlRichTextWriter::new(options.clone())?;
        self.write_to(&mut writer)?;
        writer.finish()
    }

    pub fn to_html_range(
        &self,
        offset: usize,
        length: usize,
        options: &HtmlOptions,
    ) -> Result<String, RichTextError> {
        let mut writer = super::HtmlRichTextWriter::new(options.clone())?;
        self.write_range_to(&mut writer, offset, length)?;
        writer.finish()
    }
}

impl From<&str> for RichText {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for RichText {
    fn from(value: String) -> Self {
        Self::new(&value)
    }
}

impl fmt::Display for RichText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> HighlightingColor {
        HighlightingColor {
            foreground: Some("red".into()),
            bold: Some(true),
            ..Default::default()
        }
    }

    #[test]
    fn nested_formats_merge_only_specified_properties() {
        let mut model = RichTextModel::new();
        model.apply_highlighting(1, 6, &red()).unwrap();
        model
            .apply_highlighting(
                3,
                2,
                &HighlightingColor {
                    foreground: Some("blue".into()),
                    italic: Some(true),
                    bold: Some(false),
                    ..Default::default()
                },
            )
            .unwrap();
        let sections = model.get_highlighted_sections(0, 8).unwrap();
        assert_eq!(
            sections
                .iter()
                .map(|s| (s.offset, s.length))
                .collect::<Vec<_>>(),
            [(0, 1), (1, 2), (3, 2), (5, 2), (7, 1)]
        );
        assert_eq!(sections[2].color.foreground.as_deref(), Some("blue"));
        assert_eq!(sections[2].color.bold, Some(false));
        assert_eq!(sections[2].color.italic, Some(true));
        assert_eq!(sections[3].color, red());
        assert_eq!(sections[4].color, HighlightingColor::default());
    }

    #[test]
    fn semantic_font_values_and_compatibility_overlays_have_explicit_precedence() {
        let base = HighlightingColor {
            font_family: Some("Test Sans".into()),
            font_weight: Some(650),
            font_style: Some(HighlightingFontStyle::Oblique),
            strikethrough: Some(true),
            ..Default::default()
        };
        let inherited = base.merge_with(&HighlightingColor {
            foreground: Some("red".into()),
            ..Default::default()
        });
        assert_eq!(inherited.effective_font_weight(), Some(650));
        assert_eq!(
            inherited.effective_font_style(),
            Some(HighlightingFontStyle::Oblique)
        );
        let compatibility = base.merge_with(&HighlightingColor {
            bold: Some(false),
            italic: Some(false),
            ..Default::default()
        });
        assert_eq!(compatibility.font_weight, None);
        assert_eq!(compatibility.font_style, None);
        assert_eq!(compatibility.effective_font_weight(), Some(400));
        assert_eq!(
            compatibility.effective_font_style(),
            Some(HighlightingFontStyle::Normal)
        );
        assert_eq!(compatibility.font_family, base.font_family);
        assert_eq!(compatibility.strikethrough, Some(true));
        let semantic = compatibility.merge_with(&HighlightingColor {
            bold: Some(false),
            italic: Some(false),
            font_weight: Some(800),
            font_style: Some(HighlightingFontStyle::Oblique),
            ..Default::default()
        });
        assert_eq!(semantic.bold, Some(true));
        assert_eq!(semantic.italic, Some(true));
        assert_eq!(semantic.effective_font_weight(), Some(800));
        assert_eq!(
            semantic.effective_font_style(),
            Some(HighlightingFontStyle::Oblique)
        );
    }

    #[test]
    fn individual_font_setters_clear_related_compatibility_state_and_keep_unrelated_style() {
        let mut model = RichTextModel::new();
        model.set_font_family(1, 4, Some("Test Sans")).unwrap();
        model.set_font_weight(1, 4, Some(650)).unwrap();
        model
            .set_font_style(1, 4, Some(HighlightingFontStyle::Oblique))
            .unwrap();
        model.set_strikethrough(1, 4, Some(true)).unwrap();
        model.set_bold(2, 1, Some(true)).unwrap();
        model.set_italic(2, 1, Some(false)).unwrap();
        let inner = model.get_highlighting_at(2);
        assert_eq!(inner.font_weight, None);
        assert_eq!(inner.font_style, None);
        assert_eq!(inner.effective_font_weight(), Some(700));
        assert_eq!(
            inner.effective_font_style(),
            Some(HighlightingFontStyle::Normal)
        );
        assert_eq!(inner.font_family.as_deref(), Some("Test Sans"));
        assert_eq!(inner.strikethrough, Some(true));
        assert_eq!(model.get_highlighting_at(3).font_weight, Some(650));
        model.set_font_weight(2, 1, None).unwrap();
        model.set_font_style(2, 1, None).unwrap();
        model.set_strikethrough(2, 1, None).unwrap();
        let cleared = model.get_highlighting_at(2);
        assert_eq!(cleared.effective_font_weight(), None);
        assert_eq!(cleared.effective_font_style(), None);
        assert_eq!(cleared.strikethrough, None);
        assert_eq!(cleared.font_family.as_deref(), Some("Test Sans"));
    }

    #[test]
    fn replacement_and_individual_clearing_preserve_other_properties() {
        let mut model = RichTextModel::new();
        model.apply_highlighting(0, 6, &red()).unwrap();
        model.set_foreground(1, 2, None).unwrap();
        assert_eq!(model.get_highlighting_at(1).foreground, None);
        assert_eq!(model.get_highlighting_at(1).bold, Some(true));
        model.set_highlighting(2, 2, None).unwrap();
        assert_eq!(model.get_highlighting_at(2), &HighlightingColor::default());
        model.set_highlighting(0, 6, Some(&red())).unwrap();
        assert_eq!(model.get_highlighted_sections(0, 6).unwrap().len(), 1);
    }

    #[test]
    fn local_coalescing_joins_both_boundaries_without_changing_the_suffix() {
        let mut model = RichTextModel::new();
        model.apply_highlighting(2, 2, &red()).unwrap();
        model.apply_highlighting(6, 2, &red()).unwrap();
        model.set_background(10, 2, Some("blue")).unwrap();
        model.apply_highlighting(4, 2, &red()).unwrap();
        let sections = model.get_highlighted_sections(0, 14).unwrap();
        assert_eq!(
            sections
                .iter()
                .map(|section| (section.offset, section.length))
                .collect::<Vec<_>>(),
            [(0, 2), (2, 6), (8, 2), (10, 2), (12, 2)]
        );
        assert_eq!(sections[1].color, red());
        assert_eq!(sections[3].color.background.as_deref(), Some("blue"));
        model.set_highlighting(0, 10, None).unwrap();
        assert_eq!(model.get_highlighted_sections(0, 14).unwrap().len(), 3);
    }

    #[test]
    fn slices_and_concatenation_are_lossless_at_utf16_boundaries() {
        let units = [b'a' as u16, 0xd83d, 0xde00, 0xd800, b'z' as u16];
        let mut model = RichTextModel::new();
        model.apply_highlighting(1, 3, &red()).unwrap();
        let text = RichText::from_utf16_with_model(&units, &model);
        let a = text.substring(0, 2).unwrap();
        let b = text.substring(2, 3).unwrap();
        assert_eq!(a.text_utf16(), [b'a' as u16, 0xd83d]);
        assert_eq!(b.get_highlighting_at(0).unwrap(), &red());
        let concatenated = RichText::concat(&[a, RichText::default(), b]).unwrap();
        assert_eq!(concatenated, text);
        assert_eq!(concatenated.text(), "a😀�z");
        model.set_highlighting(0, 5, None).unwrap();
        assert_eq!(text.get_highlighting_at(1).unwrap(), &red());
    }

    #[test]
    fn all_ranges_include_empty_end_but_reject_overflow_and_out_of_bounds() {
        let text = RichText::new("😀");
        assert_eq!(text.len(), 2);
        assert!(text.substring(2, 0).unwrap().is_empty());
        assert!(text.get_highlighted_sections(2, 0).unwrap().is_empty());
        assert!(text.get_highlighting_at(2).is_ok());
        for (offset, length) in [(3, 0), (1, 2), (usize::MAX, 1), (1, usize::MAX)] {
            assert!(matches!(
                text.substring(offset, length),
                Err(RichTextError::InvalidRange { .. })
            ));
            assert!(text.get_highlighted_sections(offset, length).is_err());
        }
        let mut model = RichTextModel::new();
        assert!(model.apply_highlighting(usize::MAX, 1, &red()).is_err());
        assert!(model.get_highlighted_sections(usize::MAX, 1).is_err());
        assert_eq!(model, RichTextModel::new());
    }

    #[test]
    fn empty_models_and_zero_length_edits_are_well_defined() {
        let mut model = RichTextModel::new();
        model.apply_highlighting(0, 0, &red()).unwrap();
        model.set_background(5, 0, Some("blue")).unwrap();
        assert_eq!(model, RichTextModel::new());
        let text = RichText::with_model("", &model);
        assert!(text.is_empty());
        assert!(text.get_highlighting_at(0).is_ok());
        assert!(RichText::concat(&[]).unwrap().is_empty());
    }

    #[test]
    fn deletion_collapses_format_changes_and_insertion_uses_affinity() {
        let mut model = RichTextModel::new();
        model.apply_highlighting(2, 3, &red()).unwrap();
        model
            .update_offsets_for_entry(OffsetChangeMapEntry::new(2, 0, 2))
            .unwrap();
        assert_eq!(model.get_highlighting_at(3), &HighlightingColor::default());
        assert_eq!(model.get_highlighting_at(4), &red());
        model
            .update_offsets_for_entry(OffsetChangeMapEntry::new(0, 7, 0))
            .unwrap();
        assert_eq!(model, RichTextModel::new());

        let mut model = RichTextModel::new();
        model.apply_highlighting(2, 2, &red()).unwrap();
        model
            .update_offsets_for_entry(OffsetChangeMapEntry {
                default_anchor_movement_is_before_insertion: true,
                ..OffsetChangeMapEntry::new(2, 0, 1)
            })
            .unwrap();
        assert_eq!(model.get_highlighting_at(2), &red());
        assert_eq!(model.get_highlighting_at(4), &red());
        assert_eq!(model.get_highlighting_at(5), &HighlightingColor::default());
    }

    #[test]
    fn failed_offset_update_does_not_modify_model() {
        let mut model = RichTextModel::new();
        model.apply_highlighting(5, 2, &red()).unwrap();
        let original = model.clone();
        assert!(model
            .update_offsets_for_entry(OffsetChangeMapEntry::new(1, 0, usize::MAX))
            .is_err());
        assert_eq!(model, original);
    }

    #[test]
    fn text_change_boundary_affinity_differs_from_document_replacement() {
        let mut model = RichTextModel::new();
        model.apply_highlighting(2, 2, &red()).unwrap();
        let mut document_model = model.clone();
        model
            .update_offsets_for_text_change(&TextChangeEventArgs::new(2, "xx", "q"))
            .unwrap();
        document_model
            .update_offsets_for_document_change(&DocumentChangeEventArgs::new(2, "xx", "q"))
            .unwrap();
        assert_eq!(model.get_highlighting_at(2), &HighlightingColor::default());
        assert_eq!(document_model.get_highlighting_at(2), &red());
    }
}
