//! The text an input method edits: the surrounding text and the selection
//! of the client of the input method, and the region that is being
//! composed.
//!
//! Positions are UTF-16 code units, in the text of the framework as in the
//! input method of the system.

use ferroui_base::input::text_input::{TextInputMethodClient, TextSelection};
use std::any::Any;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// `KeyEvent.ACTION_DOWN` and `KeyEvent.ACTION_UP`.
pub(crate) const KEY_EVENT_ACTION_DOWN: i32 = 0;
pub(crate) const KEY_EVENT_ACTION_UP: i32 = 1;
/// `KeyEvent.KEYCODE_ENTER` and `KeyEvent.KEYCODE_FORWARD_DEL`.
pub(crate) const KEYCODE_ENTER: i32 = 66;
pub(crate) const KEYCODE_FORWARD_DEL: i32 = 112;
/// `ExtractedText.FLAG_SINGLE_LINE`.
pub(crate) const EXTRACTED_TEXT_FLAG_SINGLE_LINE: i32 = 1;

/// A key event that is dispatched to the view (`View.DispatchKeyEvent`).
#[derive(Clone)]
pub(crate) enum KeyEventToDispatch {
    /// `new KeyEvent(action, code)`.
    New { action: i32, code: i32 },
    /// `new KeyEvent(downTime, eventTime, action, code, repeat, metaState,
    /// deviceId, scancode, flags)`.
    Full {
        down_time: i64,
        event_time: i64,
        action: i32,
        code: i32,
        repeat: i32,
        meta_state: i32,
        device_id: i32,
        scancode: i32,
        flags: i32,
    },
    /// A key event of the system, as the input method sent it: the Java
    /// object.
    System(Rc<dyn Any>),
}

/// `android.view.inputmethod.ExtractedText`, as the buffer fills it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExtractedText {
    pub flags: i32,
    pub partial_start_offset: i32,
    pub partial_end_offset: i32,
    pub selection_start: i32,
    pub selection_end: i32,
    pub start_offset: i32,
    pub text: String,
}

/// The input method, as the buffer, the commands and the connection see it
/// (`IAndroidInputMethod` of the reference). Where the reference reaches
/// through its `View` and its `IMM` members, the calls it makes on them are
/// members here.
pub(crate) trait IAndroidInputMethod {
    fn client(&self) -> Option<Rc<dyn TextInputMethodClient>>;

    fn is_active(&self) -> bool {
        self.client().is_some()
    }

    fn on_batch_edit_ended(&self);

    /// `View.DispatchKeyEvent`.
    fn dispatch_key_event(&self, key_event: &KeyEventToDispatch);

    /// `IMM.HideSoftInputFromWindow(View.WindowToken, HideSoftInputFlags.ImplicitOnly)`.
    fn hide_soft_input(&self);

    /// `IMM.UpdateSelection(View, ...)`.
    fn update_selection(&self, sel_start: i32, sel_end: i32, candidates_start: i32, candidates_end: i32);

    /// `IMM.UpdateExtractedText(View, token, text)`.
    fn update_extracted_text(&self, token: i32, text: &ExtractedText);
}

/// The top-level of an input connection, and the two functions of the
/// system the connection calls.
pub(crate) trait IInputConnectionTopLevel {
    /// `TopLevelImpl.TextInput`.
    fn text_input(&self, text: &str);

    /// `InputRoot?.FocusManager?.TryMoveFocus(NavigationDirection.Next)`.
    fn try_move_focus_next(&self);

    /// `SystemClock.UptimeMillis`.
    fn uptime_millis(&self) -> i64;

    /// `TextUtils.GetCapsMode`.
    fn get_caps_mode(&self, text: &str, off: i32, req_modes: i32) -> i32;
}

/// The length of a text in UTF-16 code units (`string.Length`).
pub(crate) fn utf16_len(text: &str) -> i32 {
    text.encode_utf16().count() as i32
}

/// `string.Substring(start, length)`.
///
/// # Panics
/// Panics when the range is not inside the text, as the reference fails.
pub(crate) fn utf16_substring(text: &str, start: i32, length: i32) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    if start < 0 || length < 0 || (start + length) as usize > units.len() {
        panic!("Index and length must refer to a location within the string.");
    }
    String::from_utf16_lossy(&units[start as usize..(start + length) as usize])
}

