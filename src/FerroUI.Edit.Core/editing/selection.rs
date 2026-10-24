use crate::document::{AnchorMovementType, DocumentChangeEventArgs, SimpleSegment};

/// A directed UTF-16 selection. `anchor` stays fixed while `caret` moves.
/// Virtual spaces count cells after the logical line's end; they never count
/// UTF-16 units or occupy document text. A session validates their placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectionSegment {
    pub anchor: usize,
    pub caret: usize,
    pub anchor_virtual_space: usize,
    pub caret_virtual_space: usize,
}

impl SelectionSegment {
    pub const fn new(anchor: usize, caret: usize) -> Self {
        Self::with_virtual_space(anchor, 0, caret, 0)
    }
    pub const fn with_virtual_space(
        anchor: usize,
        anchor_virtual_space: usize,
        caret: usize,
        caret_virtual_space: usize,
    ) -> Self {
        Self {
            anchor,
            caret,
            anchor_virtual_space,
            caret_virtual_space,
        }
    }
    pub const fn anchor_tuple(self) -> (usize, usize) {
        (self.anchor, self.anchor_virtual_space)
    }
    pub const fn caret_tuple(self) -> (usize, usize) {
        (self.caret, self.caret_virtual_space)
    }
    pub fn start_tuple(self) -> (usize, usize) {
        self.anchor_tuple().min(self.caret_tuple())
    }
    pub fn end_tuple(self) -> (usize, usize) {
        self.anchor_tuple().max(self.caret_tuple())
    }
    pub fn start_virtual_space(self) -> usize {
        self.start_tuple().1
    }
    pub fn end_virtual_space(self) -> usize {
        self.end_tuple().1
    }
    pub fn is_reversed(self) -> bool {
        self.anchor_tuple() > self.caret_tuple()
    }
    pub fn contains_position(self, offset: usize, virtual_space: usize) -> bool {
        self.start_tuple() <= (offset, virtual_space) && (offset, virtual_space) <= self.end_tuple()
    }
    pub(crate) fn from_tuples(anchor: (usize, usize), caret: (usize, usize)) -> Self {
        Self::with_virtual_space(anchor.0, anchor.1, caret.0, caret.1)
    }
    pub const fn empty(offset: usize) -> Self {
        Self::new(offset, offset)
    }
    pub fn start_offset(self) -> usize {
        self.anchor.min(self.caret)
    }
    pub fn end_offset(self) -> usize {
        self.anchor.max(self.caret)
    }
    pub fn length(self) -> usize {
        self.end_offset() - self.start_offset()
    }
    pub fn is_empty(self) -> bool {
        self.anchor_tuple() == self.caret_tuple()
    }
    pub fn contains(self, offset: usize) -> bool {
        self.start_offset() <= offset && offset <= self.end_offset()
    }
    pub fn segment(self) -> SimpleSegment {
        SimpleSegment::new(self.start_offset(), self.length())
    }

    pub fn update_on_document_change(self, change: &DocumentChangeEventArgs) -> Self {
        if self.is_empty() {
            let offset = change.get_new_offset(self.caret, AnchorMovementType::AfterInsertion);
            return Self::with_virtual_space(
                offset,
                self.caret_virtual_space,
                offset,
                self.caret_virtual_space,
            );
        }
        let start = self.start_tuple();
        let end = self.end_tuple();
        let start = (
            change.get_new_offset(
                start.0,
                if start.1 == 0 {
                    AnchorMovementType::BeforeInsertion
                } else {
                    AnchorMovementType::AfterInsertion
                },
            ),
            start.1,
        );
        let end = (
            change.get_new_offset(end.0, AnchorMovementType::AfterInsertion),
            end.1,
        );
        if self.is_reversed() {
            Self::from_tuples(end, start)
        } else {
            Self::from_tuples(start, end)
        }
    }
}

/// A nonempty, ordered collection of nonoverlapping selections.
/// Overlapping ranges and duplicate carets are coalesced before editing, so a
/// character is never replaced twice by a multicaret operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    segments: Vec<SelectionSegment>,
    primary: usize,
    rectangular: bool,
}

impl Default for Selection {
    fn default() -> Self {
        Self::empty(0)
    }
}

