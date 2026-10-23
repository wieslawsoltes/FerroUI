//! Bridges the framework's `ITextInputMethodImpl` contract to the
//! `CoreTextEditContext` of the Windows Runtime.
//!
//! The text the input method sees is kept as UTF-16, as the reference keeps
//! a string: the ranges of the edit context and the selection of the
//! framework count UTF-16 units.

/// `Math.Clamp`.
fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

/// The units of `text` from `start`, `length` of them (`Substring`), as a
/// string.
fn substring(text: &[u16], start: i32, length: i32) -> String {
    let start = start.max(0) as usize;
    let end = (start + length.max(0) as usize).min(text.len());
    String::from_utf16_lossy(&text[start.min(end)..end])
}

/// `text` with the units from `start` to `end` replaced by `new_text`.
fn splice(text: &[u16], start: i32, end: i32, new_text: &[u16]) -> Vec<u16> {
    let start = start.max(0) as usize;
    let end = end.max(0) as usize;
    let mut result = Vec::with_capacity(text.len() - (end - start) + new_text.len());
    result.extend_from_slice(&text[..start]);
    result.extend_from_slice(new_text);
    result.extend_from_slice(&text[end..]);
    result
}

/// The length of a text in UTF-16 units.
fn length(text: &[u16]) -> i32 {
    text.len() as i32
}

/// A native query counts code units, so its requested slice can contain only
/// one half of a surrogate pair. Keep those units intact for the HSTRING reply.
/// Owning the slice also releases client-state borrows before the COM callback.
fn requested_units(text: &[u16], start: i32, end: i32) -> Vec<u16> {
    let start = clamp(start, 0, length(text)) as usize;
    let end = clamp(end, start as i32, length(text)) as usize;
    text[start..end].to_vec()
}

#[cfg(windows)]
pub(crate) use imp::WinUITextInputMethod;

#[cfg(windows)]
mod imp {
    use super::{clamp, length, requested_units, splice, substring};
    use crate::core_text_input_options::CoreTextInputOptions;
    use crate::core_text_layout::{CoreTextLayoutBounds, CoreTextTextGeometry};
    use crate::core_text_synchronization::CoreTextSynchronization;
    use crate::ferro_swap_chain_panel::FerroSwapChainPanel;
    use crate::interop::event_handlers::*;
    use crate::interop::win_rt::{
        client_origin_on_screen, factory, get_window_from_window_id, take_string, take_string_exact,
    };
    use crate::interop::*;
    use ferroui_base::input::raw::{
        IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawTextInputEventArgs,
    };
    use ferroui_base::input::text_input::{
        ITextInputMethodImpl, TextInputMethodClient, TextInputMethodComposition,
        TextInputMethodConnection, TextInputMethodEdit, TextInputMethodState, TextInputOptions,
        TextSelection,
    };
    use ferroui_base::input::{
        IInputDevice, IInputRoot, IKeyboardDevice, Key, KeyDeviceType, PhysicalKey,
        RawInputModifiers,
    };
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::{Matrix, Rect as FerroRect, Ref, Visual};
    use ferroui_microcom::{ComPtr, HResult};
    use ferroui_win32::win_rt::HStringInterop;
    use std::cell::{Cell, RefCell};
    use std::rc::{Rc, Weak};

    type InputGetter = Box<dyn Fn() -> Option<Rc<dyn Fn(Rc<dyn IRawInputEventArgs>)>>>;
    type InputRootGetter = Box<dyn Fn() -> Option<Rc<dyn IInputRoot>>>;
    type KeyboardDeviceGetter = Box<dyn Fn() -> Option<Rc<dyn IKeyboardDevice>>>;

    pub(crate) struct WinUITextInputMethod {
        this: Weak<WinUITextInputMethod>,
        panel: Weak<FerroSwapChainPanel>,
        get_input: InputGetter,
        get_input_root: InputRootGetter,
        get_keyboard_device: KeyboardDeviceGetter,

