use super::*;
use crate::document::{CaretPositioningMode, DocumentEdit, LogicalDirection};
use crate::editing::rectangle_selection::{position_at_column, visual_column};

pub(super) struct PlannedReplacement {
    pub segment: SimpleSegment,
    pub text: String,
    pub virtual_space: usize,
    pub caret_from_end: usize,
}

pub(super) struct RectangularReplacement {
    pub edits: Vec<DocumentEdit>,
    pub first_insertion_length: usize,
    has_protected_rows: bool,
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
        overstrike: bool,
        validate_target: impl FnOnce() -> Result<(), EditError>,
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
        let overstrike = overstrike && selection.is_empty() && !multiline_rectangle;
        for (index, (segment, replacement)) in
            selection.segments().iter().zip(replacements).enumerate()
        {
            let text = self.with_virtual_padding(*segment, replacement)?;
            let mut range = segment.segment();
            if overstrike && !text.is_empty() && segment.caret_virtual_space == 0 {
                let line = self.document().get_line_by_offset(segment.caret)?;
                if segment.caret < line.end_offset() {
                    let limit = selection.segments().get(index + 1).map_or(
                        line.end_offset(), |next| next.start_offset().min(line.end_offset()),
                    );
                    let end = self
                        .move_position(segment.caret, CaretMovement::CharacterRight)
                        .min(limit);
                    range.length = end - range.offset;
                }
            }
            edits.push(PlannedReplacement {
                segment: range,
                caret_from_end: 0,
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
            validate_target,
        )
    }

    /// Replaces writable spans of the active box. A view can reject a stale
    /// target after read-only policy and update-start callbacks have run.
    pub fn replace_writable_rectangle(
        &self,
        text: &str,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let selection = self.selection();
        if !selection.is_rectangular() {
            return Err(EditError::InvalidSelection);
        }
        self.validate_virtual_edit_positions(&selection)?;
        let document = self.document();
        let version = document.version();
        let provider = self.read_only_section_provider();
        let segments = selection.segments();
        let multiline = text.contains(['\r', '\n']);
        let validate_target = || {
            if !is_current() {
                return Err(EditError::StaleEditPlan);
            }
            self.validate_protected_edit_context(&selection, &version, &provider)
        };
        let plan = self.plan_rectangular_replacement(&selection, text, &version, &provider)?;
        validate_target()?;
        if !plan.has_protected_rows {
            return self.replace_virtual_selections(&[text.to_owned()], false, validate_target);
        }
        let bottom_line = document
            .get_line_by_offset(segments.last().expect("a rectangle has a row").end_offset())?
            .line_number;
        let update = self.begin_prepared_update(&plan.edits, &version)?;
        validate_target()?;
        let row_anchors = segments
            .iter()
            .map(|segment| {
                let anchor = document.create_anchor(segment.start_offset())?;
                anchor.set_movement_type(AnchorMovementType::BeforeInsertion);
                anchor.set_survive_deletion(true);
                Ok(anchor)
            })
            .collect::<Result<Vec<_>, DocumentError>>()?;
        document.apply_edits(&plan.edits)?;

        let first_start = row_anchors[0].offset()?;
        let top = document.get_line_by_offset(first_start)?;
        let first_end = first_start.saturating_add(plan.first_insertion_length).min(top.end_offset());
        if multiline {
            let line = document.get_line_by_number(bottom_line.min(document.line_count()))?;
            self.set_caret_offset(line.offset + (first_end - top.offset).min(line.length))?;
        } else {
            let column = self.visual_position(first_end, 0)?;
            let tab_size = self.options().indentation_size;
            let carets = row_anchors
                .iter()
                .map(|anchor| {
                    let line = document.get_line_by_offset(anchor.offset()?)?;
                    let text = document.get_text(line.offset, line.length)?;
                    let (offset, virtual_space) = position_at_column(&text, column, tab_size);
                    let offset = line.offset + offset;
                    Ok(SelectionSegment::with_virtual_space(
                        offset, virtual_space, offset, virtual_space,
                    ))
                })
                .collect::<Result<Vec<_>, DocumentError>>()?;
            self.set_selection(Selection::from_rectangle_segments(carets, segments.len() - 1))?;
        }
        if let Some(update) = update {
            update.finish()?;
        }
        Ok(())
    }

    pub(super) fn plan_rectangular_replacement(
        &self,
        selection: &Selection,
        text: &str,
        version: &DocumentVersion,
        provider: &Rc<dyn IReadOnlySectionProvider>,
    ) -> Result<RectangularReplacement, EditError> {
        self.validate_protected_edit_context(selection, version, provider)?;
        let segments = selection.segments();
        let multiline = text.contains(['\r', '\n']);
        let rows = if multiline {
            split_rectangular_text(text, segments.len())
        } else {
            vec![text; segments.len()]
        };
        let mut edits = Vec::with_capacity(rows.len());
        let mut first_insertion_length = 0;
        let mut has_protected_rows = false;
        for (index, (&segment, text)) in segments.iter().zip(rows).enumerate() {
            let range = segment.segment();
            let parts = if range.length == 0 {
                Ok((text.is_empty() || provider.can_insert(range.offset))
                    .then_some(range)
                    .into_iter()
                    .collect())
            } else {
                Self::writable_parts(provider.as_ref(), range)
            };
            self.ensure_document_version(version)?;
            let parts = parts?;
            let fully_writable = parts.as_slice() == [range];
            has_protected_rows |= !fully_writable;
            let insertion = if fully_writable {
                self.with_virtual_padding(segment, text)?
            } else {
                text.to_owned()
            };
            if index == 0 {
                // Box editing projects the first row's intended end column.
                // Protection can shift or prevent the insertion in that row.
                first_insertion_length = insertion.encode_utf16().count();
            }
            if let Some((last, earlier)) = parts.split_last() {
                edits.extend(earlier.iter().filter(|part| part.length != 0).map(|part| {
                    DocumentEdit::new(part.offset, part.length, "")
                }));
                if last.length != 0 || !insertion.is_empty() {
                    edits.push(DocumentEdit::new(last.offset, last.length, insertion));
                }
            }
        }
        self.validate_protected_edit_context(selection, version, provider)?;
        Ok(RectangularReplacement { edits, first_insertion_length, has_protected_rows })
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
        self.virtual_padding_at(segment.start_offset(), spaces, text)
    }

    /// Computes the padding needed to materialize virtual cells at a line end.
    /// Hosts can plan the padding and their content as one document edit.
    pub fn virtual_space_padding_at(&self, offset: usize, spaces: usize) -> Result<String, EditError> {
        if spaces == 0 {
            return Ok(String::new());
        }
        self.virtual_padding_at(offset, spaces, "")
    }

    fn virtual_padding_at(
        &self,
        offset: usize,
        spaces: usize,
        text: &str,
    ) -> Result<String, EditError> {
        let options = self.options();
        let line = self.document().get_line_by_offset(offset)?;
        if spaces != 0 && offset != line.end_offset() {
            return Err(EditError::InvalidSelection);
        }
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
        validate_target: impl FnOnce() -> Result<(), EditError>,
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
        validate_target()?;
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
                let offset = anchor.offset()? + (edit.text.encode_utf16().count() - edit.caret_from_end);
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
        visual_destination: impl Fn(SelectionSegment) -> Result<Option<usize>, EditError>,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let original = self.selection();
        let version = self.document().version();
        let provider = self.read_only_section_provider();
        let options = self.options();
        let validate = || {
            if !is_current() || self.options() != options {
                return Err(EditError::StaleEditPlan);
            }
            self.validate_protected_edit_context(&original, &version, &provider)
        };
        validate()?;
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
                let target = visual_destination(*segment);
                validate()?;
                let target = match target? {
                    Some(offset) => offset,
                    None => self.logical_deletion_offset(segment.caret, backward, by_word)?,
                };
                let crosses_line = self
                    .document()
                    .get_line_by_offset(segment.caret)?
                    .line_number
                    != self.document().get_line_by_offset(target)?.line_number;
                if (original.is_rectangular() && crosses_line) || target == segment.caret {
                    segments.push(*segment);
                } else {
                    segments.push(SelectionSegment::from_tuples(segment.caret_tuple(), (target, 0)));
                }
            }
        }
        let expanded = Selection::from_segments_with_mode(
            segments,
            original.primary_index(),
            original.is_rectangular(),
        );
        self.validate_virtual_edit_positions(&expanded)?;