impl Selection {
    pub fn empty(offset: usize) -> Self {
        Self {
            segments: vec![SelectionSegment::empty(offset)],
            primary: 0,
            rectangular: false,
        }
    }
    pub fn simple(anchor: usize, caret: usize) -> Self {
        Self {
            segments: vec![SelectionSegment::new(anchor, caret)],
            primary: 0,
            rectangular: false,
        }
    }

    pub fn from_segments(
        segments: impl IntoIterator<Item = SelectionSegment>,
        primary: usize,
    ) -> Self {
        Self::from_segments_with_mode(segments, primary, false)
    }

    /// Constructs a column selection. Its mode survives document changes and
    /// single-line replacements, including a rectangle of virtual carets.
    pub fn from_rectangle_segments(
        segments: impl IntoIterator<Item = SelectionSegment>,
        primary: usize,
    ) -> Self {
        Self::from_segments_with_mode(segments, primary, true)
    }

    pub(crate) fn from_segments_with_mode(
        segments: impl IntoIterator<Item = SelectionSegment>,
        primary: usize,
        rectangular: bool,
    ) -> Self {
        let mut input: Vec<_> = segments.into_iter().collect();
        if input.is_empty() {
            let mut empty = Self::default();
            empty.rectangular = rectangular;
            return empty;
        }
        let primary_segment = input[primary.min(input.len() - 1)];
        input.sort_by_key(|s| (s.start_tuple(), s.end_tuple()));
        let mut output: Vec<SelectionSegment> = Vec::with_capacity(input.len());
        for s in input {
            if let Some(last) = output.last_mut() {
                // Adjacent nonempty selections stay separate; a caret at a
                // selection boundary is covered by that selection.
                if s.start_tuple() < last.end_tuple()
                    || (s.start_tuple() == last.end_tuple() && (s.is_empty() || last.is_empty()))
                {
                    let start = last.start_tuple();
                    let end = last.end_tuple().max(s.end_tuple());
                    let backwards = last.is_reversed() || s == primary_segment && s.is_reversed();
                    *last = if backwards {
                        SelectionSegment::from_tuples(end, start)
                    } else {
                        SelectionSegment::from_tuples(start, end)
                    };
                    continue;
                }
            }
            output.push(s);
        }
        let primary = output
            .iter()
            .position(|s| {
                s.contains_position(primary_segment.caret, primary_segment.caret_virtual_space)
            })
            .unwrap_or(0);
        Self {
            segments: output,
            primary,
            rectangular,
        }
    }

    pub fn is_rectangular(&self) -> bool {
        self.rectangular
    }
    pub fn segments(&self) -> &[SelectionSegment] {
        &self.segments
    }
    pub fn primary_index(&self) -> usize {
        self.primary
    }
    pub fn primary(&self) -> SelectionSegment {
        self.segments[self.primary]
    }
    pub fn is_empty(&self) -> bool {
        self.segments.iter().all(|s| s.is_empty())
    }
    pub fn length(&self) -> usize {
        self.segments.iter().map(|s| s.length()).sum()
    }
    pub fn surrounding_segment(&self) -> SimpleSegment {
        let start = self.segments[0].start_offset();
        SimpleSegment::new(start, self.segments.last().unwrap().end_offset() - start)
    }
    pub fn update_on_document_change(&self, change: &DocumentChangeEventArgs) -> Self {
        Self::from_segments_with_mode(
            self.segments
                .iter()
                .map(|s| s.update_on_document_change(change)),
            self.primary,
            self.rectangular,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coalesces_overlap_and_duplicate_carets_but_preserves_adjacent_ranges() {
        let selection = Selection::from_segments(
            [
                SelectionSegment::new(8, 3),
                SelectionSegment::new(4, 9),
                SelectionSegment::empty(4),
                SelectionSegment::new(9, 12),
                SelectionSegment::empty(20),
                SelectionSegment::empty(20),
            ],
            0,
        );
        assert_eq!(
            selection.segments(),
            &[
                SelectionSegment::new(9, 3),
                SelectionSegment::new(9, 12),
                SelectionSegment::empty(20)
            ]
        );
        assert_eq!(selection.primary().caret, 3);
    }
}
