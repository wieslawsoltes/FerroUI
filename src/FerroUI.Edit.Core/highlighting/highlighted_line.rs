use super::{
    HighlightingColor, HighlightingState, HtmlOptions, RichText, RichTextError, RichTextModel,
    RichTextWriter,
};
use crate::document::{DocumentLine, ISegment, ITextSource};

#[derive(Clone, Debug, PartialEq)]
pub struct HighlightedSection {
    pub offset: usize,
    pub length: usize,
    pub color: HighlightingColor,
}

impl ISegment for HighlightedSection {
    fn offset(&self) -> usize {
        self.offset
    }

    fn length(&self) -> usize {
        self.length
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HighlightedLine {
    pub line_number: usize,
    pub sections: Vec<HighlightedSection>,
    pub end_state: HighlightingState,
}

impl HighlightedLine {
    /// Checks absolute UTF-16 bounds and the ordered, properly nested section tree.
    pub fn validate_invariants(
        &self,
        document: &(impl ITextSource + ?Sized),
    ) -> Result<(), RichTextError> {
        self.validate_for_line(
            document
                .create_snapshot()
                .get_line_by_number(self.line_number)?,
        )
    }

    fn validate_for_line(&self, line: DocumentLine) -> Result<(), RichTextError> {
        let mut ends = Vec::new();
        let mut previous_offset = line.offset;
        for (index, section) in self.sections.iter().enumerate() {
            let end = section.offset.checked_add(section.length).ok_or(
                RichTextError::InvalidSections {
                    index,
                    reason: "range overflows",
                },
            )?;
            if section.offset < line.offset || end > line.end_offset() {
                return Err(RichTextError::InvalidSections {
                    index,
                    reason: "section is outside the document line",
                });
            }
            if section.offset < previous_offset {
                return Err(RichTextError::InvalidSections {
                    index,
                    reason: "sections are not sorted by offset",
                });
            }
            while ends.last().is_some_and(|end| *end <= section.offset) {
                ends.pop();
            }
            if ends.last().is_some_and(|parent_end| end > *parent_end) {
                return Err(RichTextError::InvalidSections {
                    index,
                    reason: "sections overlap without proper nesting",
                });
            }
            if section.length != 0 {
                ends.push(end);
            }
            previous_offset = section.offset;
        }
        Ok(())
    }

    /// Applies an additional line's sections after this line's formats, splitting
    /// them at existing boundaries so the ordered nesting invariant is retained.
    /// The syntax engine's `end_state` remains associated with the base line.
    pub fn merge_with(
        &mut self,
        additional: &Self,
        document: &(impl ITextSource + ?Sized),
    ) -> Result<(), RichTextError> {
        if self.line_number != additional.line_number {
            return Err(RichTextError::DifferentLine);
        }
        let line = document
            .create_snapshot()
            .get_line_by_number(self.line_number)?;
        self.validate_for_line(line)?;
        additional.validate_for_line(line)?;
        let mut merged = self.clone();
        for section in additional
            .sections
            .iter()
            .filter(|section| section.length != 0)
        {
            let end = section.offset + section.length;
            let mut boundaries = vec![section.offset, end];
            for existing in &merged.sections {
                for boundary in [existing.offset, existing.offset + existing.length] {
                    if boundary > section.offset && boundary < end {
                        boundaries.push(boundary);
                    }
                }
            }
            boundaries.sort_unstable();
            boundaries.dedup();
            for range in boundaries.windows(2) {
                merged.sections.push(HighlightedSection {
                    offset: range[0],
                    length: range[1] - range[0],
                    color: section.color.clone(),
                });
            }
            // Stable sorting keeps previous equal-sized spans outside new ones.
            merged.sections.sort_by(|left, right| {
                left.offset
                    .cmp(&right.offset)
                    .then_with(|| right.length.cmp(&left.length))
            });
        }
        merged.validate_for_line(line)?;
        self.sections = merged.sections;
        Ok(())
    }

    fn model_for_line(&self, line: DocumentLine) -> Result<RichTextModel, RichTextError> {
        self.validate_for_line(line)?;
        let mut model = RichTextModel::new();
        for section in &self.sections {
            model.apply_highlighting(
                section.offset - line.offset,
                section.length,
                &section.color,
            )?;
        }
        Ok(model)
    }

    pub fn to_rich_text_model(
        &self,
        document: &(impl ITextSource + ?Sized),
    ) -> Result<RichTextModel, RichTextError> {
        self.model_for_line(
            document
                .create_snapshot()
                .get_line_by_number(self.line_number)?,
        )
    }

    /// Copies line text and formatting, excluding its line delimiter.
    pub fn to_rich_text(
        &self,
        document: &(impl ITextSource + ?Sized),
    ) -> Result<RichText, RichTextError> {
        let snapshot = document.create_snapshot();
        let line = snapshot.get_line_by_number(self.line_number)?;
        let model = self.model_for_line(line)?;
        Ok(RichText::from_utf16_with_model(
            &snapshot.get_text_utf16(line.offset, line.length)?,
            &model,
        ))
    }

    /// Copies the absolute half-open document range `start_offset..end_offset`.
    pub fn to_rich_text_range(
        &self,
        document: &(impl ITextSource + ?Sized),
        start_offset: usize,
        end_offset: usize,
    ) -> Result<RichText, RichTextError> {
        let snapshot = document.create_snapshot();
        let line = snapshot.get_line_by_number(self.line_number)?;
        if start_offset < line.offset || end_offset < start_offset || end_offset > line.end_offset()
        {
            return Err(RichTextError::InvalidLineRange {
                start: start_offset,
                end: end_offset,
                line_start: line.offset,
                line_end: line.end_offset(),
            });
        }
        self.to_rich_text(&snapshot)?
            .substring(start_offset - line.offset, end_offset - start_offset)
    }

    pub fn write_to(
        &self,
        document: &(impl ITextSource + ?Sized),
        writer: &mut (impl RichTextWriter + ?Sized),
    ) -> Result<(), RichTextError> {
        self.to_rich_text(document)?.write_to(writer)
    }

    /// Writes the absolute half-open document range `start_offset..end_offset`.
    pub fn write_range_to(
        &self,
        document: &(impl ITextSource + ?Sized),
        writer: &mut (impl RichTextWriter + ?Sized),
        start_offset: usize,
        end_offset: usize,
    ) -> Result<(), RichTextError> {
        self.to_rich_text_range(document, start_offset, end_offset)?
            .write_to(writer)
    }

    pub fn to_html(
        &self,
        document: &(impl ITextSource + ?Sized),
        options: &HtmlOptions,
    ) -> Result<String, RichTextError> {
        self.to_rich_text(document)?.to_html(options)
    }

    /// Exports the absolute half-open document range `start_offset..end_offset`.
    pub fn to_html_range(
        &self,
        document: &(impl ITextSource + ?Sized),
        start_offset: usize,
        end_offset: usize,
        options: &HtmlOptions,
    ) -> Result<String, RichTextError> {
        self.to_rich_text_range(document, start_offset, end_offset)?
            .to_html(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::TextDocument;
    use crate::highlighting::PlainRichTextWriter;

    fn section(start: usize, end: usize, name: &str) -> HighlightedSection {
        HighlightedSection {
            offset: start,
            length: end - start,
            color: HighlightingColor {
                name: Some(name.into()),
                ..Default::default()
            },
        }
    }

    fn line(sections: Vec<HighlightedSection>) -> HighlightedLine {
        HighlightedLine {
            line_number: 1,
            sections,
            end_state: HighlightingState::default(),
        }
    }

    #[test]
    fn merges_additional_sections_at_existing_tree_boundaries() {
        let document = TextDocument::from_text("                    ");
        for nested in [false, true] {
            let mut base = line(vec![section(0, 1, "B")]);
            if nested {
                base.sections.push(section(0, 1, "BN"));
            }
            base.merge_with(&line(vec![section(0, 2, "A")]), &document)
                .unwrap();
            let mut expected = vec![section(0, 1, "B")];
            if nested {
                expected.push(section(0, 1, "BN"));
            }
            expected.extend([section(0, 1, "A"), section(1, 2, "A")]);
            assert_eq!(base.sections, expected);
            base.validate_invariants(&document).unwrap();
        }
    }

    #[test]
    fn merging_all_small_ranges_matches_effective_overlay() {
        let document = TextDocument::from_text("abcdef");
        for start in 0..=6 {
            for end in start..=6 {
                for other_start in 0..=6 {
                    for other_end in other_start..=6 {
                        let mut base = line(vec![HighlightedSection {
                            offset: start,
                            length: end - start,
                            color: HighlightingColor {
                                foreground: Some("red".into()),
                                bold: Some(true),
                                ..Default::default()
                            },
                        }]);
                        let additional = line(vec![HighlightedSection {
                            offset: other_start,
                            length: other_end - other_start,
                            color: HighlightingColor {
                                foreground: Some("blue".into()),
                                italic: Some(true),
                                ..Default::default()
                            },
                        }]);
                        let mut expected = base.to_rich_text_model(&document).unwrap();
                        expected
                            .apply_highlighting(
                                other_start,
                                other_end - other_start,
                                &additional.sections[0].color,
                            )
                            .unwrap();
                        base.merge_with(&additional, &document).unwrap();
                        assert_eq!(base.to_rich_text_model(&document).unwrap(), expected);
                    }
                }
            }
        }
    }

    #[test]
    fn invalid_crossing_unsorted_and_out_of_bounds_sections_are_rejected() {
        let document = TextDocument::from_text("abcdef");
        for sections in [
            vec![section(0, 3, "a"), section(2, 4, "b")],
            vec![section(2, 3, "a"), section(0, 1, "b")],
            vec![section(0, 7, "a")],
            vec![section(0, 1, "a"), section(0, 2, "b")],
            vec![HighlightedSection {
                offset: usize::MAX,
                length: 1,
                color: HighlightingColor::default(),
            }],
        ] {
            let highlighted = line(sections);
            assert!(highlighted.validate_invariants(&document).is_err());
            assert!(highlighted.to_rich_text(&document).is_err());
        }
        let mut base = line(vec![section(0, 2, "a")]);
        let original = base.clone();
        assert!(
            base.merge_with(&line(vec![section(0, 7, "bad")]), &document)
                .is_err()
        );
        assert_eq!(base, original);
    }

    #[test]
    fn line_conversion_uses_absolute_utf16_offsets_and_excludes_delimiter() {
        let document = TextDocument::from_utf16(&[
            b'x' as u16,
            13,
            10,
            b'a' as u16,
            0xd83d,
            0xde00,
            0xd800,
            13,
            10,
        ]);
        let highlighted = HighlightedLine {
            line_number: 2,
            sections: vec![HighlightedSection {
                offset: 4,
                length: 3,
                color: HighlightingColor {
                    foreground: Some("red".into()),
                    ..Default::default()
                },
            }],
            end_state: HighlightingState::default(),
        };
        let text = highlighted.to_rich_text(&document).unwrap();
        assert_eq!(text.text_utf16(), [b'a' as u16, 0xd83d, 0xde00, 0xd800]);
        assert_eq!(
            text.get_highlighting_at(1).unwrap().foreground.as_deref(),
            Some("red")
        );
        let mut writer = PlainRichTextWriter::new();
        highlighted
            .write_range_to(&document, &mut writer, 4, 6)
            .unwrap();
        assert_eq!(writer.finish_string().unwrap(), "😀");
        assert_eq!(
            highlighted
                .to_html_range(&document, 4, 6, &HtmlOptions::default())
                .unwrap(),
            "<span style=\"color: red; \">😀</span>"
        );
        for (start, end) in [(0, 1), (2, 4), (5, 4), (7, 8), (usize::MAX, usize::MAX)] {
            assert!(
                highlighted
                    .to_rich_text_range(&document, start, end)
                    .is_err()
            );
        }
        assert!(
            highlighted
                .to_rich_text_range(&document, 7, 7)
                .unwrap()
                .is_empty()
        );
    }
}