        if !original.is_rectangular() && original.segments().len() == 1 {
            let range = expanded.primary().segment();
            let parts = if range.length == 0 {
                vec![range]
            } else {
                let result = Self::writable_parts(provider.as_ref(), range);
                validate()?;
                result?
            };
            if parts.is_empty() {
                return Ok(());
            }
            let caret = original.primary();
            let padding = options.enable_virtual_space
                && caret.is_empty()
                && caret.caret_virtual_space != 0
                && (!backward || by_word)
                && parts.as_slice() == [range];
            if padding {
                let text = self.virtual_padding_at(caret.caret, caret.caret_virtual_space, "")?;
                let new_caret = range.offset + text.len();
                return self.apply_document_edits_and_select(
                    &[DocumentEdit::new(range.offset, range.length, text)],
                    Selection::empty(new_caret),
                    || validate().is_ok(),
                );
            }
            if range.length != 0 {
                let edits = parts.into_iter()
                    .filter(|part| part.length != 0)
                    .map(|part| DocumentEdit::new(part.offset, part.length, ""))
                    .collect::<Vec<_>>();
                if edits.is_empty() {
                    return Ok(());
                }
                return self.apply_document_edits_and_select(
                    &edits,
                    Selection::empty(range.offset),
                    || validate().is_ok(),
                );
            }
        }

