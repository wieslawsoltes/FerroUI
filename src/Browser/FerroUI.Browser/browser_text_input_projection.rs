use crate::browser_text_input_edit::{utf16_length, validate_selection};
use ferroui_base::input::text_input::{
    TextInputMethodEdit, TextInputMethodError, TextInputMethodState, TextSelection,
};

#[derive(Clone)]
struct CollapsedLineEnding {
    model_interior: i32,
    dom_end: i32,
}

/// The textarea API exposes LF even when a rich Run stores CR or CRLF. Keep
/// that projection separate from the authoritative document and map UTF-16
/// boundaries in both directions. Only CRLF changes the number of code units.
/// https://html.spec.whatwg.org/multipage/form-elements.html#the-textarea-element
#[derive(Clone)]
pub(crate) struct DomTextProjection {
    pub state: TextInputMethodState,
    /// A caret between CR and LF has no corresponding DOM boundary. It may be
    /// displayed before the projected LF, but cannot authorize a native edit.
    pub selection_is_exact: bool,
    collapsed: Vec<CollapsedLineEnding>,
}

impl DomTextProjection {
    pub fn new(model: &TextInputMethodState) -> Result<Self, TextInputMethodError> {
        utf16_length(&model.text)?;
        validate_selection(&model.text, model.selection)?;
        let mut text = String::with_capacity(model.text.len());
        let mut collapsed = Vec::new();
        let mut characters = model.text.chars().peekable();
        let mut model_offset = 0;
        let mut dom_offset = 0;
        while let Some(character) = characters.next() {
            if character == '\r' {
                text.push('\n');
                model_offset += 1;
                dom_offset += 1;
                if characters.peek() == Some(&'\n') {
                    collapsed.push(CollapsedLineEnding {
                        model_interior: model_offset,
                        dom_end: dom_offset,
                    });
                    characters.next();
                    model_offset += 1;
                }
            } else {
                text.push(character);
                let units = character.len_utf16() as i32;
                model_offset += units;
                dom_offset += units;
            }
        }
        let exact = |position| {
            !collapsed
                .iter()
                .any(|ending| ending.model_interior == position)
        };
        let to_dom = |position| {
            position - collapsed.partition_point(|ending| ending.model_interior <= position) as i32
        };
        Ok(Self {
            state: TextInputMethodState {
                text,
                selection: TextSelection::new(
                    to_dom(model.selection.start),
                    to_dom(model.selection.end),
                ),
            },
            selection_is_exact: exact(model.selection.start) && exact(model.selection.end),
            collapsed,
        })
    }

    pub fn selection_to_model(
        &self,
        selection: TextSelection,
    ) -> Result<TextSelection, TextInputMethodError> {
        validate_selection(&self.state.text, selection)?;
        let to_model = |position| {
            position
                + self
                    .collapsed
                    .partition_point(|ending| ending.dom_end <= position) as i32
        };
        Ok(TextSelection::new(
            to_model(selection.start),
            to_model(selection.end),
        ))
    }

