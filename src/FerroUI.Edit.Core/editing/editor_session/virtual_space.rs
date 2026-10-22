use super::*;
use crate::document::DocumentEdit;
use crate::editing::rectangle_selection::{position_at_column, visual_column};

pub(super) struct PlannedReplacement {
    pub segment: SimpleSegment,
    pub text: String,
    pub virtual_space: usize,
}

pub(super) fn normalize_selection(document: &TextDocument, selection: Selection) -> Selection {
    Selection::from_segments_with_mode(
        selection.segments().iter().map(|segment| {
            let normalize = |offset: usize, virtual_space: usize| {
                let offset = offset.min(document.text_length());
                let virtual_space = if virtual_space != 0
                    && document
                        .get_line_by_offset(offset)
                        .is_ok_and(|line| line.end_offset() == offset)
                {
                    virtual_space
                } else {
                    0
                };
                (offset, virtual_space)
            };
            SelectionSegment::from_tuples(
                normalize(segment.anchor, segment.anchor_virtual_space),
                normalize(segment.caret, segment.caret_virtual_space),
            )
        }),
        selection.primary_index(),
        selection.is_rectangular(),
    )
}

impl EditorSession {
    pub(super) fn validate_selection(&self, selection: &Selection) -> Result<(), EditError> {
        for segment in selection.segments() {
            for (offset, virtual_space) in [segment.anchor_tuple(), segment.caret_tuple()] {
                if offset > self.document().text_length()
                    || virtual_space != 0
                        && self.document().get_line_by_offset(offset)?.end_offset() != offset
                {
                    return Err(EditError::InvalidSelection);
                }
            }
        }
        Ok(())
    }

    /// Two distinct virtual positions can share one UTF-16 offset. Reject that
    /// ambiguous physical plan before mutation instead of ordering inserts by
    /// accident and putting one caret's text at another caret's column.
    pub(super) fn validate_virtual_edit_positions(
        &self,
        selection: &Selection,
    ) -> Result<(), EditError> {
        self.validate_selection(selection)?;
        if selection.segments().windows(2).any(|pair| {
            pair[0].end_offset() == pair[1].start_offset()
                && (pair[0].end_virtual_space() != 0 || pair[1].start_virtual_space() != 0)
        }) {
            return Err(EditError::AmbiguousVirtualCarets);
        }
        Ok(())
    }

    pub(super) fn replace_virtual_selections(
        &self,
        replacements: &[String],
    ) -> Result<(), EditError> {
        let selection = self.selection();
        if replacements.len() != 1 && replacements.len() != selection.segments().len() {
            return Err(EditError::ReplacementCount);
        }
        self.validate_virtual_edit_positions(&selection)?;
        let multiline_rectangle = selection.is_rectangular()
            && replacements.iter().any(|text| text.contains(['\r', '\n']));
        let replacements: Vec<&str> = if multiline_rectangle && replacements.len() == 1 {
            split_rectangular_text(&replacements[0], selection.segments().len())
        } else if replacements.len() == 1 {
            vec![replacements[0].as_str(); selection.segments().len()]
        } else {
            replacements.iter().map(String::as_str).collect()
        };
        let mut edits = Vec::with_capacity(replacements.len());
        for (segment, replacement) in selection.segments().iter().zip(replacements) {
            let text = self.with_virtual_padding(*segment, replacement)?;
            edits.push(PlannedReplacement {
                segment: segment.segment(),
                virtual_space: if text.is_empty() {
                    segment.start_virtual_space()
                } else {
                    0
                },
                text,
            });
        }
        self.execute_replacements(
            edits,
            selection.primary_index(),
            selection.is_rectangular() && !multiline_rectangle,
            multiline_rectangle,
        )
    }

    pub(super) fn with_virtual_padding(
        &self,
        segment: SelectionSegment,
        text: &str,
    ) -> Result<String, EditError> {
        let spaces = segment.start_virtual_space();
        if spaces == 0 || text.is_empty() || matches!(text, "\n" | "\r" | "\r\n") {
            return Ok(text.to_owned());
        }
        let options = self.options();
        let line = self.document().get_line_by_offset(segment.start_offset())?;
        let line_text = self.document().get_text(line.offset, line.length)?;
        let tabs = if !options.convert_tabs_to_spaces && line_text.chars().all(|c| c == '\t') {
            spaces / options.indentation_size
        } else {
            0
        };
        let spaces = spaces - tabs * options.indentation_size;
        let capacity = tabs
            .checked_add(spaces)
            .and_then(|length| length.checked_add(text.len()))
            .ok_or(EditError::VirtualSpaceTooLarge)?;
        let mut padded = String::new();
        padded
            .try_reserve_exact(capacity)
            .map_err(|_| EditError::VirtualSpaceTooLarge)?;
        padded.extend(std::iter::repeat_n('\t', tabs));
        padded.extend(std::iter::repeat_n(' ', spaces));
        padded.push_str(text);
        Ok(padded)
    }