        edit_context: RefCell<Option<ComPtr<ICoreTextEditContext>>>,
        input_options: CoreTextInputOptions,
        client: RefCell<Option<Rc<dyn TextInputMethodClient>>>,
        /// The subscriptions to the events of the client.
        client_events: RefCell<Vec<Rc<dyn IDisposable>>>,
        has_focus: Cell<bool>,
        is_composing: Cell<bool>,

        ime_text: RefCell<Vec<u16>>,
        ime_selection: Cell<CoreTextRange>,
        composition_start: Cell<i32>,
        composition_length: Cell<i32>,
        cursor_rect: Cell<FerroRect>,
        suppress_client_echo: Cell<bool>,
        connection: RefCell<Option<Rc<TextInputMethodConnection>>>,
        exact_composition: RefCell<Option<TextInputMethodComposition>>,
        exact_client: Cell<bool>,
        client_generation: Cell<u64>,
        composition_generation: Cell<u64>,
        layout_generation: Cell<u64>,
        synchronization: RefCell<CoreTextSynchronization>,
        notifying: Cell<bool>,
    }

    struct FlagGuard<'a>(&'a Cell<bool>, bool);
    impl<'a> FlagGuard<'a> {
        fn set(flag: &'a Cell<bool>) -> Self {
            Self(flag, flag.replace(true))
        }
    }
    impl Drop for FlagGuard<'_> {
        fn drop(&mut self) {
            self.0.set(self.1);
        }
    }

    fn range(start: i32, end: i32) -> CoreTextRange {
        CoreTextRange {
            start_caret_position: start,
            end_caret_position: end,
        }
    }

    impl WinUITextInputMethod {
        pub(crate) fn new(
            panel: Weak<FerroSwapChainPanel>,
            get_input: InputGetter,
            get_input_root: InputRootGetter,
            get_keyboard_device: KeyboardDeviceGetter,
        ) -> Rc<WinUITextInputMethod> {
            Rc::new_cyclic(|this| WinUITextInputMethod {
                this: this.clone(),
                panel,
                get_input,
                get_input_root,
                get_keyboard_device,
                edit_context: RefCell::new(None),
                input_options: CoreTextInputOptions::default(),
                client: RefCell::new(None),
                client_events: RefCell::new(Vec::new()),
                has_focus: Cell::new(false),
                is_composing: Cell::new(false),
                ime_text: RefCell::new(Vec::new()),
                ime_selection: Cell::new(range(0, 0)),
                composition_start: Cell::new(0),
                composition_length: Cell::new(0),
                cursor_rect: Cell::new(FerroRect::default()),
                suppress_client_echo: Cell::new(false),
                connection: RefCell::new(None),
                exact_composition: RefCell::new(None),
                exact_client: Cell::new(false),
                client_generation: Cell::new(0),
                composition_generation: Cell::new(0),
                layout_generation: Cell::new(0),
                synchronization: RefCell::new(CoreTextSynchronization::default()),
                notifying: Cell::new(false),
            })
        }

        fn get_or_create_context(&self) -> Option<ComPtr<ICoreTextEditContext>> {
            let existing = self.edit_context.borrow().clone();
            if let Some(edit_context) = existing {
                self.apply_input_options(&edit_context);
                return Some(edit_context);
            }

            let generation = self.client_generation.get();
            let ctx = (|| -> Result<ComPtr<ICoreTextEditContext>, HResult> {
                let statics = factory::<ICoreTextServicesManagerStatics>(
                    "Windows.UI.Text.Core.CoreTextServicesManager",
                )?;
                let manager = statics.get_for_current_view()?.ok_or(HResult::POINTER)?;
                let ctx = manager.create_edit_context()?.ok_or(HResult::POINTER)?;
                ctx.set_input_pane_display_policy(CoreTextInputPaneDisplayPolicy::Automatic)?;

                macro_rules! subscribe {
                    ($add:ident, $handler:ident, $method:ident) => {
                        let this = self.this.clone();
                        ctx.$add(Some(&$handler(move |sender, args| {
                            if let (Some(this), Some(sender), Some(args)) =
                                (this.upgrade(), sender, args)
                            {
                                this.$method(sender, args);
                            }
                        })))?;
                    };
                }
                subscribe!(
                    add_text_requested,
                    text_requested_handler,
                    on_text_requested
                );
                subscribe!(
                    add_selection_requested,
                    selection_requested_handler,
                    on_selection_requested
                );
                subscribe!(
                    add_layout_requested,
                    layout_requested_handler,
                    on_layout_requested
                );
                subscribe!(add_text_updating, text_updating_handler, on_text_updating);
                subscribe!(
                    add_selection_updating,
                    selection_updating_handler,
                    on_selection_updating
                );
                subscribe!(
                    add_format_updating,
                    format_updating_handler,
                    on_format_updating
                );
                subscribe!(
                    add_composition_started,
                    composition_started_handler,
                    on_composition_started
                );
                subscribe!(
                    add_composition_completed,
                    composition_completed_handler,
                    on_composition_completed
                );
                let this = self.this.clone();
                ctx.add_focus_removed(Some(&focus_removed_handler(move |sender, args| {
                    if let (Some(this), Some(sender)) = (this.upgrade(), sender) {
                        this.on_focus_removed(sender, args);
                    }
                })))?;
                Ok(ctx)
            })();
            let ctx = match ctx {
                Ok(ctx) => ctx,
                Err(error) => {
                    if let Some(logger) =
                        Logger::try_get(LogEventLevel::Warning, LogArea::WINUI_PLATFORM)
                    {
                        logger.log(
                            None,
                            &format!("Creating the CoreTextEditContext failed: {error}"),
                        );
                    }
                    return None;
                }
            };

            // Context factories and event registration can invoke application
            // callbacks. A replacement client may already have created its own
            // context; an older initialization must not overwrite it.
            if generation != self.client_generation.get() {
                return self.edit_context.borrow().clone();
            }
            *self.edit_context.borrow_mut() = Some(ctx.clone());
            self.apply_input_options(&ctx);
            Some(ctx)
        }

        fn apply_input_options(&self, context: &ICoreTextEditContext) {
            if let Err(error) = self
                .input_options
                .apply(|scope| context.set_input_scope(scope))
            {
                if let Some(logger) =
                    Logger::try_get(LogEventLevel::Warning, LogArea::WINUI_PLATFORM)
                {
                    logger.log(
                        None,
                        &format!("Applying CoreText input options failed: {error}"),
                    );
                }
            }
        }

        /// Called by the panel when it gains/loses focus.
        pub(crate) fn on_panel_focus_changed(&self, has_focus: bool) {
            self.invalidate_layout_request();
            self.has_focus.set(has_focus);
            let Some(edit_context) = self.edit_context.borrow().clone() else {
                return;
            };
            if has_focus && self.client.borrow().is_some() {
                let _ = edit_context.notify_focus_enter();
                self.flush_notifications();
            } else {
                let _ = edit_context.notify_focus_leave();
            }
        }

        fn reset_composition_state(&self) {
            self.is_composing.set(false);
            self.composition_start.set(0);
            self.composition_length.set(0);
            self.exact_composition.borrow_mut().take();
            self.composition_generation
                .set(self.composition_generation.get().wrapping_add(1));
        }

        fn sync_from_client(&self, notify_server: bool) {
            let generation = self.client_generation.get();
            let composition_generation = self.composition_generation.get();
            let Some(client) = self.client.borrow().clone() else {
                self.ime_text.borrow_mut().clear();
                self.ime_selection.set(range(0, 0));
                return;
            };

            let state = if self.exact_client.get() {
                // Reading the installed client's current visible state remains
                // valid during a manager handoff; editing requires its captured
                // connection. Never downgrade a stale exact client to the legacy
                // committed-only surrounding-text projection.
                let Some(editing) = client.input_method_editing() else {
                    return;
                };
                let Ok(state) = editing.edit_state() else {
                    return;
                };
                state
            } else {
                TextInputMethodState {
                    text: client.surrounding_text(),
                    selection: client.selection(),
                }
            };
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            *self.ime_text.borrow_mut() = state.text.encode_utf16().collect();
            let text_length = length(&self.ime_text.borrow());
            let sel = state.selection;
            self.ime_selection.set(range(
                clamp(sel.start, 0, text_length),
                clamp(sel.end, 0, text_length),
            ));
            self.synchronization
                .borrow_mut()
                .update(TextInputMethodState {
                    text: state.text,
                    selection: TextSelection::new(
                        clamp(sel.start, 0, text_length),
                        clamp(sel.end, 0, text_length),
                    ),
                });
            if notify_server {
                self.flush_notifications();
            }
        }

        /// Native failures leave the last acknowledged snapshot intact. The
        /// next text, selection, layout or focus notification retries from it.
        /// A reentrant call only updates pending state; it cannot recursively
        /// notify an inconsistent range or acknowledge a replacement client.
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
                    context.notify_text_changed(
                        range(0, pending.old_length),
                        pending.new_length,
                        selection,
                    )
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

        fn on_client_surrounding_text_changed(&self) {
            if self.suppress_client_echo.get() {
                return;
            }
            if self.is_composing.get() {
                let exact = self.exact_composition.borrow().clone();
                if exact.is_none() {
                    return;
                }
                // This notification was not caused by our suppressed request.
                // An external edit makes the captured platform session stale.
                if exact.is_some_and(|session| !session.is_current()) {
                    self.reset_composition_state();
                }
            }
            self.sync_from_client(true);
        }

        fn on_client_selection_changed(&self) {
            self.on_client_surrounding_text_changed();
        }

        fn on_text_requested(
            &self,
            _sender: &ICoreTextEditContext,
            args: &ICoreTextTextRequestedEventArgs,
        ) {
            let Ok(Some(request)) = args.request() else {
                return;
            };
            let Ok(range) = request.range() else { return };
            let text = requested_units(
                &self.ime_text.borrow(),
                range.start_caret_position,
                range.end_caret_position,
            );
            if let Ok(value) = HStringInterop::new_utf16(&text) {
                let _ = request.set_text(value.handle());
            }
        }

        fn on_selection_requested(
            &self,
            _sender: &ICoreTextEditContext,
            args: &ICoreTextSelectionRequestedEventArgs,
        ) {
            if let Ok(Some(request)) = args.request() {
                let _ = request.set_selection(self.ime_selection.get());
            }
        }

        fn on_text_updating(
            &self,
            sender: &ICoreTextEditContext,
            args: &ICoreTextTextUpdatingEventArgs,
        ) {
            let generation = self.client_generation.get();
            let composition_generation = self.composition_generation.get();
            let _ = args.set_result(CoreTextTextUpdatingResult::Failed);
            if generation != self.client_generation.get() {
                return;
            }
            if !self.exact_client.get() {
                self.on_legacy_text_updating(sender, args);
                return;
            }
            let connection = self.connection.borrow().clone();
            let Some(connection) =
                connection.filter(|connection| connection.supports_exact_edits())
            else {
                return;
            };
            let (Ok(native_range), Ok(selection)) = (args.range(), args.new_selection()) else {
                return;
            };
            // Read scalar properties first, then immediately take ownership of
            // the returned HSTRING. A later failing getter must not leak it.
            let Ok(text) = args.text().and_then(take_string_exact) else {
                return;
            };
            let edit = TextInputMethodEdit::new(
                TextSelection::new(
                    native_range.start_caret_position,
                    native_range.end_caret_position,
                ),
                text,
                TextSelection::new(selection.start_caret_position, selection.end_caret_position),
            );
            let before = self.synchronization.borrow().current().clone();
            let text_pending = self
                .synchronization
                .borrow()
                .pending()
                .is_some_and(|pending| pending.text_changed);
            if text_pending || !connection.edit_state().is_ok_and(|state| state == before) {
                self.sync_from_client(true);
                return;
            }
            let Ok(expected) = edit.apply_to(&before.text) else {
                return;
            };
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            let suppression = FlagGuard::set(&self.suppress_client_echo);
            let result = if self.is_composing.get() {
                let session = self.exact_composition.borrow().clone();
                session
                    .ok_or(ferroui_base::input::text_input::TextInputMethodError::StaleConnection)
                    .and_then(|session| session.apply_edit(&edit))
            } else {
                connection.apply_edit(&edit, crate::ferro_swap_chain_panel::tick_count())
            };
            drop(suppression);
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            self.sync_from_client(false);
            let accepted = result.is_ok()
                && connection
                    .edit_state()
                    .is_ok_and(|state| state.text == expected && state.selection == edit.selection);
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            if accepted {
                // Snapshot before the COM setter: it can synchronously reenter.
                let pending = self.synchronization.borrow().pending();
                if args
                    .set_result(CoreTextTextUpdatingResult::Succeeded)
                    .is_ok()
                {
                    if let Some(pending) = pending {
                        self.synchronization.borrow_mut().acknowledge(&pending);
                    }
                }
            } else {
                self.flush_notifications();
            }
        }

        fn on_legacy_text_updating(
            &self,
            _sender: &ICoreTextEditContext,
            args: &ICoreTextTextUpdatingEventArgs,
        ) {
            let new_text: Vec<u16> = args
                .text()
                .map(take_string)
                .unwrap_or_default()
                .encode_utf16()
                .collect();
            let Ok(range) = args.range() else { return };
            let (start, end) = {
                let text = self.ime_text.borrow();
                let start = clamp(range.start_caret_position, 0, length(&text));
                let end = clamp(range.end_caret_position, start, length(&text));
                (start, end)
            };

            let updated = splice(&self.ime_text.borrow(), start, end, &new_text);
            *self.ime_text.borrow_mut() = updated;
            if let Ok(new_selection) = args.new_selection() {
                self.ime_selection.set(new_selection);
            }

            if self.is_composing.get() {
                if self.composition_length.get() == 0 {
                    self.composition_start.set(start);
                }
                let ime_selection = self.ime_selection.get();
                let text_length = length(&self.ime_text.borrow());
                let composition_length =
                    0.max(ime_selection.end_caret_position - self.composition_start.get());
                let composition_length =
                    composition_length.min(text_length - self.composition_start.get());
                self.composition_length.set(composition_length);

                let preedit = substring(
                    &self.ime_text.borrow(),
                    self.composition_start.get(),
                    composition_length,
                );
                let client = self.client.borrow().clone();
                if let Some(client) = client {
                    client.set_preedit_text_with_cursor(
                        Some(&preedit),
                        Some(ime_selection.end_caret_position - self.composition_start.get()),
                    );
                }
            } else if !new_text.is_empty() {
                self.dispatch_raw_text(&String::from_utf16_lossy(&new_text), true);
            }

            let _ = args.set_result(CoreTextTextUpdatingResult::Succeeded);
        }

        fn on_selection_updating(
            &self,
            sender: &ICoreTextEditContext,
            args: &ICoreTextSelectionUpdatingEventArgs,
        ) {
            let generation = self.client_generation.get();
            let composition_generation = self.composition_generation.get();
            let _ = args.set_result(CoreTextSelectionUpdatingResult::Failed);
            if generation != self.client_generation.get() {
                return;
            }
            if !self.exact_client.get() {
                self.on_legacy_selection_updating(sender, args);
                return;
            }
            let connection = self.connection.borrow().clone();
            let Some(connection) =
                connection.filter(|connection| connection.supports_exact_edits())
            else {
                return;
            };
            let Ok(selection) = args.selection() else {
                return;
            };
            let selection =
                TextSelection::new(selection.start_caret_position, selection.end_caret_position);
            let before = self.synchronization.borrow().current().clone();
            let text_pending = self
                .synchronization
                .borrow()
                .pending()
                .is_some_and(|pending| pending.text_changed);
            if text_pending || !connection.edit_state().is_ok_and(|state| state == before) {
                self.sync_from_client(true);
                return;
            }
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            let suppression = FlagGuard::set(&self.suppress_client_echo);
            let result = if self.is_composing.get() {
                let session = self.exact_composition.borrow().clone();
                session
                    .ok_or(ferroui_base::input::text_input::TextInputMethodError::StaleConnection)
                    .and_then(|session| session.select_exact(selection))
            } else {
                connection.select_exact(selection)
            };
            drop(suppression);
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            self.sync_from_client(false);
            let accepted = result.is_ok()
                && connection
                    .edit_state()
                    .is_ok_and(|state| state.text == before.text && state.selection == selection);
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            if accepted {
                let pending = self.synchronization.borrow().pending();
                if args
                    .set_result(CoreTextSelectionUpdatingResult::Succeeded)
                    .is_ok()
                {
                    if let Some(pending) = pending {
                        self.synchronization.borrow_mut().acknowledge(&pending);
                    }
                }
            } else {
                self.flush_notifications();
            }
        }

        fn on_legacy_selection_updating(
            &self,
            _sender: &ICoreTextEditContext,
            args: &ICoreTextSelectionUpdatingEventArgs,
        ) {
            if let Ok(selection) = args.selection() {
                self.ime_selection.set(selection);
            }
            let client = self.client.borrow().clone();
            if let (false, Some(client)) = (self.is_composing.get(), client) {
                self.suppress_client_echo.set(true);
                let ime_selection = self.ime_selection.get();
                client.set_selection(TextSelection::new(
                    ime_selection.start_caret_position,
                    ime_selection.end_caret_position,
                ));
                self.suppress_client_echo.set(false);
            }
            let _ = args.set_result(CoreTextSelectionUpdatingResult::Succeeded);
        }

        fn on_format_updating(
            &self,
            _sender: &ICoreTextEditContext,
            args: &ICoreTextFormatUpdatingEventArgs,
        ) {
            // Conversion-target clauses and the native underline palette are
            // not yet projected by this adapter. Do not acknowledge an ignored
            // formatting request as successfully applied.
            let _ = args.set_result(CoreTextFormatUpdatingResult::Failed);
        }

        fn on_composition_started(
            &self,
            sender: &ICoreTextEditContext,
            args: &ICoreTextCompositionStartedEventArgs,
        ) {
            self.invalidate_layout_request();
            if !self.exact_client.get() {
                self.on_legacy_composition_started(sender, args);
                return;
            }
            let connection = self.connection.borrow().clone();
            let Some(connection) =
                connection.filter(|connection| connection.supports_exact_edits())
            else {
                return;
            };
            let generation = self.client_generation.get();
            let composition_generation = self.composition_generation.get();
            let suppression = FlagGuard::set(&self.suppress_client_echo);
            let result = connection.begin_composition();
            drop(suppression);
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            if let Ok(session) = result {
                self.reset_composition_state();
                *self.exact_composition.borrow_mut() = Some(session);
                self.is_composing.set(true);
            }
            self.sync_from_client(true);
        }

        fn on_legacy_composition_started(
            &self,
            _sender: &ICoreTextEditContext,
            _args: &ICoreTextCompositionStartedEventArgs,
        ) {
            let ime_selection = self.ime_selection.get();
            if ime_selection.start_caret_position != ime_selection.end_caret_position {
                let input = (self.get_input)();
                let input_root = (self.get_input_root)();
                let keyboard = (self.get_keyboard_device)();
                if let (Some(input), Some(input_root), Some(keyboard)) =
                    (input, input_root, keyboard)
                {
                    self.suppress_client_echo.set(true);
                    let timestamp = crate::ferro_swap_chain_panel::tick_count();
                    let device: Rc<dyn IInputDevice> = keyboard;
                    for type_ in [RawKeyEventType::KeyDown, RawKeyEventType::KeyUp] {
                        input(Rc::new(RawKeyEventArgs::new(
                            device.clone(),
                            timestamp,
                            input_root.clone(),
                            type_,
                            Key::Delete,
                            RawInputModifiers::NONE,
                            PhysicalKey::Delete,
                            None,
                            KeyDeviceType::Keyboard,
                        )));
                    }
                    self.suppress_client_echo.set(false);
                }
            }

            self.is_composing.set(true);
            self.composition_start
                .set(self.ime_selection.get().start_caret_position);
            self.composition_length.set(0);
        }

        fn on_composition_completed(
            &self,
            sender: &ICoreTextEditContext,
            args: &ICoreTextCompositionCompletedEventArgs,
        ) {
            self.invalidate_layout_request();
            if !self.exact_client.get() {
                self.on_legacy_composition_completed(sender, args);
                return;
            }
            let Some(session) = self.exact_composition.borrow().clone() else {
                return;
            };
            let generation = self.client_generation.get();
            let composition_generation = self.composition_generation.get();
            let suppression = FlagGuard::set(&self.suppress_client_echo);
            let result = session.complete(crate::ferro_swap_chain_panel::tick_count());
            drop(suppression);
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            if result.is_ok() || !session.is_current() {
                self.reset_composition_state();
            }
            self.sync_from_client(true);
        }

        fn on_legacy_composition_completed(
            &self,
            _sender: &ICoreTextEditContext,
            _args: &ICoreTextCompositionCompletedEventArgs,
        ) {
            self.is_composing.set(false);
            let committed = {
                let text = self.ime_text.borrow();
                let len = 0.max(
                    self.composition_length
                        .get()
                        .min(length(&text) - self.composition_start.get()),
                );
                if len > 0 {
                    substring(&text, self.composition_start.get(), len)
                } else {
                    String::new()
                }
            };

            let client = self.client.borrow().clone();
            if let Some(client) = client {
                client.set_preedit_text(None);
            }
            self.composition_length.set(0);

            if !committed.is_empty() {
                self.dispatch_raw_text(&committed, false);
            }
        }

        fn on_focus_removed(&self, _sender: &ICoreTextEditContext, _args: Option<&IInspectable>) {
            self.has_focus.set(false);
            self.reset();
        }

        fn dispatch_raw_text(&self, text: &str, notify_server: bool) {
            let input = (self.get_input)();
            let input_root = (self.get_input_root)();
            let keyboard = (self.get_keyboard_device)();
            let (Some(input), Some(input_root), Some(keyboard)) = (input, input_root, keyboard)
            else {
                return;
            };

            self.suppress_client_echo.set(true);
            let device: Rc<dyn IInputDevice> = keyboard;
            input(Rc::new(RawTextInputEventArgs::new(
                device,
                crate::ferro_swap_chain_panel::tick_count(),
                input_root,
                text,
            )));
            self.suppress_client_echo.set(false);

            self.sync_from_client(notify_server);
        }
    }

    impl ITextInputMethodImpl for WinUITextInputMethod {
        fn set_client(&self, client: Option<Rc<dyn TextInputMethodClient>>) {
            let generation = self.client_generation.get().wrapping_add(1);
            self.client_generation.set(generation);
            let old = self.client.borrow().clone();
            if self.is_composing.get() {
                let suppression = FlagGuard::set(&self.suppress_client_echo);
                if let Some(old) = old.as_ref() {
                    if let Some(editing) = old.input_method_editing() {
                        let _ = editing.cancel_composition();
                    } else {
                        old.set_preedit_text(None);
                    }
                }
                drop(suppression);
                if generation != self.client_generation.get() {
                    return;
                }
            }
            let subscriptions = std::mem::take(&mut *self.client_events.borrow_mut());
            for subscription in subscriptions {
                subscription.dispose();
            }
            if generation != self.client_generation.get() {
                return;
            }

            // Client accessors can synchronously change focus and install a
            // replacement. Stage their results without borrowing our slots.
            let connection = client
                .as_ref()
                .and_then(|client| client.input_method_connection());
            if generation != self.client_generation.get() {
                return;
            }
            let exact_client = client.as_ref().is_some_and(|client| {
                client
                    .input_method_editing()
                    .is_some_and(|editing| editing.supports_exact_edits())
            });
            if generation != self.client_generation.get() {
                return;
            }
            self.connection.replace(connection);
            self.exact_client.set(exact_client);
            self.client.replace(client.clone());
            self.reset_composition_state();
            self.synchronization.borrow_mut().replace_client();

            let Some(client) = client else {
                self.ime_text.borrow_mut().clear();
                self.ime_selection.set(range(0, 0));
                self.synchronization
                    .borrow_mut()
                    .update(TextInputMethodState {
                        text: String::new(),
                        selection: TextSelection::default(),
                    });
                let context = self.edit_context.borrow().clone();
                if let (Some(edit_context), true) = (context, self.has_focus.get()) {
                    let _ = edit_context.notify_focus_leave();
                }
                return;
            };

            let handler = |call: fn(&WinUITextInputMethod)| -> Rc<dyn Fn()> {
                let this = self.this.clone();
                Rc::new(move || {
                    if let Some(this) = this
                        .upgrade()
                        .filter(|this| this.client_generation.get() == generation)
                    {
                        call(&this);
                    }
                })
            };
            let text_subscription =
                client.surrounding_text_changed(handler(Self::on_client_surrounding_text_changed));
            if generation != self.client_generation.get() {
                text_subscription.dispose();
                return;
            }
            let selection_subscription =
                client.selection_changed(handler(Self::on_client_selection_changed));
            if generation != self.client_generation.get() {
                text_subscription.dispose();
                selection_subscription.dispose();
                return;
            }
            self.client_events
                .replace(vec![text_subscription, selection_subscription]);

            let ctx = self.get_or_create_context();
            if generation != self.client_generation.get() {
                return;
            }
            self.sync_from_client(true);
            if generation != self.client_generation.get() {
                return;
            }
            if let (Some(ctx), true) = (ctx, self.has_focus.get()) {
                let _ = ctx.notify_focus_enter();
            }
        }

        fn set_cursor_rect(&self, rect: FerroRect) {
            self.invalidate_layout_request();
            self.cursor_rect.set(rect);
            self.flush_notifications();
        }

        fn set_options(&self, options: &TextInputOptions) {
            self.input_options.update(options);
            let Some(edit_context) = self.edit_context.borrow().clone() else {
                return;
            };
            self.apply_input_options(&edit_context);
        }

        fn reset(&self) {
            let generation = self.client_generation.get();
            let composition_generation = self.composition_generation.get();
            if self.is_composing.get() {
                let client = self.client.borrow().clone();
                let suppression = FlagGuard::set(&self.suppress_client_echo);
                if let Some(client) = client {
                    if let Some(editing) = client.input_method_editing() {
                        let _ = editing.cancel_composition();
                    } else {
                        client.set_preedit_text(None);
                    }
                }
                drop(suppression);
            }
            if generation != self.client_generation.get()
                || composition_generation != self.composition_generation.get()
            {
                return;
            }
            self.reset_composition_state();
            self.sync_from_client(true);
        }
    }

    include!("win_ui_text_input_method_layout.rs");

    #[cfg(test)]
    include!("win_ui_text_input_method_reentrancy_tests.rs");
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the integration.
    use super::*;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    #[test]
    fn clamp_is_the_one_of_math() {
        assert_eq!(0, clamp(-3, 0, 5));
        assert_eq!(5, clamp(9, 0, 5));
        assert_eq!(3, clamp(3, 0, 5));
        assert_eq!(5, clamp(i32::MAX, 0, 5));
    }

    #[test]
    fn substrings_count_utf16_units() {
        let text = units("a\u{1F600}b");
        assert_eq!(4, length(&text));
        assert_eq!("\u{1F600}", substring(&text, 1, 2));
        assert_eq!("b", substring(&text, 3, 1));
        assert_eq!("", substring(&text, 4, 0));
    }

    #[test]
    fn native_text_request_preserves_exact_code_units_and_snapshot_lifetime() {
        let mut text = units("a😀\0b");
        let request = requested_units(&text, 1, 2);
        assert_eq!(request, [0xd83d]);
        assert_eq!(requested_units(&text, 2, 4), [0xde00, 0]);
        // A reentrant client update cannot change a response captured before
        // invoking the native setter, and does not need a retained RefCell borrow.
        text.clear();
        assert_eq!(request, [0xd83d]);
        assert!(requested_units(&text, -1, i32::MAX).is_empty());
    }

    #[test]
    fn a_range_is_replaced() {
        assert_eq!(units("abXYe"), splice(&units("abcde"), 2, 4, &units("XY")));
        assert_eq!(
            units("hello world"),
            splice(&units("hello"), 5, 5, &units(" world"))
        );
        assert_eq!(units(""), splice(&units("abc"), 0, 3, &[]));
    }
}
