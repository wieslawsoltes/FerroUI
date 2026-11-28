use super::*;
use crate::document::DocumentEdit;
use crate::editing::rectangle_selection::{position_at_column, visual_column};

struct PasteRow {
    position: SelectionSegment,
    text: String,
    padding_length: usize,
    insertion_length: usize,
}

impl EditorSession {
    /// Pastes a column of text at one empty caret using logical line and cell
    /// coordinates. Tabs advance to configured stops and graphemes occupy one
    /// cell. Native views can use `paste_rectangle_at` for pixel projection.
    ///
    /// Returns `false` without changing text, selection, or history if the
    /// selection is not one caret, too few existing lines remain, or virtual
    /// space is disabled and the last line cannot reach the starting UTF-16
    /// column. The caller may then perform an ordinary paste. CR, LF, and CRLF
    /// separate input rows; a trailing separator includes a final empty row.
    /// Existing document delimiters remain unchanged.
    ///
    /// Paste skips protected rows. Writable rows share one undo unit.
    /// With `select_inserted_text`, the inserted ranges stay selected in
    /// rectangle mode. Otherwise the caret moves to the final row, using the
    /// first row's intended UTF-16 end column, clamped to that final row.
    pub fn paste_rectangle(
        &self,
        text: &str,
        select_inserted_text: bool,
    ) -> Result<bool, EditError> {
        self.paste_rectangle_checked(text, select_inserted_text, || true)
    }

    /// Lets the view reject a paste when a callback changes its binding.
    pub fn paste_rectangle_checked(
        &self,
        text: &str,
        select_inserted_text: bool,
        is_current: impl Fn() -> bool,
    ) -> Result<bool, EditError> {
        let selection = self.selection();
        if selection.segments().len() != 1 || !selection.is_empty() {
            return Ok(false);
        }
        self.validate_selection(&selection)?;
        let caret = selection.primary();
        let first = self.document().get_line_by_offset(caret.caret)?;
        if caret.caret > first.end_offset() {
            return Err(EditError::InvalidSelection);
        }
        let available = self.document().line_count() - first.line_number + 1;
        let Some(rows) = split_paste_rows(text, available) else {
            return Ok(false);
        };
        let last_number = first.line_number + rows.len() - 1;
        let last = self.document().get_line_by_number(last_number)?;
        let options = self.options();
        if !options.enable_virtual_space
            && !selection.is_rectangular()
            && last.length < caret.caret - first.offset
        {
            return Ok(false);
        }
        let prefix = self
            .document()
            .get_text(first.offset, caret.caret - first.offset)?;
        let column = visual_column(&prefix, options.indentation_size)
            .checked_add(caret.caret_virtual_space)
            .ok_or(EditError::VirtualSpaceTooLarge)?;
        let mut positions = Vec::with_capacity(rows.len());
        positions.push(caret);
        for number in first.line_number + 1..=last_number {
            let line = self.document().get_line_by_number(number)?;
            let content = self.document().get_text(line.offset, line.length)?;
            let (offset, virtual_space) =
                position_at_column(&content, column, options.indentation_size);
            positions.push(SelectionSegment::with_virtual_space(
                line.offset + offset,
                virtual_space,
                line.offset + offset,
                virtual_space,
            ));
        }
        let positions = Selection::from_rectangle_segments(positions, rows.len() - 1);
        self.apply_rectangle_paste(&positions, &rows, select_inserted_text, None, &is_current)?;
        Ok(true)
    }

    /// Pastes one input row at each explicitly projected rectangle caret.
    /// Preparing `positions` does not require changing the live selection.
    /// Each segment must be empty, in rectangle mode, and on a distinct logical
    /// line. Rows may skip lines, allowing a native view to omit folded lines.
    /// Positions use current UTF-16 offsets and optional cells beyond line end.
    ///
    /// The number of CR/LF/CRLF-delimited input rows must equal the number of
    /// positions. Protected rows are skipped before calculating padding.
    /// All positions are checked first, and every insertion shares one undo unit.
    /// Host callbacks that change the document during preparation invalidate
    /// the plan. The operation itself leaves selection unchanged on failure.
    ///
    /// Selected inserted ranges exclude virtual-space padding. Empty input rows
    /// do not materialize padding. Without `select_inserted_text`, one caret is
    /// placed on the last target line at the first row's intended UTF-16 end
    /// column, clamped to the last line. A single-row empty paste retains any
    /// unmaterialized virtual caret. The prepared selection's primary index is
    /// preserved when selecting the inserted ranges.
    pub fn paste_rectangle_at(
        &self,
        positions: &Selection,
        text: &str,
        select_inserted_text: bool,
    ) -> Result<(), EditError> {
        self.paste_rectangle_at_checked(positions, text, select_inserted_text, || true)
    }