    pub(super) fn execute_replacements(
        &self,
        edits: Vec<PlannedReplacement>,
        primary: usize,
        rectangular: bool,
        collapse_rectangle: bool,
    ) -> Result<(), EditError> {
        let original_version = self.document().version();
        let document_edits: Vec<_> = edits
            .iter()
            .filter(|edit| edit.segment.length != 0 || !edit.text.is_empty())
            .map(|edit| DocumentEdit::new(edit.segment.offset, edit.segment.length, &edit.text))
            .collect();
        if edits
            .windows(2)
            .any(|pair| pair[0].segment.end_offset() > pair[1].segment.offset)
        {
            return Err(DocumentError::OverlappingEdits.into());
        }
        let bottom_line = if collapse_rectangle {
            self.document()
                .get_line_by_offset(self.selection().segments().last().unwrap().end_offset())?
                .line_number
        } else {
            1
        };
        let update = self.begin_prepared_update(&document_edits, &original_version)?;
        let carets: Result<Vec<_>, DocumentError> = edits
            .iter()
            .map(|edit| {
                let anchor = self.document().create_anchor(edit.segment.offset)?;
                anchor.set_movement_type(AnchorMovementType::BeforeInsertion);
                anchor.set_survive_deletion(true);
                Ok(anchor)
            })
            .collect();
        let carets = carets?;
        if !document_edits.is_empty() {
            self.document().apply_edits(&document_edits)?;
        }
        let segments: Result<Vec<_>, DocumentError> = carets
            .iter()
            .zip(&edits)
            .map(|(anchor, edit)| {
                let offset = anchor.offset()? + edit.text.encode_utf16().count();
                Ok(SelectionSegment::with_virtual_space(
                    offset,
                    edit.virtual_space,
                    offset,
                    edit.virtual_space,
                ))
            })
            .collect();
        let segments = segments?;
        if collapse_rectangle {
            // Multiline rectangular paste ends column mode. The first inserted
            // row determines the UTF-16 column on the rectangle's bottom row.
            let first = segments.first().copied().unwrap_or_default();
            let column = self.document().get_location(first.caret)?.column - 1;
            let line = self
                .document()
                .get_line_by_number(bottom_line.min(self.document().line_count()))?;
            self.set_caret_offset(line.offset + column.min(line.length))?;
        } else {
            let selection = Selection::from_segments_with_mode(segments, primary, rectangular);
            self.set_selection(normalize_selection(self.document(), selection))?;
        }
        if let Some(update) = update {
            update.finish()?;
        }
        Ok(())
    }

    fn visual_position(&self, offset: usize, virtual_space: usize) -> Result<usize, DocumentError> {
        let line = self.document().get_line_by_offset(offset)?;
        let prefix = self
            .document()
            .get_text(line.offset, (offset - line.offset).min(line.length))?;
        Ok(visual_column(&prefix, self.options().indentation_size).saturating_add(virtual_space))
    }

    pub(super) fn move_caret_with_virtual_space(&self, movement: CaretMovement, extend: bool) {
        let selection = self.selection();
        let options = self.options();
        let allow_virtual = options.enable_virtual_space || selection.is_rectangular();
        let vertical = matches!(movement, CaretMovement::LineUp | CaretMovement::LineDown);
        let old_columns = self.inner.desired_columns.borrow().clone();
        let mut columns = Vec::with_capacity(selection.segments().len());
        let mut segments = Vec::with_capacity(selection.segments().len());
        for (index, segment) in selection.segments().iter().enumerate() {
            let desired = if vertical {
                old_columns.get(index).copied().flatten().or_else(|| {
                    self.visual_position(segment.caret, segment.caret_virtual_space)
                        .ok()
                })
            } else {
                None
            };
            columns.push(desired);
            let target = if !extend
                && !segment.is_empty()
                && movement == CaretMovement::CharacterLeft
            {
                segment.start_tuple()
            } else if !extend && !segment.is_empty() && movement == CaretMovement::CharacterRight {
                segment.end_tuple()
            } else {
                self.move_virtual_position(segment.caret_tuple(), movement, allow_virtual, desired)
            };
            segments.push(SelectionSegment::from_tuples(
                if extend {
                    segment.anchor_tuple()
                } else {
                    target
                },
                target,
            ));
        }
        *self.inner.selection.borrow_mut() = Selection::from_segments_with_mode(
            segments,
            selection.primary_index(),
            selection.is_rectangular(),
        );
        *self.inner.desired_columns.borrow_mut() = if vertical { columns } else { Vec::new() };
    }

