//! KeyboardEngine.Input.cs functions/navigation, clipboard tasks, swipes and precise selection.
use super::*;
use crate::{
    editor::EditorResult,
    settings::{OneHandedMode, SwipeAction},
    text::TextUtils,
};
use std::task::{Context, Poll, Waker};
impl KeyboardEngine {
    pub fn hide_requested(&self) -> &Event<Self> {
        &self.hide_requested
    }
    pub fn subtype_picker_requested(&self) -> &Event<Self> {
        &self.subtype_picker_requested
    }
    pub fn settings_requested(&self) -> &Event<Self> {
        &self.settings_requested
    }
    pub fn resize_mode_requested(&self) -> &Event<Self> {
        &self.resize_mode_requested
    }
    pub fn voice_input_requested(&self) -> &Event<Self> {
        &self.voice_input_requested
    }
    pub(super) fn handle_function(&self, data: &dyn ITextKeyData) -> EngineResult<()> {
        let editor = self.editor();
        match data.data().code() {
            KeyCode::ViewCharacters => self.set_mode(KeyboardMode::Characters)?,
            KeyCode::ViewSymbols => self.set_mode(KeyboardMode::Symbols)?,
            KeyCode::ViewSymbols2 => self.set_mode(KeyboardMode::Symbols2)?,
            KeyCode::ViewNumeric => self.set_mode(KeyboardMode::Numeric)?,
            KeyCode::ViewNumericAdvanced => self.set_mode(KeyboardMode::NumericAdvanced)?,
            KeyCode::ViewPhone => self.set_mode(KeyboardMode::Phone)?,
            KeyCode::ViewPhone2 => self.set_mode(KeyboardMode::Phone2)?,
            KeyCode::ImeUiModeText => self.set_ui_mode(KeyboardUiMode::Text)?,
            KeyCode::ImeUiModeMedia => self.set_ui_mode(KeyboardUiMode::Media)?,
            KeyCode::ImeUiModeClipboard => self.set_ui_mode(KeyboardUiMode::Clipboard)?,
            KeyCode::ImeUiModeEditing => self.set_ui_mode(KeyboardUiMode::Editing)?,
            KeyCode::LanguageSwitch | KeyCode::ImeNextSubtype | KeyCode::SystemNextInputMethod => {
                self.next_subtype()?
            }
            KeyCode::ImePrevSubtype | KeyCode::SystemPrevInputMethod => self.previous_subtype()?,
            KeyCode::ShowSubtypePicker
            | KeyCode::ImeSubtypePicker
            | KeyCode::SystemInputMethodPicker => self.subtype_picker_requested.raise(self),
            KeyCode::Settings => self.settings_requested.raise(self),
            KeyCode::ImeHideUi => self.hide_requested.raise(self),
            KeyCode::VoiceInput => self.voice_input_requested.raise(self),
            KeyCode::ArrowLeft => self.navigate(|e| e.move_horizontal(-1))?,
            KeyCode::ArrowRight => self.navigate(|e| e.move_horizontal(1))?,
            KeyCode::ArrowUp => self.navigate(|e| e.move_vertical(-1))?,
            KeyCode::ArrowDown => self.navigate(|e| e.move_vertical(1))?,
            KeyCode::MoveStartOfLine => self.navigate(|e| e.move_line_start())?,
            KeyCode::MoveEndOfLine => self.navigate(|e| e.move_line_end())?,
            KeyCode::MoveStartOfPage => self.navigate(|e| {
                e.move_document_start();
                Ok(())
            })?,
            KeyCode::MoveEndOfPage => self.navigate(|e| {
                e.move_document_end();
                Ok(())
            })?,
            KeyCode::ClipboardCopy => {
                self.start_clipboard_operation(self.copy_async())?;
            }
            KeyCode::ClipboardCut => {
                self.start_clipboard_operation(self.cut_async())?;
            }
            KeyCode::ClipboardPaste => {
                self.start_clipboard_operation(self.paste_async())?;
            }
            KeyCode::ClipboardSelect => {
                if let Some(editor) = editor {
                    editor.toggle_selection_mode()?;
                }
                self.state_changed.raise(self);
            }
            KeyCode::ClipboardSelectAll => self.navigate(|e| {
                e.select_all();
                Ok(())
            })?,
            KeyCode::ClipboardClearHistory => self.clipboard_history.clear(false),
            KeyCode::ClipboardClearFullHistory => self.clipboard_history.clear(true),
            KeyCode::ClipboardClearPrimaryClip => {
                if let Some(primary) = self.clipboard_history.primary() {
                    self.clipboard_history.remove(&primary);
                }
            }
            KeyCode::Undo => self.edit(|e| e.undo())?,
            KeyCode::Redo => self.edit(|e| e.redo())?,
            KeyCode::CompactLayoutToLeft => self.settings.set_one_handed_mode(OneHandedMode::Left),
            KeyCode::CompactLayoutToRight => {
                self.settings.set_one_handed_mode(OneHandedMode::Right)
            }
            KeyCode::ToggleCompactLayout => self.toggle_one_handed(),
            KeyCode::ToggleFloatingWindow => self.settings.set_floating(!self.settings.floating()),
            KeyCode::ToggleResizeMode => self.resize_mode_requested.raise(self),
            KeyCode::SplitLayout => self.settings.set_split_keyboard(true),
            KeyCode::MergeLayout => self.settings.set_split_keyboard(false),
            KeyCode::ToggleIncognitoMode => self
                .settings
                .set_incognito_mode(!self.settings.incognito_mode()),
            KeyCode::ToggleAutocorrect => self
                .settings
                .set_auto_correct(!self.settings.auto_correct()),
            KeyCode::ToggleSmartbarVisibility => self
                .settings
                .set_smartbar_enabled(!self.settings.smartbar_enabled()),
            KeyCode::CharWidthSwitcher => {
                self.half_width.set(!self.half_width.get());
                self.recompute_keyboard()?;
                self.state_changed.raise(self);
            }
            KeyCode::CharWidthFull => {
                self.half_width.set(false);
                self.recompute_keyboard()?;
                self.state_changed.raise(self);
            }
            KeyCode::CharWidthHalf => {
                self.half_width.set(true);
                self.recompute_keyboard()?;
                self.state_changed.raise(self);
            }
            KeyCode::KanaSwitcher => {
                self.kana_kata.set(!self.kana_kata.get());
                self.recompute_keyboard()?;
                self.state_changed.raise(self);
            }
            KeyCode::KanaHira => {
                self.kana_kata.set(false);
                self.recompute_keyboard()?;
                self.state_changed.raise(self);
            }
            KeyCode::KanaKata | KeyCode::KanaHalfKata => {
                self.kana_kata.set(true);
                self.recompute_keyboard()?;
                self.state_changed.raise(self);
            }
            _ => {} // Source switch intentionally ignores other function codes.
        }
        Ok(())
    }
    fn toggle_one_handed(&self) {
        self.settings.set_one_handed_mode(
            if self.settings.one_handed_mode() == OneHandedMode::Off {
                OneHandedMode::Right
            } else {
                OneHandedMode::Off
            },
        );
    }
    fn navigate(
        &self,
        action: impl FnOnce(&EditorController) -> EditorResult<()>,
    ) -> EngineResult<()> {
        if self.editor().is_none() {
            return Ok(());
        }
        self.finish_glide(false)?;
        *self.last_auto_correction.borrow_mut() = None;
        self.last_insert_was_auto_space.set(false);
        self.without_target_events(|| Ok(action(self.editor_required()?.as_ref())?))?;
        self.editor_required()?.break_undo_group();
        self.update_auto_capitalization()?;
        self.update_suggestions()
    }
    pub fn move_cursor(&self, steps: i32) -> EngineResult<()> {
        self.navigate(|e| e.move_horizontal(steps))
    }
    pub fn clipboard_service(&self) -> Rc<dyn IClipboardService> {
        self.clipboard_service.borrow().clone()
    }
    pub fn set_clipboard_service(&self, service: Rc<dyn IClipboardService>) {
        *self.clipboard_service.borrow_mut() = service;
    }
    /// Selects a native executor for pending clipboard key commands. The initial poll is
    /// performed synchronously by Core; the hook only resumes pending work on the owner thread.
    pub fn set_clipboard_spawner(&self, spawner: Option<ClipboardSpawner>) {
        *self.clipboard_spawner.borrow_mut() = spawner;
    }
    pub fn clipboard_spawner(&self) -> Option<ClipboardSpawner> {
        self.clipboard_spawner.borrow().clone()
    }
    /// Fire-and-forget key-command failures are observable without emulating Task status.
    /// Callers awaiting copy_async/cut_async/paste_async receive their Result directly.
    pub fn clipboard_failed(&self) -> &Event<EngineError> {
        &self.clipboard_failed
    }
    fn start_clipboard_operation(&self, operation: ClipboardOperation) -> EngineResult<()> {
        let engine = self.self_ref.upgrade().expect("live engine receiver");
        let mut job: crate::clipboard::ClipboardJob = Box::pin(async move {
            if let Err(error) = operation.await {
                engine.clipboard_failed.raise(&error);
            }
        });
        if let Poll::Ready(()) = job.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
            return Ok(());
        }
        let Some(spawner) = self.clipboard_spawner() else {
            // A pending operation cannot silently disappear or block the input thread.
            return Err(EngineError::ClipboardRuntimeRequired);
        };
        spawner(job);
        Ok(())
    }
    /// Native Rust future: execution starts when polled; dropping it cancels the waiter.
    pub fn copy_async(&self) -> ClipboardOperation {
        let engine = self.self_ref.upgrade().expect("live engine receiver");
        Box::pin(async move {
            let text = engine.editor().map(|e| e.selected_text()).transpose()?;
            let Some(text) = text.filter(|t| !t.is_empty()) else {
                return Ok(());
            };
            engine
                .clipboard_service()
                .set_text_async(Some(text.clone()))
                .await
                .map_err(EngineError::ClipboardService)?;
            engine.record_clipboard(&text)?;
            Ok(())
        })
    }
    pub fn cut_async(&self) -> ClipboardOperation {
        let engine = self.self_ref.upgrade().expect("live engine receiver");
        Box::pin(async move {
            let text = engine.editor().map(|e| e.selected_text()).transpose()?;
            let Some(text) = text.filter(|t| !t.is_empty()) else {
                return Ok(());
            };
            engine
                .clipboard_service()
                .set_text_async(Some(text.clone()))
                .await
                .map_err(EngineError::ClipboardService)?;
            engine.record_clipboard(&text)?;
            engine.edit(|e| e.delete_backward())?;
            Ok(())
        })
    }
    pub fn paste_async(&self) -> ClipboardOperation {
        let engine = self.self_ref.upgrade().expect("live engine receiver");
        Box::pin(async move {
            let mut text = engine
                .clipboard_service()
                .get_text_async()
                .await
                .map_err(EngineError::ClipboardService)?;
            if text.as_ref().is_none_or(Utf16Text::is_empty) {
                text = engine.clipboard_history.primary().map(|c| c.text.clone());
            }
            if let Some(text) = text.filter(|t| !t.is_empty()) {
                engine.input_text(&text)?;
            }
            Ok(())
        })
    }
    pub fn record_clipboard(&self, text: &Utf16Text) -> EngineResult<()> {
        if self.settings.clipboard_history_enabled() && !self.is_incognito() {
            self.clipboard_history.add(Some(text))?;
        }
        Ok(())
    }
    pub fn execute_swipe_action(&self, action: SwipeAction, count: i32) -> EngineResult<()> {
        match action {
            SwipeAction::NoAction => return Ok(()),
            SwipeAction::CycleToPreviousKeyboardMode => self.set_mode(match self.mode() {
                KeyboardMode::Characters => KeyboardMode::Numeric,
                KeyboardMode::Symbols => KeyboardMode::Characters,
                KeyboardMode::Symbols2 => KeyboardMode::Symbols,
                _ => KeyboardMode::Characters,
            })?,
            SwipeAction::CycleToNextKeyboardMode => self.set_mode(match self.mode() {
                KeyboardMode::Characters => KeyboardMode::Symbols,
                KeyboardMode::Symbols => KeyboardMode::Symbols2,
                KeyboardMode::Symbols2 => KeyboardMode::Characters,
                _ => KeyboardMode::Characters,
            })?,
            SwipeAction::DeleteCharacter => {
                for _ in 0..count {
                    self.edit(|e| e.delete_backward())?;
                }
            }
            SwipeAction::DeleteWord => {
                for _ in 0..count {
                    self.edit(|e| e.delete_word_backward())?;
                }
            }
            SwipeAction::DeleteCharactersPrecisely
            | SwipeAction::DeleteWordsPrecisely
            | SwipeAction::SelectCharactersPrecisely
            | SwipeAction::SelectWordsPrecisely => {}
            SwipeAction::HideKeyboard => self.hide_requested.raise(self),
            SwipeAction::InsertSpace => self.handle_space()?,
            SwipeAction::MoveCursorUp => {
                for _ in 0..count {
                    self.navigate(|e| e.move_vertical(-1))?;
                }
            }
            SwipeAction::MoveCursorDown => {
                for _ in 0..count {
                    self.navigate(|e| e.move_vertical(1))?;
                }
            }
            SwipeAction::MoveCursorLeft => self.move_cursor(count.wrapping_neg())?,
            SwipeAction::MoveCursorRight => self.move_cursor(count)?,
            SwipeAction::MoveCursorStartOfLine => self.navigate(|e| e.move_line_start())?,
            SwipeAction::MoveCursorEndOfLine => self.navigate(|e| e.move_line_end())?,
            SwipeAction::MoveCursorStartOfPage => self.navigate(|e| {
                e.move_document_start();
                Ok(())
            })?,
            SwipeAction::MoveCursorEndOfPage => self.navigate(|e| {
                e.move_document_end();
                Ok(())
            })?,
            SwipeAction::Redo => self.edit(|e| e.redo())?,
            SwipeAction::Undo => self.edit(|e| e.undo())?,
            SwipeAction::Shift => {
                if self.mode() == KeyboardMode::Characters {
                    self.set_shift_state(
                        if matches!(
                            self.shift_state(),
                            ShiftState::Unshifted | ShiftState::ShiftedAutomatic
                        ) {
                            ShiftState::ShiftedManual
                        } else {
                            ShiftState::Unshifted
                        },
                    )?;
                }
            }
            SwipeAction::ShowSubtypePicker => self.subtype_picker_requested.raise(self),
            SwipeAction::SwitchToPrevSubtype => self.previous_subtype()?,
            SwipeAction::SwitchToNextSubtype => self.next_subtype()?,
            SwipeAction::SwitchToClipboardContext => self.set_ui_mode(KeyboardUiMode::Clipboard)?,
            SwipeAction::SwitchToMediaContext => self.set_ui_mode(KeyboardUiMode::Media)?,
            SwipeAction::SwitchToEditingContext => self.set_ui_mode(KeyboardUiMode::Editing)?,
            SwipeAction::ToggleOneHandedMode => self.toggle_one_handed(),
            SwipeAction::ToggleSmartbarVisibility => self
                .settings
                .set_smartbar_enabled(!self.settings.smartbar_enabled()),
            _ => {} // Source unknown numeric action reaches trailing feedback without another effect.
        }
        self.feedback(FeedbackKind::GestureStep);
        Ok(())
    }
    pub fn begin_precise_selection(&self) -> EngineResult<()> {
        if let Some(editor) = self.editor() {
            self.precise_origin.set(Some(editor.selection_end()?));
        }
        Ok(())
    }
    pub fn update_precise_selection(&self, steps: i32, by_words: bool) -> EngineResult<()> {
        let Some(origin) = self
            .precise_origin
            .get()
            .filter(|_| self.editor().is_some())
        else {
            return Ok(());
        };
        let text = self.editor_required()?.text();
        let origin = origin.min(text.len() as i32);
        let mut start = origin;
        for _ in 0..steps.max(0) {
            if start == 0 {
                break;
            }
            start = if by_words {
                TextUtils::find_previous_word_boundary(&text, start)
            } else {
                let from = start.wrapping_sub(16).max(0);
                let length = start.min(16);
                let end = from
                    .checked_add(length)
                    .filter(|e| *e >= 0 && *e as usize <= text.len())
                    .ok_or(EditorError::SubstringOutOfRange)?;
                let part = text
                    .units()
                    .get(from as usize..end as usize)
                    .ok_or(EditorError::SubstringOutOfRange)?;
                start.wrapping_sub(TextUtils::last_grapheme_length(&Utf16Text::from_units(
                    part.to_vec(),
                )))
            };
        }
        self.without_target_events(|| {
            self.editor_required()?
                .target()
                .select(start, origin.wrapping_sub(start));
            Ok(())
        })?;
        self.feedback(FeedbackKind::GestureStep);
        Ok(())
    }
    pub fn end_precise_selection(&self, delete: bool) -> EngineResult<()> {
        if self.editor().is_none() || self.precise_origin.get().is_none() {
            return Ok(());
        }
        self.precise_origin.set(None);
        if delete && self.editor_required()?.has_selection()? {
            self.edit(|e| e.delete_backward())
        } else {
            self.update_suggestions()
        }
    }
    pub fn cancel_precise_selection(&self) -> EngineResult<()> {
        let Some(origin) = self
            .precise_origin
            .get()
            .filter(|_| self.editor().is_some())
        else {
            return Ok(());
        };
        self.precise_origin.set(None);
        self.without_target_events(|| {
            self.editor_required()?.set_caret(origin);
            Ok(())
        })
    }
}