    /// Lets the view reject a projected paste when a callback changes its binding.
    pub fn paste_rectangle_at_checked(
        &self,
        positions: &Selection,
        text: &str,
        select_inserted_text: bool,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        self.paste_projected_rectangle(positions, text, select_inserted_text, None, &is_current)
    }

    /// Drops a rectangle at projected carets and optionally removes its source
    /// ranges in the same undo unit. All destinations must be writable because
    /// a native move can also remove the source in another document.
    pub fn drop_rectangle_at(
        &self,
        positions: &Selection,
        text: &str,
        source_deletions: &[DocumentEdit],
    ) -> Result<(), EditError> {
        self.paste_projected_rectangle(positions, text, true, Some(source_deletions), &|| true)
    }

    fn paste_projected_rectangle(
        &self,
        positions: &Selection,
        text: &str,
        select_inserted_text: bool,
        source_deletions: Option<&[DocumentEdit]>,
        is_current: &dyn Fn() -> bool,
    ) -> Result<(), EditError> {
        if !positions.is_rectangular()
            || !positions.is_empty()
            || source_deletions.is_some_and(|edits| edits.iter().any(|edit| !edit.text.is_empty()))
        {
            return Err(EditError::InvalidSelection);
        }
        let rows = split_paste_rows(text, positions.segments().len())
            .filter(|rows| rows.len() == positions.segments().len())
            .ok_or(EditError::ReplacementCount)?;
        self.apply_rectangle_paste(positions, &rows, select_inserted_text, source_deletions, is_current)
    }

    /// Inserts and selects dropped text and removes any same-document move
    /// ranges in one edit. Optional virtual space is materialized as padding.
    pub fn drop_text_at(
        &self,
        position: SelectionSegment,
        text: &str,
        source_deletions: &[DocumentEdit],
    ) -> Result<(), EditError> {
        if !position.is_empty()
            || text.is_empty()
            || source_deletions.iter().any(|edit| !edit.text.is_empty())
        {
            return Err(EditError::InvalidSelection);
        }
        let version = self.document().version();
        let selection = self.selection();
        self.validate_virtual_edit_positions(&Selection::from_segments([position], 0))?;
        let padded = self.with_virtual_padding(position, text)?;
        let insertion_length = padded.encode_utf16().count();
        let padding_length = insertion_length - text.encode_utf16().count();
        let mut edits = source_deletions.to_vec();
        edits.push(DocumentEdit::new(position.caret, 0, &padded));
        let update = self.begin_prepared_update(&edits, &version)?;
        if self.selection() != selection {
            return Err(EditError::StaleEditPlan);
        }
        let anchor = self.document().create_anchor(position.caret)?;
        anchor.set_movement_type(AnchorMovementType::BeforeInsertion);
        self.document().apply_edits(&edits)?;
        let start = anchor.offset()?;
        self.set_selection(Selection::simple(
            start + padding_length,
            start + insertion_length,
        ))?;
        if let Some(update) = update {
            update.finish()?;
        }
        Ok(())
    }

