// Not from upstream: the CoreText host of a client of the exact-edit connection
// (`TextInputMethodClient::input_method_connection` returns `Some`, decision D5 of
// `docs/porting/winui-rich-text.md`). CoreText's TextUpdating carries an exact
// replacement (range, text, new selection); such a client applies it through its
// connection and acknowledges it only when the client accepted exactly that
// edit. Every other client keeps the adapter of the reference above.
impl WinUITextInputMethod {
    fn sync_from_connection_client(&self, notify_server: bool) {
        let generation = self.client_generation.get();
        let composition_generation = self.composition_generation.get();
        let Some(client) = self.client.borrow().clone() else {
            self.ime_text.borrow_mut().clear();
            self.ime_selection.set(range(0, 0));
            return;
        };
        // Reading the installed client's current visible state remains valid
        // during a manager handoff; editing requires its captured connection.
        // Never downgrade an exact client to the committed-only surrounding text.
        let Some(editing) = client.input_method_editing() else {
            return;
        };
        let Ok(state) = editing.edit_state() else {
            return;
        };
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        *self.ime_text.borrow_mut() = state.text.encode_utf16().collect();
        let text_length = length(&self.ime_text.borrow());
        let sel = state.selection;
        let selection = TextSelection::new(clamp(sel.start, 0, text_length), clamp(sel.end, 0, text_length));
        self.ime_selection.set(range(selection.start, selection.end));
        self.synchronization.borrow_mut().update(TextInputMethodState { text: state.text, selection });
        if notify_server {
            self.flush_notifications();
        }
    }

    /// Native failures leave the last acknowledged snapshot intact. The next
    /// text, selection, layout or focus notification retries from it. A
    /// reentrant call only updates pending state; it cannot recursively notify
    /// an inconsistent range or acknowledge a replacement client.
    fn flush_notifications(&self) {
        if self.notifying.get() || self.suppress_client_echo.get() {
            return;
        }
        let _guard = FlagGuard::set(&self.notifying);
        let Some(context) = self.edit_context.borrow().clone() else {
            return;
        };
        for _ in 0..8 {
            let pending = self.synchronization.borrow().pending();
            let Some(pending) = pending else {
                break;
            };
            let selection = range(pending.selection.start, pending.selection.end);
            let result = if pending.text_changed {
                context.notify_text_changed(range(0, pending.old_length), pending.new_length, selection)
            } else {
                context.notify_selection_changed(selection)
            };
            if result.is_err() {
                break;
            }
            self.synchronization.borrow_mut().acknowledge(&pending);
        }
        let _ = context.notify_layout_changed();
    }

    /// The text or the selection of a connection client changed.
    fn on_connection_client_changed(&self) {
        if self.suppress_client_echo.get() {
            return;
        }
        if self.is_composing.get() {
            let exact = self.exact_composition.borrow().clone();
            let Some(session) = exact else {
                return;
            };
            // This notification was not caused by our suppressed request. An
            // external edit makes the captured platform session stale.
            if !session.is_current() {
                self.reset_composition_state();
            }
        }
        self.sync_from_client(true);
    }

    fn on_exact_text_updating(&self, args: &ICoreTextTextUpdatingEventArgs) {
        let generation = self.client_generation.get();
        let composition_generation = self.composition_generation.get();
        let _ = args.set_result(CoreTextTextUpdatingResult::Failed);
        if generation != self.client_generation.get() {
            return;
        }
        let connection = self.connection.borrow().clone();
        let Some(connection) = connection.filter(|connection| connection.supports_exact_edits()) else {
            return;
        };
        let (Ok(native_range), Ok(selection)) = (args.range(), args.new_selection()) else {
            return;
        };
        // Read scalar properties first, then immediately take ownership of the
        // returned HSTRING. A later failing getter must not leak it.
        let Ok(text) = args.text().and_then(take_string_exact) else {
            return;
        };
        let edit = TextInputMethodEdit::new(
            TextSelection::new(native_range.start_caret_position, native_range.end_caret_position),
            text,
            TextSelection::new(selection.start_caret_position, selection.end_caret_position),
        );
        let before = self.synchronization.borrow().current().clone();
        let text_pending = self.synchronization.borrow().pending().is_some_and(|pending| pending.text_changed);
        if text_pending || !connection.edit_state().is_ok_and(|state| state == before) {
            self.sync_from_client(true);
            return;
        }
        let Ok(expected) = edit.apply_to(&before.text) else {
            return;
        };
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        let suppression = FlagGuard::set(&self.suppress_client_echo);
        let result = if self.is_composing.get() {
            let session = self.exact_composition.borrow().clone();
            session
                .ok_or(TextInputMethodError::StaleConnection)
                .and_then(|session| session.apply_edit(&edit))
        } else {
            // Committed input outside a composition: a pure insertion at the
            // caret is typing, so consecutive characters are one undo unit
            // (the editor's rule, #243 review M6).
            connection.apply_edit(&edit, crate::ferro_swap_chain_panel::tick_count())
        };
        drop(suppression);
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        self.sync_from_client(false);
        let accepted = result.is_ok()
            && connection
                .edit_state()
                .is_ok_and(|state| state.text == expected && state.selection == edit.selection);
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        if accepted {
            // Snapshot before the COM setter: it can synchronously reenter.
            let pending = self.synchronization.borrow().pending();
            if args.set_result(CoreTextTextUpdatingResult::Succeeded).is_ok() {
                if let Some(pending) = pending {
                    self.synchronization.borrow_mut().acknowledge(&pending);
                }
            }
        } else {
            self.flush_notifications();
        }
    }