    /// Converts the changed range and the result selection separately: an edit
    /// can remove CRLF pairs, so the latter belongs to the new projection.
    pub fn edit_to_model(
        &self,
        original: &str,
        edit: &TextInputMethodEdit,
    ) -> Result<TextInputMethodEdit, TextInputMethodError> {
        if !self.selection_is_exact || edit.text.contains('\r') {
            return Err(TextInputMethodError::InvalidRange);
        }
        let expected = edit.apply_to(&self.state.text)?;
        let range = self.selection_to_model(edit.range)?;
        let mut result = TextInputMethodEdit::new(range, &edit.text, TextSelection::default());
        let candidate = result.apply_to(original)?;
        let projection = Self::new(&TextInputMethodState {
            text: candidate,
            selection: TextSelection::default(),
        })?;
        // Inserting LF after an untouched CR (or deleting text between CR and
        // LF) can form a new CRLF pair. Do not change adjacent model content to
        // make that unrepresentable DOM edit fit; reject it before mutation.
        if projection.state.text != expected {
            return Err(TextInputMethodError::InvalidRange);
        }
        result.selection = projection.selection_to_model(edit.selection)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_text_input_edit::edit_from_dom;

    fn state(text: &str, anchor: i32, active: i32) -> TextInputMethodState {
        TextInputMethodState {
            text: text.to_owned(),
            selection: TextSelection::new(anchor, active),
        }
    }

    #[test]
    fn browser_dom_projection_maps_mixed_newlines_and_directed_utf16_boundaries() {
        let model = state("a\r\n😀\rb\r\nc", 10, 3);
        let projection = DomTextProjection::new(&model).unwrap();
        assert_eq!(projection.state, state("a\n😀\nb\nc", 8, 2));
        assert!(projection.selection_is_exact);
        for (dom, model) in [
            (0, 0),
            (1, 1),
            (2, 3),
            (4, 5),
            (5, 6),
            (6, 7),
            (7, 9),
            (8, 10),
        ] {
            assert_eq!(
                projection.selection_to_model(TextSelection::new(dom, 0)),
                Ok(TextSelection::new(model, 0))
            );
        }
        for selection in [
            TextSelection::new(3, 3),
            TextSelection::new(-1, 0),
            TextSelection::new(0, 9),
        ] {
            assert_eq!(
                projection.selection_to_model(selection),
                Err(TextInputMethodError::InvalidRange)
            );
        }
    }

    #[test]
    fn browser_dom_projection_keeps_unchanged_normalization_out_of_document_edits() {
        let model = state("a\r\n😀\rb", 7, 3);
        let projection = DomTextProjection::new(&model).unwrap();
        assert_eq!(
            edit_from_dom(&projection.state, None, &projection.state, "insertText"),
            Ok(None)
        );
        assert_eq!(
            projection.selection_to_model(projection.state.selection),
            Ok(model.selection)
        );
    }

    #[test]
    fn browser_dom_projection_translates_replacement_and_result_selection_independently() {
        let model = state("a\r\n😀\rb", 5, 3);
        let projection = DomTextProjection::new(&model).unwrap();
        let edit = edit_from_dom(
            &projection.state,
            Some(projection.state.selection),
            &state("a\nX\nb", 3, 2),
            "insertReplacementText",
        )
        .unwrap()
        .unwrap();
        let edit = projection.edit_to_model(&model.text, &edit).unwrap();
        assert_eq!(edit.range, TextSelection::new(3, 5));
        assert_eq!(edit.selection, TextSelection::new(4, 3));
        assert_eq!(edit.apply_to(&model.text).unwrap(), "a\r\nX\rb");

        let delete =
            TextInputMethodEdit::new(TextSelection::new(1, 2), "", TextSelection::new(1, 1));
        let delete = projection.edit_to_model(&model.text, &delete).unwrap();
        assert_eq!(delete.range, TextSelection::new(1, 3));
        assert_eq!(delete.selection, TextSelection::new(1, 1));
        assert_eq!(delete.apply_to(&model.text).unwrap(), "a😀\rb");
    }

    #[test]
    fn browser_dom_projection_rejects_crlf_interiors_and_new_collapsed_pairs() {
        let model = state("a\r\nb", 2, 2);
        let projection = DomTextProjection::new(&model).unwrap();
        assert_eq!(projection.state.selection, TextSelection::new(1, 1));
        assert!(!projection.selection_is_exact);
        let edit =
            TextInputMethodEdit::new(TextSelection::new(1, 1), "x", TextSelection::new(2, 2));
        assert_eq!(
            projection.edit_to_model(&model.text, &edit),
            Err(TextInputMethodError::InvalidRange)
        );

        for (model, edit) in [
            (
                state("a\rb", 2, 2),
                TextInputMethodEdit::new(TextSelection::new(2, 2), "\n", TextSelection::new(3, 3)),
            ),
            (
                state("a\rx\nb", 2, 3),
                TextInputMethodEdit::new(TextSelection::new(2, 3), "", TextSelection::new(2, 2)),
            ),
        ] {
            let projection = DomTextProjection::new(&model).unwrap();
            assert_eq!(
                projection.edit_to_model(&model.text, &edit),
                Err(TextInputMethodError::InvalidRange)
            );
        }
    }

    #[test]
    fn browser_dom_projection_allows_explicit_replacement_of_a_selected_line_ending() {
        let model = state("a\r\nb\rc", 1, 3);
        let projection = DomTextProjection::new(&model).unwrap();
        let edit = edit_from_dom(
            &projection.state,
            Some(projection.state.selection),
            &state("a\nb\nc", 2, 2),
            "insertReplacementText",
        )
        .unwrap()
        .unwrap();
        let edit = projection.edit_to_model(&model.text, &edit).unwrap();
        assert_eq!(edit.range, TextSelection::new(1, 3));
        assert_eq!(edit.apply_to(&model.text).unwrap(), "a\nb\rc");
    }
}
