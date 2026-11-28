use super::{caret_navigation::line_ranges, Selection, SelectionSegment};
use crate::document::{DocumentError, TextDocument};
use unicode_segmentation::UnicodeSegmentation;

/// Rectangular selection expressed in zero-based lines and visual columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RectangleSelection {
    pub anchor_line: usize,
    pub anchor_column: usize,
    pub caret_line: usize,
    pub caret_column: usize,
}

impl RectangleSelection {
    pub fn to_selection(self, text: &str, tab_size: usize) -> Selection {
        let lines = line_ranges(text);
        let units: Vec<_> = text.encode_utf16().collect();
        let first = self.anchor_line.min(self.caret_line).min(lines.len() - 1);
        let last = self.anchor_line.max(self.caret_line).min(lines.len() - 1);
        let segments = (first..=last).map(|line| {
            let (start, end, _) = lines[line];
            let content = &units[start..end];
            SelectionSegment::new(
                start + offset_at_column(content, self.anchor_column, tab_size),
                start + offset_at_column(content, self.caret_column, tab_size),
            )
        });
        Selection::from_rectangle_segments(
            segments,
            if self.caret_line < self.anchor_line {
                0
            } else {
                last - first
            },
        )
    }

    /// Projects zero-based line and cell coordinates without flattening the
    /// document. Short lines retain cells beyond their ends as virtual spaces.
    /// Tabs advance to stops and extended grapheme clusters occupy one cell.
    /// Native views may instead project pixel positions through their layout.
    pub fn to_document_selection(
        self,
        document: &TextDocument,
        tab_size: usize,
    ) -> Result<Selection, DocumentError> {
        let first = self
            .anchor_line
            .min(self.caret_line)
            .min(document.line_count() - 1);
        let last = self
            .anchor_line
            .max(self.caret_line)
            .min(document.line_count() - 1);
        let mut segments = Vec::with_capacity(last - first + 1);
        for number in first..=last {
            let line = document.get_line_by_number(number + 1)?;
            let text = document.get_text(line.offset, line.length)?;
            let anchor = position_at_column(&text, self.anchor_column, tab_size);
            let caret = position_at_column(&text, self.caret_column, tab_size);
            segments.push(SelectionSegment::with_virtual_space(
                line.offset + anchor.0,
                anchor.1,
                line.offset + caret.0,
                caret.1,
            ));
        }
        Ok(Selection::from_rectangle_segments(
            segments,
            if self.caret_line < self.anchor_line {
                0
            } else {
                last - first
            },
        ))
    }

    /// String-based equivalent of `to_document_selection`. The original
    /// `to_selection` method deliberately keeps its clamped endpoint contract.
    pub fn to_selection_with_virtual_space(self, text: &str, tab_size: usize) -> Selection {
        self.to_document_selection(&TextDocument::from_text(text), tab_size)
            .expect("clamped line numbers and complete logical lines are valid")
    }
}

pub(crate) fn position_at_column(text: &str, column: usize, tab_size: usize) -> (usize, usize) {
    let tab_size = tab_size.max(1);
    let (mut visual, mut offset) = (0, 0);
    for grapheme in text.graphemes(true) {
        if visual >= column {
            return (offset, 0);
        }
        visual += if grapheme == "\t" {
            tab_size - visual % tab_size
        } else {
            1
        };
        offset += grapheme.encode_utf16().count();
    }
    (offset, column.saturating_sub(visual))
}

pub(crate) fn visual_column(text: &str, tab_size: usize) -> usize {
    let tab_size = tab_size.max(1);
    text.graphemes(true).fold(0, |column, grapheme| {
        column
            + if grapheme == "\t" {
                tab_size - column % tab_size
            } else {
                1
            }
    })
}

pub(crate) fn offset_at_column(text: &[u16], column: usize, tab_size: usize) -> usize {
    let tab_size = tab_size.max(1);
    let (mut visual, mut offset) = (0, 0);
    while offset < text.len() && visual < column {
        if text[offset] == 9 {
            visual += tab_size - visual % tab_size;
            offset += 1;
        } else {
            visual += 1;
            let surrogate = (0xD800..=0xDBFF).contains(&text[offset])
                && text
                    .get(offset + 1)
                    .is_some_and(|c| (0xDC00..=0xDFFF).contains(c));
            offset += if surrogate { 2 } else { 1 };
        }
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tabs_short_lines_and_surrogates() {
        let selection = RectangleSelection {
            anchor_line: 0,
            anchor_column: 4,
            caret_line: 2,
            caret_column: 6,
        }
        .to_selection("\tabc\n😀abc\nx", 4);
        assert_eq!(
            selection.segments(),
            &[
                SelectionSegment::new(1, 3),
                SelectionSegment::empty(10),
                SelectionSegment::empty(12)
            ]
        );
    }
}
