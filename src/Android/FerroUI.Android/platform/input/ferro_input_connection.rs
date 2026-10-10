//! The connection an input method of the system edits the text through.
//!
//! The connection of the system is a class of the Java layer
//! (`FerroInputConnection`, an `InputConnection`), which forwards every
//! call the reference answers with more than a constant; this is the object
//! behind it. The members the reference answers with `false` for every
//! argument (`ClearMetaKeyStates`, `CommitCompletion`, `CommitContent`,
//! `CommitCorrection`, `PerformPrivateCommand`, `ReportFullscreenMode`,
//! `RequestCursorUpdates`) and its `Handler`, which is null, are answered by
//! the Java class.
//!
//! The reference guards the batch level and the queue against other
//! threads; an input connection without a handler of its own is called on
//! the main thread, which is where this object lives.

use super::edit_command::EditCommand;
use super::text_edit_buffer::{
    utf16_len, utf16_substring, ExtractedText, IAndroidInputMethod, IInputConnectionTopLevel, KeyEventToDispatch,
    TextEditBuffer, KEYCODE_ENTER, KEY_EVENT_ACTION_DOWN, KEY_EVENT_ACTION_UP,
};
use ferroui_base::input::text_input::{ContextMenuAction, TextSelection};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicI64, Ordering};

thread_local! {
    static CONNECTIONS: RefCell<HashMap<i64, Weak<FerroInputConnection>>> = RefCell::new(HashMap::new());
}

/// `EditorInfo.IME_ACTION_NEXT` and `EditorInfo.IME_ACTION_DONE`.
pub(crate) const IME_ACTION_NEXT: i32 = 5;
pub(crate) const IME_ACTION_DONE: i32 = 6;
/// `InputConnection.GET_EXTRACTED_TEXT_MONITOR`.
pub(crate) const GET_EXTRACTED_TEXT_MONITOR: i32 = 1;
/// `KeyEvent.FLAG_SOFT_KEYBOARD | KeyEvent.FLAG_KEEP_TOUCH_MODE | KeyEvent.FLAG_EDITOR_ACTION`.
pub(crate) const EDITOR_ACTION_KEY_FLAGS: i32 = 0x2 | 0x4 | 0x10;
/// `android.R.id.selectAll`, `cut`, `copy` and `paste`.
pub(crate) const ID_SELECT_ALL: i32 = 0x0102_001f;
pub(crate) const ID_CUT: i32 = 0x0102_0020;
pub(crate) const ID_COPY: i32 = 0x0102_0021;
pub(crate) const ID_PASTE: i32 = 0x0102_0022;

pub(crate) struct FerroInputConnection {
    toplevel: Weak<dyn IInputConnectionTopLevel>,
    input_method: Weak<dyn IAndroidInputMethod>,
    edit_buffer: TextEditBuffer,
    command_queue: RefCell<VecDeque<EditCommand>>,

    batch_level: Cell<i32>,
    extracted_text_token: Cell<i32>,
    is_in_monitor_mode: Cell<bool>,
    is_in_update: Cell<bool>,
}

impl FerroInputConnection {
    pub fn new(
        toplevel: Weak<dyn IInputConnectionTopLevel>,
        input_method: Weak<dyn IAndroidInputMethod>,
    ) -> Rc<FerroInputConnection> {
        Rc::new(FerroInputConnection {
            edit_buffer: TextEditBuffer::new(input_method.clone(), toplevel.clone()),
            toplevel,
            input_method,
            command_queue: RefCell::new(VecDeque::new()),
            batch_level: Cell::new(0),
            extracted_text_token: Cell::new(0),
            is_in_monitor_mode: Cell::new(false),
            is_in_update: Cell::new(false),
        })
    }

    /// The number the connection of the Java layer calls back with; the
    /// connection stays registered for as long as it lives.
    pub(crate) fn register(self: &Rc<Self>) -> i64 {
        static NEXT: AtomicI64 = AtomicI64::new(1);
        let handle = NEXT.fetch_add(1, Ordering::Relaxed);
        CONNECTIONS.with(|connections| {
            let mut connections = connections.borrow_mut();
            connections.retain(|_, connection| connection.strong_count() > 0);
            connections.insert(handle, Rc::downgrade(self));
        });
        handle
    }

    /// The connection registered under `handle`, while it lives.
    pub(crate) fn from_handle(handle: i64) -> Option<Rc<FerroInputConnection>> {
        CONNECTIONS.with(|connections| connections.borrow().get(&handle).and_then(Weak::upgrade))
    }

