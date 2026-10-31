use ferroui_base::input::text_input::{
    TextInputMethodEdit, TextInputMethodError, TextInputMethodState, TextSelection,
};

/// Produces one exact edit from a textarea's canonical LF value. Offsets count
/// UTF-16 units, while comparisons use scalar boundaries so a shared surrogate
/// cannot become the boundary between the retained text and the replacement.
///
/// The native selection captured by beforeinput is the strongest anchor. It
/// prevents repeated characters from moving an insertion/deletion to another
/// formatted run. Without beforeinput, the resulting caret anchors ordinary
/// insertions/deletions before the minimal changed span is used as a fallback.
pub(crate) fn edit_from_dom(
    before: &TextInputMethodState,
    native_selection: Option<TextSelection>,
    after: &TextInputMethodState,
    input_type: &str,
) -> Result<Option<TextInputMethodEdit>, TextInputMethodError> {
    if before.text.contains('\r') || after.text.contains('\r') {
        return Err(TextInputMethodError::InvalidRange);
    }
    validate_selection(&before.text, before.selection)?;
    validate_selection(&after.text, after.selection)?;
    if let Some(selection) = native_selection {
        validate_selection(&before.text, selection)?;
    }
    let old_length = utf16_length(&before.text)?;
    let new_length = utf16_length(&after.text)?;
    let difference = new_length - old_length;

    if before.text == after.text {
        // An explicit replacement of selected identical text can still carry
        // typing attributes. An unchanged mirrored keyboard event is a no-op.
        if let Some(selection) = native_selection.filter(|selection| {
            selection.start != selection.end
                && *selection != after.selection
                && input_type.starts_with("insert")
        }) {
            return anchored_edit(
                before,
                after,
                selection.start.min(selection.end),
                selection.start.max(selection.end),
                difference,
            );
        }
        return Ok(None);
    }

    if let Some(selection) = native_selection {
        let start = selection.start.min(selection.end);
        let end = selection.start.max(selection.end);
        if let Some(edit) = anchored_edit(before, after, start, end, difference)? {
            return Ok(Some(edit));
        }
        if start == end && difference < 0 {
            let removed = -difference;
            let range = if input_type.ends_with("Backward") {
                start.checked_sub(removed).map(|start| (start, end))
            } else if input_type.ends_with("Forward") {
                end.checked_add(removed).map(|end| (start, end))
            } else {
                None
            };
            if let Some((start, end)) = range {
                if let Some(edit) = anchored_edit(before, after, start, end, difference)? {
                    return Ok(Some(edit));
                }
            }
        }
    }

    if after.selection.start == after.selection.end {
        let caret = after.selection.end;
        let range = if difference > 0 {
            caret.checked_sub(difference).map(|start| (start, start))
        } else if difference < 0 {
            caret.checked_sub(difference).map(|end| (caret, end))
        } else {
            None
        };
        if let Some((start, end)) = range {
            if let Some(edit) = anchored_edit(before, after, start, end, difference)? {
                return Ok(Some(edit));
            }
        }
    }

    let prefix = before
        .text
        .chars()
        .zip(after.text.chars())
        .take_while(|(left, right)| left == right)
        .map(|(character, _)| character.len_utf8())
        .sum::<usize>();
    let mut old_end = before.text.len();
    let mut new_end = after.text.len();
    for (left, right) in before.text[prefix..]
        .chars()
        .rev()
        .zip(after.text[prefix..].chars().rev())
    {
        if left != right {
            break;
        }
        old_end -= left.len_utf8();
        new_end -= right.len_utf8();
    }
    let edit = TextInputMethodEdit::new(
        TextSelection::new(
            utf16_length(&before.text[..prefix])?,
            utf16_length(&before.text[..old_end])?,
        ),
        &after.text[prefix..new_end],
        after.selection,
    );
    if edit.apply_to(&before.text)? != after.text {
        return Err(TextInputMethodError::InvalidRange);
    }
    Ok(Some(edit))
}

fn anchored_edit(
    before: &TextInputMethodState,
    after: &TextInputMethodState,
    start: i32,
    end: i32,
    difference: i32,
) -> Result<Option<TextInputMethodEdit>, TextInputMethodError> {
    if start < 0 || end < start {
        return Ok(None);
    }
    let Some(new_end) = end
        .checked_add(difference)
        .filter(|new_end| *new_end >= start)
    else {
        return Ok(None);
    };
    let (Ok(old_start), Ok(old_end), Ok(new_start), Ok(new_end)) = (
        byte_offset(&before.text, start),
        byte_offset(&before.text, end),
        byte_offset(&after.text, start),
        byte_offset(&after.text, new_end),
    ) else {
        return Ok(None);
    };
    if before.text[..old_start] != after.text[..new_start]
        || before.text[old_end..] != after.text[new_end..]
    {
        return Ok(None);
    }
    let edit = TextInputMethodEdit::new(
        TextSelection::new(start, end),
        &after.text[new_start..new_end],
        after.selection,
    );
    edit.apply_to(&before.text)?;
    Ok(Some(edit))
}

pub(crate) fn utf16_length(text: &str) -> Result<i32, TextInputMethodError> {
    i32::try_from(text.encode_utf16().count()).map_err(|_| TextInputMethodError::InvalidRange)
}

pub(crate) fn validate_selection(
    text: &str,
    selection: TextSelection,
) -> Result<(), TextInputMethodError> {
    byte_offset(text, selection.start)?;
    byte_offset(text, selection.end)?;
    Ok(())
}