pub(crate) struct TextEditBuffer {
    text_input_method: Weak<dyn IAndroidInputMethod>,
    top_level: Weak<dyn IInputConnectionTopLevel>,
    composition: Cell<Option<TextSelection>>,
}

impl TextEditBuffer {
    pub fn new(text_input_method: Weak<dyn IAndroidInputMethod>, top_level: Weak<dyn IInputConnectionTopLevel>) -> Self {
        Self { text_input_method, top_level, composition: Cell::new(None) }
    }

    fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
        self.text_input_method.upgrade().and_then(|input_method| input_method.client())
    }

    pub fn has_composition(&self) -> bool {
        self.composition().is_some_and(|composition| composition.start != composition.end)
    }

    pub fn selection(&self) -> TextSelection {
        let selection = self.client().map(|client| client.selection()).unwrap_or_default();
        TextSelection::new(selection.start.min(selection.end), selection.start.max(selection.end))
    }

    pub fn set_selection(&self, value: TextSelection) {
        if let Some(client) = self.client() {
            client.set_selection(value);
        }
    }

    pub fn composition(&self) -> Option<TextSelection> {
        self.composition.get()
    }

    pub fn set_composition(&self, value: Option<TextSelection>) {
        self.composition.set(value.map(|v| {
            let length = utf16_len(&self.text());
            let start = v.start.clamp(0, length);
            let end = v.end.clamp(0, length);
            TextSelection::new(start.min(end), start.max(end))
        }));
    }

    pub fn selected_text(&self) -> Option<String> {
        let text = self.client().map(|client| client.surrounding_text()).unwrap_or_default();

        if text.is_empty() {
            return Some(String::new());
        }

        let selection = self.selection();
        let start = selection.start.max(0);
        let end = utf16_len(&text).min(selection.end);

        Some(if start >= end { String::new() } else { utf16_substring(&text, start, end - start) })
    }

    pub fn composing_text(&self) -> Option<String> {
        match self.composition() {
            Some(composition) if self.has_composition() => {
                Some(utf16_substring(&self.text(), composition.start, composition.end - composition.start))
            }
            _ => None,
        }
    }

    pub fn set_composing_text(&self, value: Option<&str>) {
        let value = value.unwrap_or("");
        match self.composition() {
            Some(composition) if self.has_composition() => {
                let start = composition.start;
                self.replace(composition.start, composition.end, value);
                self.set_composition(Some(TextSelection::new(start, start + utf16_len(value))));
            }
            _ => {
                let selection = self.selection();
                self.replace(selection.start, selection.end, value);
                self.set_composition(Some(TextSelection::new(selection.start, selection.start + utf16_len(value))));
            }
        }
    }

    pub fn text(&self) -> String {
        self.client().map(|client| client.surrounding_text()).unwrap_or_default()
    }

    pub fn extracted_text(&self) -> ExtractedText {
        let text = self.text();
        let selection = self.selection();
        ExtractedText {
            flags: if text.contains('\n') { 0 } else { EXTRACTED_TEXT_FLAG_SINGLE_LINE },
            partial_start_offset: -1,
            partial_end_offset: utf16_len(&text),
            selection_start: selection.start,
            selection_end: selection.end,
            start_offset: 0,
            text,
        }
    }

    pub(crate) fn remove(&self, index: i32, length: i32) {
        if let Some(client) = self.client() {
            client.set_selection(TextSelection::new(index, index + length));
            if length > 0 {
                self.dispatch_key_event(Some(&KeyEventToDispatch::New {
                    action: KEY_EVENT_ACTION_DOWN,
                    code: KEYCODE_FORWARD_DEL,
                }));
            }
        }
    }

    pub(crate) fn replace(&self, start: i32, end: i32, text: &str) {
        if let Some(client) = self.client() {
            let real_start = start.min(end);
            let real_end = start.max(end);
            if real_end > real_start {
                client.set_selection(TextSelection::new(real_start, real_end));
                self.dispatch_key_event(Some(&KeyEventToDispatch::New {
                    action: KEY_EVENT_ACTION_DOWN,
                    code: KEYCODE_FORWARD_DEL,
                }));
            }
            if let Some(top_level) = self.top_level.upgrade() {
                top_level.text_input(text);
            }
            let index = real_start + utf16_len(text);
            client.set_selection(TextSelection::new(index, index));
            self.set_composition(None);
        }
    }

    pub(crate) fn dispatch_key_event(&self, key_event: Option<&KeyEventToDispatch>) {
        // The reference dispatches a null event too, which a view answers with false.
        if let (Some(input_method), Some(key_event)) = (self.text_input_method.upgrade(), key_event) {
            input_method.dispatch_key_event(key_event);
        }
    }
}