    pub fn extracted_text_token(&self) -> i32 {
        self.extracted_text_token.get()
    }

    pub fn input_method(&self) -> Option<Rc<dyn IAndroidInputMethod>> {
        self.input_method.upgrade()
    }

    pub fn toplevel(&self) -> Option<Rc<dyn IInputConnectionTopLevel>> {
        self.toplevel.upgrade()
    }

    pub fn is_in_batch_edit(&self) -> bool {
        self.batch_level.get() > 0
    }

    pub fn is_in_monitor_mode(&self) -> bool {
        self.is_in_monitor_mode.get()
    }

    pub fn edit_buffer(&self) -> &TextEditBuffer {
        &self.edit_buffer
    }

    pub fn is_in_update(&self) -> bool {
        self.is_in_update.get()
    }

    pub fn set_is_in_update(&self, value: bool) {
        self.is_in_update.set(value);
    }

    /// `InputMethod.IsActive`; an input method that is gone is not active.
    fn is_active(&self) -> bool {
        self.input_method().is_some_and(|input_method| input_method.is_active())
    }

    pub(crate) fn update_state(&self) {
        let Some(input_method) = self.input_method() else {
            return;
        };
        let selection = self.edit_buffer.selection();

        if self.is_in_monitor_mode() && input_method.client().is_some() {
            input_method.update_extracted_text(self.extracted_text_token(), &self.edit_buffer.extracted_text());
        }

        let composition = match self.edit_buffer.composition() {
            Some(composition) if self.edit_buffer.has_composition() => composition,
            _ => TextSelection::new(-1, -1),
        };
        input_method.update_selection(selection.start, selection.end, composition.start, composition.end);
    }

    pub fn set_composing_region(&self, start: i32, end: i32) -> bool {
        if self.is_active() {
            self.queue_command(EditCommand::composition_region(start, end));
        }
        self.is_active()
    }

    pub fn set_composing_text(&self, text: Option<&str>, new_cursor_position: i32) -> bool {
        let Some(text) = text else {
            return false;
        };

        if self.is_active() {
            self.queue_command(EditCommand::composition_text(text.to_string(), new_cursor_position));
        }

        self.is_active()
    }

    pub fn set_selection(&self, start: i32, end: i32) -> bool {
        if self.is_active() {
            if self.is_in_update() {
                EditCommand::selection(start, end).apply(&self.edit_buffer);
            } else {
                self.queue_command(EditCommand::selection(start, end));
            }
        }

        self.is_active()
    }

    pub fn begin_batch_edit(&self) -> bool {
        self.batch_level.set(self.batch_level.get() + 1);
        self.is_active()
    }

    pub fn end_batch_edit(&self) -> bool {
        self.batch_level.set(self.batch_level.get() - 1);

        if !self.is_in_batch_edit() {
            self.set_is_in_update(true);
            loop {
                // A command dispatches key events and text to the view, which may call the
                // connection again: nothing of the queue is borrowed while it is applied.
                let command = self.command_queue.borrow_mut().pop_front();
                let Some(command) = command else {
                    break;
                };
                command.apply(&self.edit_buffer);
            }
            self.set_is_in_update(false);
        }

        self.update_state();
        self.is_in_batch_edit()
    }

    pub fn commit_text(&self, text: Option<&str>, new_cursor_position: i32) -> bool {
        let has_client = self.input_method().is_some_and(|input_method| input_method.client().is_some());
        let (true, Some(text)) = (has_client, text) else {
            return false;
        };

        if self.is_active() {
            self.queue_command(EditCommand::commit_text(text.to_string(), new_cursor_position));
        }

        self.is_active()
    }

    pub fn delete_surrounding_text(&self, before_length: i32, after_length: i32) -> bool {
        if self.is_active() {
            self.queue_command(EditCommand::delete_region(before_length, after_length));
        }

        self.is_active()
    }

    pub fn perform_editor_action(&self, action_code: i32) -> bool {
        match action_code {
            IME_ACTION_DONE => {
                if let Some(input_method) = self.input_method() {
                    input_method.hide_soft_input();
                }
            }
            IME_ACTION_NEXT => {
                if let Some(toplevel) = self.toplevel() {
                    toplevel.try_move_focus_next();
                }
            }
            _ => {}
        }

        let event_time = self.toplevel().map_or(0, |toplevel| toplevel.uptime_millis());
        let key_event = |action: i32| KeyEventToDispatch::Full {
            down_time: event_time,
            event_time,
            action,
            code: KEYCODE_ENTER,
            repeat: 0,
            meta_state: 0,
            device_id: 0,
            scancode: 0,
            flags: EDITOR_ACTION_KEY_FLAGS,
        };
        self.send_key_event(Some(key_event(KEY_EVENT_ACTION_DOWN)));
        self.send_key_event(Some(key_event(KEY_EVENT_ACTION_UP)));

        self.is_active()
    }