        let edits = expanded.segments().iter().map(|segment| PlannedReplacement {
            segment: segment.segment(),
            text: String::new(),
            virtual_space: segment.start_virtual_space(),
            caret_from_end: 0,
        }).collect();
        self.execute_replacements(edits, expanded.primary_index(), expanded.is_rectangular(), false, validate)
    }

    /// Resolves a deletion boundary without a formatted view. Word deletion
    /// stops at the start of the next token and at the end of the current line.
    pub fn logical_deletion_offset(
        &self,
        offset: usize,
        backward: bool,
        by_word: bool,
    ) -> Result<usize, EditError> {
        let document = self.document();
        let line = document.get_line_by_offset(offset)?;
        let source = document.create_snapshot();
        let mode = if by_word {
            CaretPositioningMode::WordStart
        } else if backward {
            CaretPositioningMode::EveryCodepoint
        } else {
            CaretPositioningMode::Normal
        };
        if backward && offset == line.offset() {
            return Ok(document.get_line_by_number(line.line_number().saturating_sub(1))
                .map_or(0, |previous| previous.end_offset()));
        }
        if !backward && offset >= line.end_offset() {
            let Ok(next) = document.get_line_by_number(line.line_number() + 1) else {
                return Ok(document.text_length());
            };
            if !by_word {
                return Ok(next.offset());
            }
            let text = source.slice(next.offset(), next.length())?;
            let column = TextUtilities::get_next_caret_position(
                &text, -1, LogicalDirection::Forward, mode,
            )?.unwrap_or(next.length());
            return Ok(next.offset() + column);
        }
        let text = source.slice(line.offset(), line.length())?;
        let direction = if backward { LogicalDirection::Backward } else { LogicalDirection::Forward };
        let column = TextUtilities::get_next_caret_position(
            &text, (offset - line.offset()) as isize, direction, mode,
        )?.unwrap_or(if backward { 0 } else { line.length() });
        Ok(line.offset() + column)
    }

    pub(super) fn insert_tab_with_virtual_space(
        &self,
        active_caret: usize,
        is_current: impl Fn() -> bool,
    ) -> Result<(), EditError> {
        let document = self.document();
        let selection = self.selection();
        let version = document.version();
        let provider = self.read_only_section_provider();
        let options = self.options();
        let validate = || {
            if !is_current() || self.options() != options {
                return Err(EditError::StaleEditPlan);
            }
            self.validate_protected_edit_context(&selection, &version, &provider)
        };
        validate()?;
        let multiline = selection.is_rectangular() && selection.segments().len() > 1
            || selection.segments().iter().any(|segment| {
                document
                    .get_line_by_offset(segment.start_offset())
                    .ok()
                    .map(|line| line.line_number)
                    != document
                        .get_line_by_offset(segment.end_offset())
                        .ok()
                        .map(|line| line.line_number)
            });
        if multiline {
            return self.indent_selection_checked(false, is_current);
        }
        if selection.segments().len() == 1 {
            let primary = selection.primary();
            if primary.length() == 0 {
                let can_insert = provider.can_insert(primary.start_offset());
                validate()?;
                if !can_insert {
                    return Ok(());
                }
            }
            let column = document.get_location(active_caret)?.column - 1;
            let replacement = options.indentation_at_column(column);
            return self.replace_writable_selection_with_text_checked(
                &replacement,
                || validate().is_ok(),
            );
        }
        let replacements: Result<Vec<_>, DocumentError> = selection
            .segments()
            .iter()
            .map(|segment| {
                let column = document.get_location(segment.caret)?.column - 1;
                Ok(options.indentation_at_column(column))
            })
            .collect();
        self.replace_virtual_selections(&replacements?, false, validate)
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
