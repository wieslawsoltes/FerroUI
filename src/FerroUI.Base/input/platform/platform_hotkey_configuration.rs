use crate::input::{Key, KeyGesture, KeyModifiers};

/// The key gestures of the platform for common editing and navigation
/// commands.
#[derive(Clone, Debug, PartialEq)]
pub struct PlatformHotkeyConfiguration {
    pub command_modifiers: KeyModifiers,
    pub whole_word_text_action_modifiers: KeyModifiers,
    pub selection_modifiers: KeyModifiers,
    pub copy: Vec<KeyGesture>,
    pub cut: Vec<KeyGesture>,
    pub paste: Vec<KeyGesture>,
    pub undo: Vec<KeyGesture>,
    pub redo: Vec<KeyGesture>,
    pub select_all: Vec<KeyGesture>,
    pub move_cursor_to_the_start_of_line: Vec<KeyGesture>,
    pub move_cursor_to_the_end_of_line: Vec<KeyGesture>,
    pub move_cursor_to_the_start_of_document: Vec<KeyGesture>,
    pub move_cursor_to_the_end_of_document: Vec<KeyGesture>,
    pub move_cursor_to_the_start_of_line_with_selection: Vec<KeyGesture>,
    pub move_cursor_to_the_end_of_line_with_selection: Vec<KeyGesture>,
    pub move_cursor_to_the_start_of_document_with_selection: Vec<KeyGesture>,
    pub move_cursor_to_the_end_of_document_with_selection: Vec<KeyGesture>,
    pub open_context_menu: Vec<KeyGesture>,
    pub back: Vec<KeyGesture>,
    pub page_up: Vec<KeyGesture>,
    pub page_down: Vec<KeyGesture>,
    pub page_right: Vec<KeyGesture>,
    pub page_left: Vec<KeyGesture>,
}

impl Default for PlatformHotkeyConfiguration {
    fn default() -> Self {
        Self::new(KeyModifiers::CONTROL)
    }
}

impl PlatformHotkeyConfiguration {
    /// Creates a configuration for the given command modifiers, with Shift
    /// as the selection modifier and Control as the whole-word modifier.
    pub fn new(command_modifiers: KeyModifiers) -> Self {
        Self::with_modifiers(command_modifiers, KeyModifiers::SHIFT, KeyModifiers::CONTROL)
    }

    /// The configuration of Apple platforms (macOS, iOS, iPadOS and the browser on them):
    /// Command is the command modifier, Option the whole-word modifier, and Command with the
    /// left and right arrows moves to the start and the end of the line.
    pub fn apple() -> Self {
        let mut hotkeys = Self::with_modifiers(KeyModifiers::META, KeyModifiers::SHIFT, KeyModifiers::ALT);
        let (command, selection) = (hotkeys.command_modifiers, hotkeys.selection_modifiers);
        hotkeys.move_cursor_to_the_start_of_line.push(KeyGesture::new(Key::Left, command));
        hotkeys.move_cursor_to_the_start_of_line_with_selection.push(KeyGesture::new(Key::Left, command | selection));
        hotkeys.move_cursor_to_the_end_of_line.push(KeyGesture::new(Key::Right, command));
        hotkeys.move_cursor_to_the_end_of_line_with_selection.push(KeyGesture::new(Key::Right, command | selection));
        hotkeys
    }

    /// Creates a configuration for the given modifiers.
    pub fn with_modifiers(
        command_modifiers: KeyModifiers,
        selection_modifiers: KeyModifiers,
        whole_word_text_action_modifiers: KeyModifiers,
    ) -> Self {
        let g = KeyGesture::new;
        let k = KeyGesture::from_key;

        Self {
            command_modifiers,
            selection_modifiers,
            whole_word_text_action_modifiers,
            copy: vec![g(Key::C, command_modifiers), g(Key::Insert, KeyModifiers::CONTROL)],
            cut: vec![g(Key::X, command_modifiers)],
            paste: vec![g(Key::V, command_modifiers), g(Key::Insert, KeyModifiers::SHIFT)],
            undo: vec![g(Key::Z, command_modifiers)],
            redo: vec![g(Key::Y, command_modifiers), g(Key::Z, command_modifiers | selection_modifiers)],
            select_all: vec![g(Key::A, command_modifiers)],
            move_cursor_to_the_start_of_line: vec![k(Key::Home)],
            move_cursor_to_the_end_of_line: vec![k(Key::End)],
            move_cursor_to_the_start_of_document: vec![g(Key::Home, command_modifiers)],
            move_cursor_to_the_end_of_document: vec![g(Key::End, command_modifiers)],
            move_cursor_to_the_start_of_line_with_selection: vec![g(Key::Home, selection_modifiers)],
            move_cursor_to_the_end_of_line_with_selection: vec![g(Key::End, selection_modifiers)],
            move_cursor_to_the_start_of_document_with_selection: vec![g(
                Key::Home,
                command_modifiers | selection_modifiers,
            )],
            move_cursor_to_the_end_of_document_with_selection: vec![g(
                Key::End,
                command_modifiers | selection_modifiers,
            )],
            open_context_menu: vec![k(Key::Apps)],
            back: vec![g(Key::Left, KeyModifiers::ALT)],
            page_left: vec![g(Key::PageUp, KeyModifiers::SHIFT)],
            page_right: vec![g(Key::PageDown, KeyModifiers::SHIFT)],
            page_up: vec![k(Key::PageUp)],
            page_down: vec![k(Key::PageDown)],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::platform::KeyGestureFormatInfo;

    #[test]
    fn apple_platforms_use_command_and_option() {
        let hotkeys = PlatformHotkeyConfiguration::apple();
        assert_eq!(hotkeys.command_modifiers, KeyModifiers::META);
        assert_eq!(hotkeys.whole_word_text_action_modifiers, KeyModifiers::ALT);
        assert!(hotkeys.copy.contains(&KeyGesture::new(Key::C, KeyModifiers::META)));
        assert!(hotkeys.move_cursor_to_the_start_of_line.contains(&KeyGesture::new(Key::Left, KeyModifiers::META)));
        assert!(hotkeys
            .move_cursor_to_the_end_of_line_with_selection
            .contains(&KeyGesture::new(Key::Right, KeyModifiers::META | KeyModifiers::SHIFT)));

        let format = KeyGestureFormatInfo::apple();
        assert_eq!(format.meta(), "\u{2318}");
        assert_ne!(format, KeyGestureFormatInfo::default());
    }
}
