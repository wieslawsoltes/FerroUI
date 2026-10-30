use super::{TextInputMethodError, TextSelection};

/// An exact platform edit in the current visible text, including an IME preview.
/// All positions count UTF-16 code units. `range` is ordered; `selection` is
/// directed and refers to the resulting text. Empty text is a real deletion.
///
/// This is the portable contract of CoreText's Range/Text/NewSelection request.
/// A client must reject unsupported filtering, normalization or length changes
/// before mutation; it must never acknowledge a different replacement as exact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputMethodEdit {
    pub range: TextSelection,
    pub text: String,
    pub selection: TextSelection,
}

impl TextInputMethodEdit {
    pub fn new(range: TextSelection, text: impl Into<String>, selection: TextSelection) -> Self {
        Self {
            range,
            text: text.into(),
            selection,
        }
    }

    /// Validates both ranges without clamping or splitting a surrogate pair and
    /// returns the complete candidate text. No client state is changed.
    pub fn apply_to(&self, current: &str) -> Result<String, TextInputMethodError> {
        if self.range.start > self.range.end {
            return Err(TextInputMethodError::InvalidRange);
        }
        let start = byte_offset(current, self.range.start)?;
        let end = byte_offset(current, self.range.end)?;
        let capacity = current
            .len()
            .checked_sub(end - start)
            .and_then(|length| length.checked_add(self.text.len()))
            .ok_or(TextInputMethodError::InvalidRange)?;
        let mut result = String::with_capacity(capacity);
        result.push_str(&current[..start]);
        result.push_str(&self.text);
        result.push_str(&current[end..]);
        if result.encode_utf16().count() > i32::MAX as usize {
            return Err(TextInputMethodError::InvalidRange);
        }
        validate_selection(&result, self.selection)?;
        Ok(result)
    }
}

/// A coherent visible state exposed by an exact-edit client. Unlike legacy
/// surrounding-text APIs, this includes the current accepted composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputMethodState {
    pub text: String,
    pub selection: TextSelection,
}

pub(crate) fn validate_selection(
    text: &str,
    selection: TextSelection,
) -> Result<(), TextInputMethodError> {
    byte_offset(text, selection.start)?;
    byte_offset(text, selection.end)?;
    Ok(())
}

fn byte_offset(text: &str, position: i32) -> Result<usize, TextInputMethodError> {
    let position = usize::try_from(position).map_err(|_| TextInputMethodError::InvalidRange)?;
    let mut units = 0;
    for (byte, character) in text.char_indices() {
        if units == position {
            return Ok(byte);
        }
        units += character.len_utf16();
        if units > position {
            return Err(TextInputMethodError::InvalidRange);
        }
    }
    (units == position)
        .then_some(text.len())
        .ok_or(TextInputMethodError::InvalidRange)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_edit_replaces_utf16_range_and_keeps_directed_result_selection() {
        let edit =
            TextInputMethodEdit::new(TextSelection::new(1, 3), "xy", TextSelection::new(3, 1));
        assert_eq!(edit.apply_to("a😀b").unwrap(), "axyb");
    }

    #[test]
    fn exact_edit_deletion_is_not_an_empty_input_no_op() {
        let edit = TextInputMethodEdit::new(TextSelection::new(0, 4), "", TextSelection::new(0, 0));
        assert_eq!(edit.apply_to("a😀b").unwrap(), "");
    }

    #[test]
    fn exact_edit_rejects_surrogate_interiors_and_out_of_bounds_without_clamping() {
        for range in [
            TextSelection::new(-1, 0),
            TextSelection::new(2, 3),
            TextSelection::new(3, 2),
            TextSelection::new(0, 5),
        ] {
            let edit = TextInputMethodEdit::new(range, "x", TextSelection::new(0, 0));
            assert_eq!(
                edit.apply_to("a😀b"),
                Err(TextInputMethodError::InvalidRange)
            );
        }
        let edit =
            TextInputMethodEdit::new(TextSelection::new(0, 0), "😀", TextSelection::new(1, 1));
        assert_eq!(edit.apply_to(""), Err(TextInputMethodError::InvalidRange));
    }
}
