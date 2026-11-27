use super::*;
use crate::document::DocumentChangeEventArgs;

impl EditorSession {
    pub(super) fn insert_rectangular_line_break(
        &self,
        selection: &Selection,
        newline: &str,
        validate_host: impl Fn() -> Result<(), EditError>,
    ) -> Result<(), EditError> {
        let document = self.document();
        let version = document.version();
        let provider = self.read_only_section_provider();
        let validate_target = || {
            validate_host()?;
            self.validate_protected_edit_context(selection, &version, &provider)
        };
        let mut plan =
            self.plan_rectangular_replacement(selection, newline, &version, &provider)?;
        let segments = selection.segments();
        let bottom = document
            .get_line_by_offset(segments.last().expect("a rectangle has a row").end_offset())?
            .line_number;
        let preview = TextDocument::from_snapshot(document.create_snapshot());
        let original = preview.version();
        preview.apply_edits(&plan.edits)?;

        let first_end = segments[0]
            .start_offset()
            .saturating_add(plan.first_insertion_length)
            .min(preview.text_length());
        let column = preview.get_location(first_end)?.column - 1;
        let line = preview.get_line_by_number(bottom.min(preview.line_count()))?;
        let mut caret = line.offset + column.min(line.length);
        if let Some(strategy) = self.indentation_strategy() {
            let before_indent = preview.version();
            let reverse_changes = before_indent.get_changes_to(&original)?;
            let start = translate_offset(
                &reverse_changes,
                line.offset,
                AnchorMovementType::AfterInsertion,
            );
            let end = translate_offset(
                &reverse_changes,
                line.end_offset(),
                AnchorMovementType::BeforeInsertion,
            );
            let range = SimpleSegment::new(start, end.saturating_sub(start));
            let writable = Self::writable_parts(provider.as_ref(), range);
            validate_target()?;
            if writable?.as_slice() == [range] {
                let indentation = strategy.indent_line_on_enter(&preview, line.line_number)?;
                if preview.version() != before_indent {
                    return Err(EditError::StaleEditPlan);
                }
                if !indentation.is_empty() {
                    preview.apply_edits_with_mapping(
                        &indentation,
                        OffsetChangeMappingType::RemoveAndInsert,
                    )?;
                    caret = before_indent.move_offset_to(
                        &preview.version(),
                        caret,
                        AnchorMovementType::AfterInsertion,
                    )?;
                    plan.edits = compose_indentation(
                        &original,
                        &preview,
                        &reverse_changes,
                        &plan.edits,
                        &indentation,
                    )?;
                }
            }
        }
        validate_target()?;
        let update = self.begin_prepared_update(&plan.edits, &version)?;
        validate_target()?;
        document.apply_edits(&plan.edits)?;
        self.set_caret_offset(caret)?;
        if let Some(update) = update {
            update.finish()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_indentation_error_leaves_the_rectangle_and_document_unchanged() {
        struct RejectIndentation;

        impl IndentationStrategy for RejectIndentation {
            fn indent_line(
                &self,
                preview: &TextDocument,
                line: usize,
            ) -> Result<Vec<DocumentEdit>, IndentationError> {
                assert_eq!(preview.text(), "af\ngl\nmnopqr");
                assert_eq!(line, 3);
                Err(IndentationError::InvalidAccessorState)
            }

            fn indent_lines(
                &self,
                _: &TextDocument,
                _: usize,
                _: usize,
            ) -> Result<Vec<DocumentEdit>, IndentationError> {
                Err(IndentationError::InvalidAccessorState)
            }
        }

        let original = "abcdef\nghijkl\nmnopqr";
        let document = TextDocument::from_text(original);
        let editor = EditorSession::new(document.clone());
        let selection = Selection::from_rectangle_segments(
            [(1, 5), (8, 12), (15, 19)].map(|(start, end)| SelectionSegment::new(start, end)),
            2,
        );
        editor.set_selection(selection.clone()).unwrap();
        editor.set_indentation_strategy(Some(Rc::new(RejectIndentation)));
        assert!(matches!(
            editor.insert_document_new_line(),
            Err(EditError::Indentation(
                IndentationError::InvalidAccessorState
            ))
        ));
        assert_eq!(document.text(), original);
        assert_eq!(editor.selection(), selection);
        assert!(!document.can_undo());
    }
}

fn translate_offset(
    changes: &[DocumentChangeEventArgs],
    offset: usize,
    movement: AnchorMovementType,
) -> usize {
    changes.iter().fold(offset, |offset, change| {
        change.get_new_offset(offset, movement)
    })
}

/// Rebase indentation through the preview and resolve overlaps with deleted rows
/// before submitting a single preflighted batch in live document coordinates.
fn compose_indentation(
    original: &DocumentVersion,
    preview: &TextDocument,
    reverse_changes: &[DocumentChangeEventArgs],
    replacement: &[DocumentEdit],
    indentation: &[DocumentEdit],
) -> Result<Vec<DocumentEdit>, DocumentError> {
    let mut ranges: Vec<_> = replacement
        .iter()
        .map(|edit| (edit.offset, edit.offset + edit.length))
        .chain(indentation.iter().map(|edit| {
            (
                translate_offset(
                    reverse_changes,
                    edit.offset,
                    AnchorMovementType::BeforeInsertion,
                ),
                translate_offset(
                    reverse_changes,
                    edit.offset + edit.length,
                    AnchorMovementType::AfterInsertion,
                ),
            )
        }))
        .collect();
    ranges.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());
    for (start, end) in ranges {
        match merged.last_mut() {
            Some((_, previous_end)) if *previous_end >= start => {
                *previous_end = (*previous_end).max(end)
            }
            _ => merged.push((start, end)),
        }
    }
    let forward_changes = original.get_changes_to(&preview.version())?;
    merged
        .into_iter()
        .map(|(start, end)| {
            let first =
                translate_offset(&forward_changes, start, AnchorMovementType::BeforeInsertion);
            let last = translate_offset(&forward_changes, end, AnchorMovementType::AfterInsertion);
            Ok(DocumentEdit::new(
                start,
                end - start,
                preview.get_text(first, last - first)?,
            ))
        })
        .collect()
}