    /// `request_token` is the token of the request; `None` without a
    /// request.
    pub fn get_extracted_text(&self, request_token: Option<i32>, flags: i32) -> Option<ExtractedText> {
        self.is_in_monitor_mode.set(flags & GET_EXTRACTED_TEXT_MONITOR != 0);

        self.extracted_text_token.set(if self.is_in_monitor_mode() {
            request_token.unwrap_or(0)
        } else {
            self.extracted_text_token()
        });

        if !self.is_active() {
            return None;
        }

        Some(self.edit_buffer.extracted_text())
    }

    pub fn perform_context_menu_action(&self, id: i32) -> bool {
        let Some(client) = self.input_method().and_then(|input_method| input_method.client()) else {
            return false;
        };

        match id {
            ID_SELECT_ALL => {
                client.execute_context_menu_action(ContextMenuAction::SelectAll);
                return true;
            }
            ID_CUT => {
                client.execute_context_menu_action(ContextMenuAction::Cut);
                return true;
            }
            ID_COPY => {
                client.execute_context_menu_action(ContextMenuAction::Copy);
                return true;
            }
            ID_PASTE => {
                client.execute_context_menu_action(ContextMenuAction::Paste);
                return true;
            }
            _ => {}
        }
        self.is_active()
    }

    pub fn close_connection(&self) {
        self.command_queue.borrow_mut().clear();
        self.batch_level.set(0);
    }

    pub fn delete_surrounding_text_in_code_points(&self, before_length: i32, after_length: i32) -> bool {
        if self.is_active() {
            self.queue_command(EditCommand::delete_region_in_code_points(before_length, after_length));
        }

        self.is_active()
    }

    pub fn finish_composing_text(&self) -> bool {
        if self.is_active() {
            self.queue_command(EditCommand::FinishComposing);
        }

        self.is_active()
    }

    pub fn get_cursor_caps_mode(&self, req_modes: i32) -> i32 {
        self.toplevel().map_or(0, |toplevel| {
            toplevel.get_caps_mode(&self.edit_buffer.text(), self.edit_buffer.selection().start, req_modes)
        })
    }

    pub fn get_selected_text_formatted(&self, _flags: i32) -> Option<String> {
        Some(self.edit_buffer.selected_text().unwrap_or_default())
    }

    pub fn get_text_after_cursor_formatted(&self, n: i32, _flags: i32) -> Option<String> {
        let text = self.edit_buffer.text();
        let length = utf16_len(&text);
        let end = self.edit_buffer.selection().end.min(length);
        Self::safe_substring(&text, end, n.min(length - end))
    }

    pub fn get_text_before_cursor_formatted(&self, n: i32, _flags: i32) -> Option<String> {
        let selection_start = self.edit_buffer.selection().start;
        // The reference subtracts without looking for an overflow; an input method may ask
        // for "everything" with the largest number.
        let start = 0.max(selection_start.saturating_sub(n));
        let length = selection_start - start;
        Self::safe_substring(&self.edit_buffer.text(), start, length)
    }

    pub fn send_key_event(&self, e: Option<KeyEventToDispatch>) -> bool {
        self.queue_command(EditCommand::KeyEvent(e));

        true
    }

    fn queue_command(&self, command: EditCommand) {
        self.begin_batch_edit();

        self.command_queue.borrow_mut().push_back(command);

        self.end_batch_edit();
    }