    fn move_virtual_position(
        &self,
        position: (usize, usize),
        movement: CaretMovement,
        allow_virtual: bool,
        desired: Option<usize>,
    ) -> (usize, usize) {
        let (offset, virtual_space) = position;
        let Ok(line) = self.document().get_line_by_offset(offset) else {
            return position;
        };
        match movement {
            CaretMovement::CharacterLeft if virtual_space != 0 => {
                return (offset, virtual_space - 1)
            }
            CaretMovement::WordLeft if virtual_space != 0 => return (offset, 0),
            CaretMovement::CharacterLeft if allow_virtual && offset == line.offset => {
                return position
            }
            CaretMovement::CharacterRight
                if (allow_virtual || virtual_space != 0) && offset == line.end_offset() =>
            {
                return (offset, virtual_space.saturating_add(1));
            }
            CaretMovement::LineUp | CaretMovement::LineDown => {
                let number = if movement == CaretMovement::LineUp {
                    line.line_number.saturating_sub(1)
                } else {
                    line.line_number + 1
                };
                let Ok(target) = self.document().get_line_by_number(number) else {
                    return position;
                };
                let Ok(text) = self.document().get_text(target.offset, target.length) else {
                    return position;
                };
                let column = desired
                    .or_else(|| self.visual_position(offset, virtual_space).ok())
                    .unwrap_or(0);
                let target_position =
                    position_at_column(&text, column, self.options().indentation_size);
                return (
                    target.offset + target_position.0,
                    if allow_virtual { target_position.1 } else { 0 },
                );
            }
            _ => {}
        }
        (self.move_position(offset, movement), 0)
    }

    pub(super) fn delete_with_virtual_space(
        &self,
        backward: bool,
        by_word: bool,
    ) -> Result<(), EditError> {
        let original = self.selection();
        let movement = match (backward, by_word) {
            (true, false) => CaretMovement::CharacterLeft,
            (false, false) => CaretMovement::CharacterRight,
            (true, true) => CaretMovement::WordLeft,
            (false, true) => CaretMovement::WordRight,
        };
        let mut segments = Vec::with_capacity(original.segments().len());
        for segment in original.segments() {
            if !segment.is_empty() {
                segments.push(*segment);
            } else if backward && segment.caret_virtual_space != 0 {
                let target = (
                    segment.caret,
                    if by_word {
                        0
                    } else {
                        segment.caret_virtual_space - 1
                    },
                );
                segments.push(SelectionSegment::from_tuples(segment.caret_tuple(), target));
            } else {
                let target = self.move_position(segment.caret, movement);
                let crosses_line = self
                    .document()
                    .get_line_by_offset(segment.caret)?
                    .line_number
                    != self.document().get_line_by_offset(target)?.line_number;
                if original.is_rectangular() && crosses_line || target == segment.caret {
                    segments.push(*segment);
                } else {
                    segments.push(SelectionSegment::new(segment.caret, target));
                }
            }
        }
        self.set_selection(Selection::from_segments_with_mode(
            segments,
            original.primary_index(),
            original.is_rectangular(),
        ))?;
        if let Err(error) = self.replace_selection_with_text("") {
            self.set_selection(original)?;
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn insert_tab_with_virtual_space(&self) -> Result<(), EditError> {
        let selection = self.selection();
        let multiline = selection.is_rectangular() && selection.segments().len() > 1
            || selection.segments().iter().any(|segment| {
                self.document()
                    .get_line_by_offset(segment.start_offset())
                    .ok()
                    .map(|line| line.line_number)
                    != self
                        .document()
                        .get_line_by_offset(segment.end_offset())
                        .ok()
                        .map(|line| line.line_number)
            });
        if multiline {
            return self.indent_selection(false);
        }
        let options = self.options();
        let replacements: Result<Vec<_>, DocumentError> = selection
            .segments()
            .iter()
            .map(|segment| {
                if !options.convert_tabs_to_spaces {
                    return Ok("\t".to_owned());
                }
                let column =
                    self.visual_position(segment.start_offset(), segment.start_virtual_space())?;
                Ok(" ".repeat(options.indentation_size - column % options.indentation_size))
            })
            .collect();
        self.replace_selections(&replacements?)
    }
}

fn split_rectangular_text(text: &str, maximum_rows: usize) -> Vec<&str> {
    let mut rows = Vec::with_capacity(maximum_rows.min(16));
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() && rows.len() + 1 < maximum_rows {
        if matches!(bytes[index], b'\r' | b'\n') {
            rows.push(&text[start..index]);
            if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
            start = index + 1;
        }
        index += 1;
    }
    rows.push(&text[start..]);
    rows
}

#[cfg(test)]
mod tests;
