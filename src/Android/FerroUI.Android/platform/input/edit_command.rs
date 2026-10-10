//! What an input method asks of the text, as commands the input connection
//! queues and applies to the edit buffer when its batch ends.
//!
//! The reference has an abstract class and a class per command; here the
//! commands are the variants of [`EditCommand`], and the constructors of the
//! classes are its functions.

use super::text_edit_buffer::{utf16_len, KeyEventToDispatch, TextEditBuffer};
use ferroui_base::input::text_input::TextSelection;

pub(crate) enum EditCommand {
    /// `SelectionCommand`.
    Selection { start: i32, end: i32 },
    /// `CompositionRegionCommand`.
    CompositionRegion { start: i32, end: i32 },
    /// `DeleteRegionCommand`.
    DeleteRegion { before: i32, after: i32 },
    /// `DeleteRegionInCodePointsCommand`.
    DeleteRegionInCodePoints { before: i32, after: i32 },
    /// `CompositionTextCommand`.
    CompositionText { text: String, new_cursor_position: i32 },
    /// `CommitTextCommand`.
    CommitText { text: String, new_cursor_position: i32 },
    /// `FinishComposingCommand`.
    FinishComposing,
    /// `KeyEventCommand`.
    KeyEvent(Option<KeyEventToDispatch>),
}

/// `char.IsSurrogatePair` of two code units; a position outside the text is
/// no unit.
///
/// The reference indexes the text without looking at its length here, and
/// fails with an exception that reaches the input method when the position
/// is outside (the caret at the end of the text, for one). A position
/// outside is answered with "no pair" instead: recorded in
/// docs/porting/DEVIATIONS.md.
fn is_surrogate_pair(text: &[u16], lead: i32, trail: i32) -> bool {
    let at = |index: i32| usize::try_from(index).ok().and_then(|index| text.get(index).copied());
    match (at(lead), at(trail)) {
        (Some(lead), Some(trail)) => (0xd800..=0xdbff).contains(&lead) && (0xdc00..=0xdfff).contains(&trail),
        _ => false,
    }
}

impl EditCommand {
    pub fn selection(start: i32, end: i32) -> EditCommand {
        EditCommand::Selection { start: start.min(end), end: start.max(end) }
    }

    pub fn composition_region(start: i32, end: i32) -> EditCommand {
        EditCommand::CompositionRegion { start: start.min(end), end: start.max(end) }
    }

    pub fn delete_region(before: i32, after: i32) -> EditCommand {
        EditCommand::DeleteRegion { before, after }
    }

    pub fn delete_region_in_code_points(before: i32, after: i32) -> EditCommand {
        EditCommand::DeleteRegionInCodePoints { before, after }
    }

    pub fn composition_text(text: String, new_cursor_position: i32) -> EditCommand {
        EditCommand::CompositionText { text, new_cursor_position }
    }

    pub fn commit_text(text: String, new_cursor_position: i32) -> EditCommand {
        EditCommand::CommitText { text, new_cursor_position }
    }