    fn safe_substring(text: &str, start: i32, length: i32) -> Option<String> {
        if utf16_len(text) < start + length {
            None
        } else {
            // A negative start or length makes the reference fail; the system asks for
            // neither.
            Some(utf16_substring(text, start.max(0), length.max(0)))
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of the connection.
    use super::super::text_edit_buffer::test_support::*;
    use super::super::text_edit_buffer::EXTRACTED_TEXT_FLAG_SINGLE_LINE;
    use super::*;

    fn connection(text: &str, start: i32, end: i32) -> (Rc<TestClient>, Rc<TestInputMethod>, Rc<FerroInputConnection>) {
        let client = TestClient::new(text, start, end);
        let input_method = TestInputMethod::new(Some(client.clone()));
        let as_input_method: Rc<dyn IAndroidInputMethod> = input_method.clone();
        let as_top_level: Rc<dyn IInputConnectionTopLevel> = input_method.clone();
        let connection = FerroInputConnection::new(Rc::downgrade(&as_top_level), Rc::downgrade(&as_input_method));
        (client, input_method, connection)
    }

    #[test]
    fn a_command_outside_a_batch_is_applied_at_once_and_the_selection_is_reported() {
        let (client, input_method, connection) = connection("", 0, 0);
        assert!(connection.commit_text(Some("hi"), 1));
        assert_eq!(client.text(), "hi");
        assert_eq!(
            input_method.take_calls(),
            vec![Call::TextInput("hi".to_string()), Call::UpdateSelection(2, 2, -1, -1)]
        );
    }

    #[test]
    fn commands_of_a_batch_are_applied_when_the_outermost_batch_ends() {
        let (client, input_method, connection) = connection("", 0, 0);
        assert!(connection.begin_batch_edit());
        connection.begin_batch_edit();
        connection.set_composing_text(Some("ka"), 1);
        connection.set_composing_text(Some("kan"), 1);
        assert_eq!(client.text(), "");
        assert!(connection.is_in_batch_edit());
        // The inner batch ends: still in a batch.
        assert!(connection.end_batch_edit());
        assert_eq!(client.text(), "");
        assert!(!connection.end_batch_edit());
        assert_eq!(client.text(), "kan");
        assert_eq!(connection.edit_buffer().composition(), Some(TextSelection::new(0, 3)));
        // The composing region is reported with the selection.
        assert_eq!(input_method.take_calls().last(), Some(&Call::UpdateSelection(3, 3, 0, 3)));

        connection.finish_composing_text();
        assert_eq!(input_method.take_calls(), vec![Call::UpdateSelection(3, 3, -1, -1)]);
        assert_eq!(client.text(), "kan");
    }

    #[test]
    fn without_a_client_nothing_is_queued_and_the_answers_are_false() {
        let (_client, input_method, connection) = connection("abc", 1, 1);
        *input_method.client.borrow_mut() = None;
        assert!(!connection.set_composing_region(0, 1));
        assert!(!connection.set_composing_text(Some("x"), 1));
        assert!(!connection.set_selection(0, 0));
        assert!(!connection.commit_text(Some("x"), 1));
        assert!(!connection.delete_surrounding_text(1, 1));
        assert!(!connection.delete_surrounding_text_in_code_points(1, 1));
        assert!(!connection.finish_composing_text());
        assert!(!connection.perform_context_menu_action(ID_COPY));
        assert_eq!(connection.get_extracted_text(Some(3), 0), None);
        assert!(input_method.take_calls().is_empty());
    }

    #[test]
    fn a_text_that_is_null_is_refused() {
        let (_client, _input_method, connection) = connection("abc", 1, 1);
        assert!(!connection.set_composing_text(None, 1));
        assert!(!connection.commit_text(None, 1));
    }

    #[test]
    fn the_text_around_the_cursor_is_cut_to_what_exists() {
        let (_client, _input_method, connection) = connection("hello world", 5, 5);
        assert_eq!(connection.get_text_before_cursor_formatted(3, 0).as_deref(), Some("llo"));
        assert_eq!(connection.get_text_before_cursor_formatted(100, 0).as_deref(), Some("hello"));
        assert_eq!(connection.get_text_before_cursor_formatted(i32::MAX, 0).as_deref(), Some("hello"));
        assert_eq!(connection.get_text_after_cursor_formatted(3, 0).as_deref(), Some(" wo"));
        assert_eq!(connection.get_text_after_cursor_formatted(100, 0).as_deref(), Some(" world"));
        assert_eq!(connection.get_selected_text_formatted(0).as_deref(), Some(""));

        connection.set_selection(0, 5);
        assert_eq!(connection.get_selected_text_formatted(0).as_deref(), Some("hello"));
        assert_eq!(connection.get_cursor_caps_mode(7), 11 * 1000 + 7);
    }

    #[test]
    fn the_extracted_text_is_monitored_with_the_token_of_the_request() {
        let (_client, input_method, connection) = connection("abc", 1, 2);
        let extracted = connection.get_extracted_text(Some(42), GET_EXTRACTED_TEXT_MONITOR).expect("active");
        assert_eq!(extracted.text, "abc");
        assert_eq!((extracted.selection_start, extracted.selection_end), (1, 2));
        assert_eq!(extracted.flags, EXTRACTED_TEXT_FLAG_SINGLE_LINE);
        assert!(connection.is_in_monitor_mode());
        assert_eq!(connection.extracted_text_token(), 42);

        connection.update_state();
        let calls = input_method.take_calls();
        assert_eq!(calls.len(), 2);
        assert!(matches!(&calls[0], Call::UpdateExtractedText(42, text) if text.text == "abc"));
        assert_eq!(calls[1], Call::UpdateSelection(1, 2, -1, -1));

        // A request that does not monitor ends the monitoring and keeps the token.
        connection.get_extracted_text(Some(7), 0);
        assert!(!connection.is_in_monitor_mode());
        assert_eq!(connection.extracted_text_token(), 42);
        connection.update_state();
        assert_eq!(input_method.take_calls(), vec![Call::UpdateSelection(1, 2, -1, -1)]);
    }

    #[test]
    fn an_editor_action_hides_the_keyboard_or_moves_the_focus_and_sends_enter() {
        let (_client, input_method, connection) = connection("abc", 3, 3);
        assert!(connection.perform_editor_action(IME_ACTION_DONE));
        assert_eq!(
            input_method.take_calls(),
            vec![
                Call::HideSoftInput,
                Call::KeyFull(KEY_EVENT_ACTION_DOWN, KEYCODE_ENTER, EDITOR_ACTION_KEY_FLAGS),
                Call::UpdateSelection(3, 3, -1, -1),
                Call::KeyFull(KEY_EVENT_ACTION_UP, KEYCODE_ENTER, EDITOR_ACTION_KEY_FLAGS),
                Call::UpdateSelection(3, 3, -1, -1),
            ]
        );

        connection.perform_editor_action(IME_ACTION_NEXT);
        assert_eq!(input_method.take_calls().first(), Some(&Call::MoveFocusNext));

        // Another action only sends the key.
        connection.perform_editor_action(2);
        assert_eq!(input_method.take_calls().len(), 4);
    }

    #[test]
    fn the_context_menu_actions_reach_the_client() {
        let (client, _input_method, connection) = connection("abc", 0, 0);
        assert!(connection.perform_context_menu_action(ID_SELECT_ALL));
        assert!(connection.perform_context_menu_action(ID_CUT));
        assert!(connection.perform_context_menu_action(ID_COPY));
        assert!(connection.perform_context_menu_action(ID_PASTE));
        // Another id is not an action, and the answer is whether the input method is active.
        assert!(connection.perform_context_menu_action(1));
        assert_eq!(
            *client.actions.borrow(),
            vec![ContextMenuAction::SelectAll, ContextMenuAction::Cut, ContextMenuAction::Copy, ContextMenuAction::Paste]
        );
    }

    #[test]
    fn a_selection_during_an_update_is_applied_without_the_queue() {
        let (client, input_method, connection) = connection("hello", 0, 0);
        connection.set_is_in_update(true);
        assert!(connection.set_selection(4, 2));
        assert_eq!(client.selection.get(), TextSelection::new(2, 4));
        // No batch ended: nothing was reported.
        assert!(input_method.take_calls().is_empty());
    }

    #[test]
    fn closing_drops_what_is_queued() {
        let (client, _input_method, connection) = connection("", 0, 0);
        connection.begin_batch_edit();
        connection.commit_text(Some("lost"), 1);
        connection.close_connection();
        assert!(!connection.is_in_batch_edit());
        connection.begin_batch_edit();
        connection.end_batch_edit();
        assert_eq!(client.text(), "");
    }

    #[test]
    fn a_key_event_of_the_input_method_is_dispatched_in_order() {
        let (client, input_method, connection) = connection("", 0, 0);
        connection.begin_batch_edit();
        connection.commit_text(Some("a"), 1);
        assert!(connection.send_key_event(Some(KeyEventToDispatch::System(Rc::new(())))));
        connection.commit_text(Some("b"), 1);
        connection.end_batch_edit();
        assert_eq!(client.text(), "ab");
        // Every end of a batch reports the selection, the inner ones the one from before.
        assert_eq!(
            input_method.take_calls(),
            vec![
                Call::UpdateSelection(0, 0, -1, -1),
                Call::UpdateSelection(0, 0, -1, -1),
                Call::UpdateSelection(0, 0, -1, -1),
                Call::TextInput("a".to_string()),
                Call::KeySystem,
                Call::TextInput("b".to_string()),
                Call::UpdateSelection(2, 2, -1, -1)
            ]
        );
    }
}