/// A client, an input method and a top-level for the tests of this
/// directory: the client is a text with a selection that a forward delete
/// and text input edit as a text box does.
#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use ferroui_base::input::text_input::{ContextMenuAction, TextInputMethodClientEvents};
    use ferroui_base::{Rect, Ref, Visual};
    use std::cell::RefCell;

    pub(crate) struct TestClient {
        events: TextInputMethodClientEvents,
        pub text: RefCell<Vec<u16>>,
        pub selection: Cell<TextSelection>,
        pub actions: RefCell<Vec<ContextMenuAction>>,
    }

    impl TestClient {
        pub fn new(text: &str, start: i32, end: i32) -> Rc<TestClient> {
            Rc::new(TestClient {
                events: TextInputMethodClientEvents::new(),
                text: RefCell::new(text.encode_utf16().collect()),
                selection: Cell::new(TextSelection::new(start, end)),
                actions: RefCell::new(Vec::new()),
            })
        }

        pub fn text(&self) -> String {
            String::from_utf16_lossy(&self.text.borrow())
        }

        fn ordered(&self) -> (usize, usize) {
            let selection = self.selection.get();
            let length = self.text.borrow().len() as i32;
            let start = selection.start.min(selection.end).clamp(0, length);
            let end = selection.start.max(selection.end).clamp(0, length);
            (start as usize, end as usize)
        }

        /// What a text box does with a forward delete: the selection goes,
        /// or the unit after the caret.
        pub fn forward_delete(&self) {
            let (start, end) = self.ordered();
            let mut text = self.text.borrow_mut();
            if end > start {
                text.drain(start..end);
            } else if start < text.len() {
                text.remove(start);
            }
            drop(text);
            self.selection.set(TextSelection::new(start as i32, start as i32));
        }

        /// What a text box does with text input: the selection is replaced
        /// and the caret follows the text.
        pub fn input(&self, input: &str) {
            let (start, end) = self.ordered();
            let units: Vec<u16> = input.encode_utf16().collect();
            let caret = (start + units.len()) as i32;
            self.text.borrow_mut().splice(start..end, units);
            self.selection.set(TextSelection::new(caret, caret));
        }
    }

    impl TextInputMethodClient for TestClient {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }

        fn text_view_visual(&self) -> Ref<Visual> {
            unreachable!("the Android input method does not ask for the visual")
        }

        fn supports_preedit(&self) -> bool {
            false
        }

        fn supports_surrounding_text(&self) -> bool {
            true
        }

        fn surrounding_text(&self) -> String {
            self.text()
        }

        fn cursor_rectangle(&self) -> Rect {
            Rect::default()
        }

        fn selection(&self) -> TextSelection {
            self.selection.get()
        }

        fn set_selection(&self, value: TextSelection) {
            self.selection.set(value);
        }

        fn execute_context_menu_action(&self, action: ContextMenuAction) {
            self.actions.borrow_mut().push(action);
        }
    }

    /// What the input method and the top-level were asked to do.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub(crate) enum Call {
        KeyNew(i32, i32),
        KeyFull(i32, i32, i32),
        KeySystem,
        HideSoftInput,
        UpdateSelection(i32, i32, i32, i32),
        UpdateExtractedText(i32, ExtractedText),
        TextInput(String),
        MoveFocusNext,
        BatchEditEnded,
    }

    pub(crate) struct TestInputMethod {
        pub client: RefCell<Option<Rc<TestClient>>>,
        pub calls: RefCell<Vec<Call>>,
    }

    impl TestInputMethod {
        pub fn new(client: Option<Rc<TestClient>>) -> Rc<TestInputMethod> {
            Rc::new(TestInputMethod { client: RefCell::new(client), calls: RefCell::new(Vec::new()) })
        }

        pub fn take_calls(&self) -> Vec<Call> {
            std::mem::take(&mut *self.calls.borrow_mut())
        }
    }

    impl IAndroidInputMethod for TestInputMethod {
        fn client(&self) -> Option<Rc<dyn TextInputMethodClient>> {
            self.client.borrow().clone().map(|client| client as Rc<dyn TextInputMethodClient>)
        }

        fn on_batch_edit_ended(&self) {
            self.calls.borrow_mut().push(Call::BatchEditEnded);
        }

        fn dispatch_key_event(&self, key_event: &KeyEventToDispatch) {
            let client = self.client.borrow().clone();
            match key_event {
                KeyEventToDispatch::New { action, code } => {
                    self.calls.borrow_mut().push(Call::KeyNew(*action, *code));
                    if let (Some(client), KEY_EVENT_ACTION_DOWN, KEYCODE_FORWARD_DEL) = (client, *action, *code) {
                        client.forward_delete();
                    }
                }
                KeyEventToDispatch::Full { action, code, flags, .. } => {
                    self.calls.borrow_mut().push(Call::KeyFull(*action, *code, *flags));
                }
                KeyEventToDispatch::System(_) => self.calls.borrow_mut().push(Call::KeySystem),
            }
        }

        fn hide_soft_input(&self) {
            self.calls.borrow_mut().push(Call::HideSoftInput);
        }

        fn update_selection(&self, sel_start: i32, sel_end: i32, candidates_start: i32, candidates_end: i32) {
            self.calls.borrow_mut().push(Call::UpdateSelection(sel_start, sel_end, candidates_start, candidates_end));
        }

        fn update_extracted_text(&self, token: i32, text: &ExtractedText) {
            self.calls.borrow_mut().push(Call::UpdateExtractedText(token, text.clone()));
        }
    }

    impl IInputConnectionTopLevel for TestInputMethod {
        fn text_input(&self, text: &str) {
            self.calls.borrow_mut().push(Call::TextInput(text.to_string()));
            let client = self.client.borrow().clone();
            if let Some(client) = client {
                client.input(text);
            }
        }

        fn try_move_focus_next(&self) {
            self.calls.borrow_mut().push(Call::MoveFocusNext);
        }

        fn uptime_millis(&self) -> i64 {
            777
        }

        fn get_caps_mode(&self, text: &str, off: i32, req_modes: i32) -> i32 {
            // Not the function of the system: something that shows what it was given.
            utf16_len(text) * 1000 + off * 10 + req_modes
        }
    }

    /// A buffer over a client with `text` and the selection `start..end`.
    pub(crate) fn buffer_of(text: &str, start: i32, end: i32) -> (Rc<TestClient>, Rc<TestInputMethod>, TextEditBuffer) {
        let client = TestClient::new(text, start, end);
        let input_method = TestInputMethod::new(Some(client.clone()));
        let as_input_method: Rc<dyn IAndroidInputMethod> = input_method.clone();
        let as_top_level: Rc<dyn IInputConnectionTopLevel> = input_method.clone();
        let buffer = TextEditBuffer::new(Rc::downgrade(&as_input_method), Rc::downgrade(&as_top_level));
        (client, input_method, buffer)
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the buffer.
    use super::test_support::*;
    use super::*;

    #[test]
    fn the_selection_is_ordered_and_the_text_is_that_of_the_client() {
        let (client, _input_method, buffer) = buffer_of("hello", 4, 1);
        assert_eq!(buffer.selection(), TextSelection::new(1, 4));
        assert_eq!(buffer.text(), "hello");
        assert_eq!(buffer.selected_text().as_deref(), Some("ell"));

        buffer.set_selection(TextSelection::new(2, 2));
        assert_eq!(client.selection.get(), TextSelection::new(2, 2));
        assert_eq!(buffer.selected_text().as_deref(), Some(""));
    }

    #[test]
    fn without_a_client_the_buffer_is_empty_and_does_nothing() {
        let input_method = TestInputMethod::new(None);
        let as_input_method: Rc<dyn IAndroidInputMethod> = input_method.clone();
        let as_top_level: Rc<dyn IInputConnectionTopLevel> = input_method.clone();
        let buffer = TextEditBuffer::new(Rc::downgrade(&as_input_method), Rc::downgrade(&as_top_level));

        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.selection(), TextSelection::default());
        assert_eq!(buffer.selected_text().as_deref(), Some(""));
        buffer.replace(0, 0, "x");
        buffer.remove(0, 1);
        assert!(input_method.take_calls().is_empty());
    }

    #[test]
    fn the_composition_is_clamped_to_the_text_and_ordered() {
        let (_client, _input_method, buffer) = buffer_of("hello", 0, 0);
        assert!(!buffer.has_composition());
        assert_eq!(buffer.composing_text(), None);

        buffer.set_composition(Some(TextSelection::new(9, 2)));
        assert_eq!(buffer.composition(), Some(TextSelection::new(2, 5)));
        assert!(buffer.has_composition());
        assert_eq!(buffer.composing_text().as_deref(), Some("llo"));

        // An empty region is no composition.
        buffer.set_composition(Some(TextSelection::new(3, 3)));
        assert!(!buffer.has_composition());
        buffer.set_composition(None);
        assert_eq!(buffer.composition(), None);
    }

    #[test]
    fn composing_text_replaces_the_selection_and_then_the_composition() {
        let (client, input_method, buffer) = buffer_of("ab", 1, 1);
        buffer.set_composing_text(Some("x"));
        assert_eq!(client.text(), "axb");
        assert_eq!(buffer.composition(), Some(TextSelection::new(1, 2)));
        assert_eq!(input_method.take_calls(), vec![Call::TextInput("x".to_string())]);

        // The second text of the composition replaces the first: the region is selected,
        // deleted with a forward delete, and the new text is input.
        buffer.set_composing_text(Some("xyz"));
        assert_eq!(client.text(), "axyzb");
        assert_eq!(buffer.composition(), Some(TextSelection::new(1, 4)));
        assert_eq!(client.selection.get(), TextSelection::new(4, 4));
        assert_eq!(
            input_method.take_calls(),
            vec![Call::KeyNew(KEY_EVENT_ACTION_DOWN, KEYCODE_FORWARD_DEL), Call::TextInput("xyz".to_string())]
        );

        // No text ends the composition and removes its text.
        buffer.set_composing_text(None);
        assert_eq!(client.text(), "ab");
        assert!(!buffer.has_composition());
    }

    #[test]
    fn positions_are_utf16_code_units() {
        // The emoji is two units.
        let (client, _input_method, buffer) = buffer_of("a\u{1f600}b", 1, 3);
        assert_eq!(buffer.selected_text().as_deref(), Some("\u{1f600}"));
        assert_eq!(buffer.extracted_text().partial_end_offset, 4);
        buffer.replace(1, 3, "é");
        assert_eq!(client.text(), "aéb");
        assert_eq!(client.selection.get(), TextSelection::new(2, 2));
    }

    #[test]
    fn the_extracted_text_is_the_whole_text_with_the_selection() {
        let (_client, _input_method, single) = buffer_of("one", 1, 2);
        assert_eq!(
            single.extracted_text(),
            ExtractedText {
                flags: EXTRACTED_TEXT_FLAG_SINGLE_LINE,
                partial_start_offset: -1,
                partial_end_offset: 3,
                selection_start: 1,
                selection_end: 2,
                start_offset: 0,
                text: "one".to_string(),
            }
        );
        let (_client, _input_method, multi) = buffer_of("one\ntwo", 0, 0);
        assert_eq!(multi.extracted_text().flags, 0);
    }

    #[test]
    fn remove_selects_and_deletes_only_a_length() {
        let (client, input_method, buffer) = buffer_of("hello", 0, 0);
        buffer.remove(1, 3);
        assert_eq!(client.text(), "ho");
        assert_eq!(input_method.take_calls(), vec![Call::KeyNew(KEY_EVENT_ACTION_DOWN, KEYCODE_FORWARD_DEL)]);

        buffer.remove(1, 0);
        assert_eq!(client.text(), "ho");
        assert_eq!(client.selection.get(), TextSelection::new(1, 1));
        assert!(input_method.take_calls().is_empty());
    }
}