    pub fn apply(&self, buffer: &TextEditBuffer) {
        match self {
            EditCommand::Selection { start, end } => {
                let length = utf16_len(&buffer.text());
                let start = (*start).clamp(0, length);
                let end = (*end).clamp(0, length);
                buffer.set_selection(TextSelection::new(start, end));
            }
            EditCommand::CompositionRegion { start, end } => {
                buffer.set_composition(Some(TextSelection::new(*start, *end)));
            }
            EditCommand::DeleteRegion { before, after } => {
                let end = utf16_len(&buffer.text()).min(buffer.selection().end + after);
                let end_count = end - buffer.selection().end;
                let start = 0.max(buffer.selection().start - before);
                buffer.remove(buffer.selection().end, end_count);
                buffer.remove(start, buffer.selection().start - start);
                buffer.set_selection(TextSelection::new(start, start));
            }
            EditCommand::DeleteRegionInCodePoints { before, after } => {
                let text: Vec<u16> = buffer.text().encode_utf16().collect();
                let selection = buffer.selection();
                let mut before_length_in_char = 0;

                for _ in 0..*before {
                    before_length_in_char += 1;
                    if selection.start > before_length_in_char
                        && is_surrogate_pair(
                            &text,
                            selection.start - before_length_in_char - 1,
                            selection.start - before_length_in_char,
                        )
                    {
                        before_length_in_char += 1;
                    }

                    if before_length_in_char == selection.start {
                        break;
                    }
                }

                let mut after_length_in_char = 0;
                for _ in 0..*after {
                    after_length_in_char += 1;
                    if selection.end > after_length_in_char
                        && is_surrogate_pair(
                            &text,
                            selection.end + after_length_in_char - 1,
                            selection.end + after_length_in_char,
                        )
                    {
                        after_length_in_char += 1;
                    }

                    if selection.end + after_length_in_char == text.len() as i32 {
                        break;
                    }
                }

                let start = selection.start - before_length_in_char;
                buffer.remove(selection.end, after_length_in_char);
                buffer.remove(start, before_length_in_char);
                buffer.set_selection(TextSelection::new(start, start));
            }
            EditCommand::CompositionText { text, new_cursor_position } => {
                buffer.set_composing_text(Some(text));
                let new_cursor = if *new_cursor_position > 0 {
                    buffer.selection().start + new_cursor_position - 1
                } else {
                    buffer.selection().start + new_cursor_position
                };
                buffer.set_selection(TextSelection::new(new_cursor, new_cursor));
            }
            EditCommand::CommitText { text, new_cursor_position } => {
                match buffer.composition() {
                    Some(composition) if buffer.has_composition() => {
                        buffer.replace(composition.start, composition.end, text);
                    }
                    _ => {
                        let selection = buffer.selection();
                        buffer.replace(selection.start, selection.end, text);
                    }
                }
                let new_cursor = if *new_cursor_position > 0 {
                    buffer.selection().start + new_cursor_position - 1
                } else {
                    buffer.selection().start + new_cursor_position - utf16_len(text)
                };
                buffer.set_selection(TextSelection::new(new_cursor, new_cursor));
            }
            EditCommand::FinishComposing => buffer.set_composition(None),
            EditCommand::KeyEvent(key_event) => buffer.dispatch_key_event(key_event.as_ref()),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the commands.
    use super::super::text_edit_buffer::test_support::*;
    use super::super::text_edit_buffer::{KEYCODE_ENTER, KEY_EVENT_ACTION_DOWN};
    use super::*;

    #[test]
    fn a_selection_is_ordered_and_clamped_to_the_text() {
        let (client, _input_method, buffer) = buffer_of("hello", 0, 0);
        EditCommand::selection(9, 2).apply(&buffer);
        assert_eq!(client.selection.get(), TextSelection::new(2, 5));
        EditCommand::selection(-3, 1).apply(&buffer);
        assert_eq!(client.selection.get(), TextSelection::new(0, 1));
    }

    #[test]
    fn a_composing_region_marks_text_that_exists() {
        let (client, _input_method, buffer) = buffer_of("hello", 5, 5);
        EditCommand::composition_region(5, 0).apply(&buffer);
        assert_eq!(buffer.composition(), Some(TextSelection::new(0, 5)));
        // The text that follows replaces the region.
        EditCommand::composition_text("help".to_string(), 1).apply(&buffer);
        assert_eq!(client.text(), "help");
        assert_eq!(buffer.composition(), Some(TextSelection::new(0, 4)));
        assert_eq!(client.selection.get(), TextSelection::new(4, 4));
    }

    #[test]
    fn deleting_around_the_caret_removes_after_and_then_before() {
        let (client, _input_method, buffer) = buffer_of("abcdef", 3, 3);
        EditCommand::delete_region(2, 1).apply(&buffer);
        assert_eq!(client.text(), "aef");
        assert_eq!(client.selection.get(), TextSelection::new(1, 1));

        // More than there is on either side.
        EditCommand::delete_region(10, 10).apply(&buffer);
        assert_eq!(client.text(), "");
        assert_eq!(client.selection.get(), TextSelection::new(0, 0));
    }

    #[test]
    fn deleting_in_code_points_takes_both_units_of_a_pair() {
        let (client, _input_method, buffer) = buffer_of("a\u{1f600}b\u{1f601}c", 4, 4);
        // Before the caret: "b" and the emoji (three units). After: the second emoji.
        EditCommand::delete_region_in_code_points(2, 1).apply(&buffer);
        assert_eq!(client.text(), "ac");
        assert_eq!(client.selection.get(), TextSelection::new(1, 1));
    }

    #[test]
    fn deleting_in_code_points_stops_at_the_ends_of_the_text() {
        let (client, _input_method, buffer) = buffer_of("ab", 2, 2);
        EditCommand::delete_region_in_code_points(5, 0).apply(&buffer);
        assert_eq!(client.text(), "");

        // One unit before the end: the reference reads the unit after the last one here.
        let (client, _input_method, buffer) = buffer_of("abcd", 3, 3);
        EditCommand::delete_region_in_code_points(0, 1).apply(&buffer);
        assert_eq!(client.text(), "abc");

        let (client, _input_method, buffer) = buffer_of("ab", 0, 0);
        EditCommand::delete_region_in_code_points(0, 5).apply(&buffer);
        assert_eq!(client.text(), "");
    }

    #[test]
    fn composing_text_follows_the_cursor_position_of_the_input_method() {
        let (client, _input_method, buffer) = buffer_of("ab", 1, 1);
        // 1: the caret after the text.
        EditCommand::composition_text("xy".to_string(), 1).apply(&buffer);
        assert_eq!(client.text(), "axyb");
        assert_eq!(client.selection.get(), TextSelection::new(3, 3));
        assert_eq!(buffer.composition(), Some(TextSelection::new(1, 3)));

        // 0 and below count from the caret the replacement left after the text.
        EditCommand::composition_text("xyz".to_string(), 0).apply(&buffer);
        assert_eq!(client.text(), "axyzb");
        assert_eq!(client.selection.get(), TextSelection::new(4, 4));
    }

    #[test]
    fn committing_replaces_the_composition_or_the_selection() {
        let (client, input_method, buffer) = buffer_of("ab", 1, 1);
        EditCommand::composition_text("x".to_string(), 1).apply(&buffer);
        input_method.take_calls();
        EditCommand::commit_text("xyz".to_string(), 1).apply(&buffer);
        assert_eq!(client.text(), "axyzb");
        assert_eq!(client.selection.get(), TextSelection::new(4, 4));
        assert!(!buffer.has_composition());

        // Without a composition the selection is replaced.
        client.selection.set(TextSelection::new(0, 1));
        EditCommand::commit_text("Q".to_string(), 1).apply(&buffer);
        assert_eq!(client.text(), "Qxyzb");
        assert_eq!(client.selection.get(), TextSelection::new(1, 1));

        // A position of zero puts the caret before the committed text.
        EditCommand::commit_text("--".to_string(), 0).apply(&buffer);
        assert_eq!(client.text(), "Q--xyzb");
        assert_eq!(client.selection.get(), TextSelection::new(1, 1));
    }

    #[test]
    fn finishing_keeps_the_text_and_ends_the_composition() {
        let (client, _input_method, buffer) = buffer_of("", 0, 0);
        EditCommand::composition_text("abc".to_string(), 1).apply(&buffer);
        EditCommand::FinishComposing.apply(&buffer);
        assert_eq!(client.text(), "abc");
        assert_eq!(buffer.composition(), None);
    }

    #[test]
    fn a_key_event_is_dispatched_to_the_view() {
        let (_client, input_method, buffer) = buffer_of("", 0, 0);
        EditCommand::KeyEvent(Some(KeyEventToDispatch::New { action: KEY_EVENT_ACTION_DOWN, code: KEYCODE_ENTER }))
            .apply(&buffer);
        EditCommand::KeyEvent(None).apply(&buffer);
        assert_eq!(input_method.take_calls(), vec![Call::KeyNew(KEY_EVENT_ACTION_DOWN, KEYCODE_ENTER)]);
    }
}