    fn apply_rectangle_paste(
        &self,
        positions: &Selection,
        rows: &[&str],
        select_inserted_text: bool,
        source_deletions: Option<&[DocumentEdit]>,
        is_current: &dyn Fn() -> bool,
    ) -> Result<(), EditError> {
        let version = self.document().version();
        let selection = self.selection();
        let provider = self.read_only_section_provider();
        let validate_target = || {
            if !is_current() {
                return Err(EditError::StaleEditPlan);
            }
            self.validate_protected_edit_context(&selection, &version, &provider)
        };
        validate_target()?;
        self.validate_virtual_edit_positions(positions)?;
        let mut previous_line = 0;
        let mut plan = Vec::with_capacity(rows.len());
        let mut first_intended_length = 0;
        for (position, text) in positions.segments().iter().copied().zip(rows) {
            let line = self.document().get_line_by_offset(position.caret)?;
            if line.line_number <= previous_line || position.caret > line.end_offset() {
                return Err(EditError::InvalidSelection);
            }
            previous_line = line.line_number;
            let skipped = if source_deletions.is_none() && !text.is_empty() {
                let writable = provider.can_insert(position.caret);
                validate_target()?;
                !writable
            } else {
                false
            };
            let padded = if skipped {
                String::new()
            } else {
                self.with_virtual_padding(position, text)?
            };
            let text_length = text.encode_utf16().count();
            let insertion_length = padded.encode_utf16().count();
            if plan.is_empty() {
                first_intended_length = if skipped { text_length } else { insertion_length };
            }
            plan.push(PasteRow {
                position,
                padding_length: insertion_length.saturating_sub(text_length),
                insertion_length,
                text: padded,
            });
        }
        let mut edits = source_deletions.unwrap_or_default().to_vec();
        edits.extend(
            plan.iter()
                .filter(|row| !row.text.is_empty())
                .map(|row| DocumentEdit::new(row.position.caret, 0, &row.text)),
        );
        let update = self.begin_prepared_update(&edits, &version)?;
        validate_target()?;
        let anchors: Result<Vec<_>, DocumentError> = plan
            .iter()
            .map(|row| {
                let anchor = self.document().create_anchor(row.position.caret)?;
                anchor.set_movement_type(AnchorMovementType::BeforeInsertion);
                anchor.set_survive_deletion(true);
                Ok(anchor)
            })
            .collect();
        let anchors = anchors?;
        if !edits.is_empty() {
            self.document().apply_edits(&edits)?;
        }
        let inserted: Result<Vec<_>, DocumentError> = anchors
            .iter()
            .zip(&plan)
            .map(|(anchor, row)| {
                let start = anchor.offset()?;
                let virtual_space = if row.text.is_empty() {
                    row.position.caret_virtual_space
                } else {
                    0
                };
                Ok(SelectionSegment::with_virtual_space(
                    start + row.padding_length,
                    virtual_space,
                    start + row.insertion_length,
                    virtual_space,
                ))
            })
            .collect();
        let inserted = inserted?;
        if select_inserted_text {
            self.set_selection(Selection::from_rectangle_segments(
                inserted,
                positions.primary_index(),
            ))?;
        } else if inserted.len() == 1 && first_intended_length == plan[0].insertion_length {
            self.set_caret_position(inserted[0].caret, inserted[0].caret_virtual_space)?;
        } else {
            let first_end = anchors[0].offset()?.saturating_add(first_intended_length)
                .min(self.document().text_length());
            let column = self.document().get_location(first_end)?.column - 1;
            let line = self.document().get_line_by_number(previous_line)?;
            self.set_caret_offset(line.offset + column.min(line.length))?;
        }
        // Consolidated notifications may mirror an edit. The live selection is
        // installed first so its normal document subscription tracks that edit.
        if let Some(update) = update {
            update.finish()?;
        }
        Ok(())
    }
}

/// Borrow complete rows and reject overflow without allocating for the whole
/// clipboard. Unlike `str::lines`, this preserves a trailing empty row and
/// recognizes a lone CR as well as LF and CRLF.
fn split_paste_rows(text: &str, maximum: usize) -> Option<Vec<&str>> {
    if maximum == 0 {
        return None;
    }
    let mut rows = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut offset = 0;
    while offset < bytes.len() {
        if matches!(bytes[offset], b'\r' | b'\n') {
            if rows.len() + 1 == maximum {
                return None;
            }
            rows.push(&text[start..offset]);
            offset += if bytes[offset] == b'\r' && bytes.get(offset + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            start = offset;
        } else {
            offset += 1;
        }
    }
    rows.push(&text[start..]);
    Some(rows)
}

#[cfg(test)]
mod tests;