    fn on_exact_selection_updating(&self, args: &ICoreTextSelectionUpdatingEventArgs) {
        let generation = self.client_generation.get();
        let composition_generation = self.composition_generation.get();
        let _ = args.set_result(CoreTextSelectionUpdatingResult::Failed);
        if generation != self.client_generation.get() {
            return;
        }
        let connection = self.connection.borrow().clone();
        let Some(connection) = connection.filter(|connection| connection.supports_exact_edits()) else {
            return;
        };
        let Ok(selection) = args.selection() else {
            return;
        };
        let selection = TextSelection::new(selection.start_caret_position, selection.end_caret_position);
        let before = self.synchronization.borrow().current().clone();
        let text_pending = self.synchronization.borrow().pending().is_some_and(|pending| pending.text_changed);
        if text_pending || !connection.edit_state().is_ok_and(|state| state == before) {
            self.sync_from_client(true);
            return;
        }
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        let suppression = FlagGuard::set(&self.suppress_client_echo);
        let result = if self.is_composing.get() {
            let session = self.exact_composition.borrow().clone();
            session
                .ok_or(TextInputMethodError::StaleConnection)
                .and_then(|session| session.select_exact(selection))
        } else {
            connection.select_exact(selection)
        };
        drop(suppression);
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        self.sync_from_client(false);
        let accepted = result.is_ok()
            && connection
                .edit_state()
                .is_ok_and(|state| state.text == before.text && state.selection == selection);
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        if accepted {
            let pending = self.synchronization.borrow().pending();
            if args.set_result(CoreTextSelectionUpdatingResult::Succeeded).is_ok() {
                if let Some(pending) = pending {
                    self.synchronization.borrow_mut().acknowledge(&pending);
                }
            }
        } else {
            self.flush_notifications();
        }
    }

    /// A composition of a connection client starts without deleting the
    /// selection (the reference sends Delete first); the selected text is
    /// replaced by the composition's first exact edit.
    fn on_exact_composition_started(&self) {
        self.invalidate_layout_request();
        let connection = self.connection.borrow().clone();
        let Some(connection) = connection.filter(|connection| connection.supports_exact_edits()) else {
            return;
        };
        let generation = self.client_generation.get();
        let composition_generation = self.composition_generation.get();
        let suppression = FlagGuard::set(&self.suppress_client_echo);
        let result = connection.begin_composition();
        drop(suppression);
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        if let Ok(session) = result {
            self.reset_composition_state();
            *self.exact_composition.borrow_mut() = Some(session);
            self.is_composing.set(true);
        }
        self.sync_from_client(true);
    }

    /// Commits exactly the accepted preview as one undo unit.
    fn on_exact_composition_completed(&self) {
        self.invalidate_layout_request();
        let Some(session) = self.exact_composition.borrow().clone() else {
            return;
        };
        let generation = self.client_generation.get();
        let composition_generation = self.composition_generation.get();
        let suppression = FlagGuard::set(&self.suppress_client_echo);
        let result = session.complete(crate::ferro_swap_chain_panel::tick_count());
        drop(suppression);
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        if result.is_ok() || !session.is_current() {
            self.reset_composition_state();
        }
        self.sync_from_client(true);
    }

    fn reset_connection_client(&self) {
        let generation = self.client_generation.get();
        let composition_generation = self.composition_generation.get();
        if self.is_composing.get() {
            let client = self.client.borrow().clone();
            let suppression = FlagGuard::set(&self.suppress_client_echo);
            if let Some(editing) = client.as_ref().and_then(|client| client.input_method_editing()) {
                let _ = editing.cancel_composition();
            }
            drop(suppression);
        }
        if generation != self.client_generation.get() || composition_generation != self.composition_generation.get() {
            return;
        }
        self.reset_composition_state();
        self.sync_from_client(true);
    }
}