fn byte_offset(text: &str, offset: i32) -> Result<usize, TextInputMethodError> {
    let offset = usize::try_from(offset).map_err(|_| TextInputMethodError::InvalidRange)?;
    let mut units = 0;
    for (byte, character) in text.char_indices() {
        if units == offset {
            return Ok(byte);
        }
        units += character.len_utf16();
        if units > offset {
            return Err(TextInputMethodError::InvalidRange);
        }
    }
    if units == offset {
        Ok(text.len())
    } else {
        Err(TextInputMethodError::InvalidRange)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(text: &str, anchor: i32, active: i32) -> TextInputMethodState {
        TextInputMethodState {
            text: text.to_owned(),
            selection: TextSelection::new(anchor, active),
        }
    }

    #[test]
    fn browser_dom_input_anchors_repeated_insertions_at_the_native_selection() {
        let before = state("aaaa", 2, 2);
        let edit = edit_from_dom(
            &before,
            Some(before.selection),
            &state("aaaaa", 3, 3),
            "insertText",
        )
        .unwrap()
        .unwrap();
        assert_eq!(edit.range, TextSelection::new(2, 2));
        assert_eq!(edit.text, "a");
        // A no-beforeinput event uses the resulting caret instead of putting
        // its repeated character at the longest common prefix (the end).
        let edit = edit_from_dom(&before, None, &state("aaaaa", 2, 2), "insertText")
            .unwrap()
            .unwrap();
        assert_eq!(edit.range, TextSelection::new(1, 1));
    }

    #[test]
    fn browser_dom_input_anchors_backward_forward_and_mobile_deletions() {
        let before = state("aaaa", 2, 2);
        for (input_type, after, expected) in [
            (
                "deleteContentBackward",
                state("aaa", 1, 1),
                TextSelection::new(1, 2),
            ),
            (
                "deleteContentForward",
                state("aaa", 2, 2),
                TextSelection::new(2, 3),
            ),
        ] {
            let edit = edit_from_dom(&before, Some(before.selection), &after, input_type)
                .unwrap()
                .unwrap();
            assert_eq!(edit.range, expected);
            assert!(edit.text.is_empty());
        }
        let edit = edit_from_dom(&before, None, &state("aaa", 0, 0), "deleteContentBackward")
            .unwrap()
            .unwrap();
        assert_eq!(edit.range, TextSelection::new(0, 1));
    }

    #[test]
    fn browser_dom_input_replacement_uses_the_changed_span_when_the_caret_is_outside_it() {
        let before = state("teh cat", 3, 3);
        let edit = edit_from_dom(
            &before,
            Some(before.selection),
            &state("the cat", 3, 3),
            "insertReplacementText",
        )
        .unwrap()
        .unwrap();
        assert_eq!(edit.range, TextSelection::new(1, 3));
        assert_eq!(edit.text, "he");
        assert_eq!(edit.apply_to(&before.text).unwrap(), "the cat");
    }

    #[test]
    fn browser_dom_input_keeps_scalar_boundaries_when_emoji_share_a_high_surrogate() {
        let before = state("a😀b", 4, 4);
        let after = state("a😁b", 3, 1);
        let edit = edit_from_dom(&before, None, &after, "insertReplacementText")
            .unwrap()
            .unwrap();
        assert_eq!(edit.range, TextSelection::new(1, 3));
        assert_eq!(edit.text, "😁");
        assert_eq!(edit.selection, TextSelection::new(3, 1));
        assert_eq!(edit.apply_to(&before.text).unwrap(), after.text);
    }

    #[test]
    fn browser_dom_input_preserves_selected_replacements_and_empty_deletions() {
        let before = state("a😀b", 3, 1);
        let edit = edit_from_dom(
            &before,
            Some(before.selection),
            &state("ab", 1, 1),
            "deleteContentBackward",
        )
        .unwrap()
        .unwrap();
        assert_eq!(edit.range, TextSelection::new(1, 3));
        assert_eq!(edit.text, "");
        let same = edit_from_dom(
            &before,
            Some(before.selection),
            &state("a😀b", 3, 3),
            "insertReplacementText",
        )
        .unwrap()
        .unwrap();
        assert_eq!(same.range, TextSelection::new(1, 3));
        assert_eq!(same.text, "😀");
    }

    #[test]
    fn browser_dom_input_supports_canonical_multiline_text_and_rejects_invalid_selections() {
        let before = state("a\n😀b", 4, 4);
        let after = state("a\n😀\nZb", 6, 4);
        let edit = edit_from_dom(&before, None, &after, "insertText")
            .unwrap()
            .unwrap();
        assert_eq!(edit.range, TextSelection::new(4, 4));
        assert_eq!(edit.text, "\nZ");
        assert_eq!(edit.apply_to(&before.text).unwrap(), after.text);
        for selection in [
            TextSelection::new(-1, 0),
            TextSelection::new(3, 3),
            TextSelection::new(0, 20),
        ] {
            assert_eq!(
                edit_from_dom(&before, Some(selection), &after, "insertText"),
                Err(TextInputMethodError::InvalidRange)
            );
        }
    }

    #[test]
    fn browser_dom_input_same_value_after_keyboard_commit_is_not_another_edit() {
        let before = state("aXd", 2, 2);
        assert_eq!(
            edit_from_dom(&before, None, &before, "insertText"),
            Ok(None)
        );
        assert_eq!(
            edit_from_dom(&before, Some(before.selection), &before, "insertText"),
            Ok(None)
        );
    }
}
