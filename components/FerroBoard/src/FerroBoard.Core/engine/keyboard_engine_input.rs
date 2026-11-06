//! KeyboardEngine.Input.cs key routing, shift wiring and text/editing paths. Derived Apache-2.0.
use super::*;
use crate::{
    editor::{EditorResult, EnterAction},
    text::{IComposer, TextUtils},
};

impl KeyboardEngine {
    pub fn input_key(&self, data: &dyn ITextKeyData) -> EngineResult<()> {
        self.on_key_down(data)?;
        self.on_key_up(data)
    }
    pub fn type_text(&self, text: &Utf16Text) -> EngineResult<()> {
        self.type_text_nullable(Some(text))
    }
    pub fn type_text_nullable(&self, text: Option<&Utf16Text>) -> EngineResult<()> {
        // GetCodePoints reads text.Length before any key or target processing in the source.
        let text = text.ok_or(EngineError::NullReference)?;
        for cp in TextUtils::get_code_points(text) {
            let key: TextKey = match cp {
                10 => TextKeyData::function(KeyCode::Enter, KeyType::EnterEditing, None),
                32 => TextKeyData::space_key(),
                _ => {
                    let scalar =
                        char::from_u32(cp as u32).ok_or(KeyDataError::InvalidCodePoint(cp))?;
                    let mut units = [0; 2];
                    let text = Utf16Text::from_units(scalar.encode_utf16(&mut units).to_vec());
                    AutoTextKeyData::create_from_text(
                        self.apply_shift(&text)?,
                        KeyType::Character,
                        0,
                        None,
                    )
                }
            };
            self.input_key(key.as_ref())?;
        }
        Ok(())
    }
    fn apply_shift(&self, text: &Utf16Text) -> EngineResult<Utf16Text> {
        let cp = TextUtils::get_code_points(text).first().copied();
        if self.shift_state().is_uppercase()
            && cp.is_some_and(|c| crate::text::is_lower_code_point(c as u32))
        {
            Ok(AutoTextKeyData::to_upper_with_culture(
                Some(text),
                Some(&self.culture()),
            )?)
        } else {
            Ok(text.clone())
        }
    }
    pub fn on_key_down(&self, data: &dyn ITextKeyData) -> EngineResult<()> {
        match data.data().code() {
            KeyCode::Shift => self.shift.down(
                || {
                    self.clock()
                        .ok_or(EngineError::NullReference)
                        .map(|clock| clock())
                },
                || self.settings.double_tap_shift_for_caps_lock(),
                || self.settings.double_tap_delay(),
                self,
            ),
            KeyCode::CapsLock => {
                self.set_shift_state(if self.shift_state() == ShiftState::CapsLock {
                    ShiftState::Unshifted
                } else {
                    ShiftState::CapsLock
                })
            }
            _ => Ok(()),
        }
    }
    pub fn on_key_up(&self, data: &dyn ITextKeyData) -> EngineResult<()> {
        let code = data.data().code();
        if !matches!(code, KeyCode::Delete | KeyCode::Shift | KeyCode::CapsLock) {
            *self.last_auto_correction.borrow_mut() = None;
        }
        if code != KeyCode::Space {
            self.last_space_time.set(None);
        }
        match code {
            KeyCode::Shift => {
                return self.shift.up(
                    || {
                        self.clock()
                            .ok_or(EngineError::NullReference)
                            .map(|clock| clock())
                    },
                    self,
                );
            }
            KeyCode::CapsLock => return Ok(()),
            KeyCode::Delete => {
                self.feedback(FeedbackKind::KeyPressDelete);
                self.handle_delete()?;
            }
            KeyCode::DeleteWord => {
                self.feedback(FeedbackKind::KeyPressDelete);
                self.edit(|e| e.delete_word_backward())?;
            }
            KeyCode::ForwardDelete => {
                self.feedback(FeedbackKind::KeyPressDelete);
                self.edit(|e| e.delete_forward())?;
            }
            KeyCode::ForwardDeleteWord => {
                self.feedback(FeedbackKind::KeyPressDelete);
                self.edit(|e| e.delete_word_forward())?;
            }
            KeyCode::Enter => {
                self.feedback(FeedbackKind::KeyPressEnter);
                self.handle_enter()?;
            }
            KeyCode::Space => {
                self.feedback(FeedbackKind::KeyPressSpace);
                self.handle_space()?;
            }
            KeyCode::CjkSpace => {
                self.feedback(FeedbackKind::KeyPressSpace);
                self.commit_raw(&"　".into())?;
            }
            KeyCode::Tab => {
                self.feedback(FeedbackKind::KeyPressFunction);
                self.commit_raw(&"\t".into())?;
            }
            _ => {
                if data.produces_text()? {
                    self.feedback(FeedbackKind::KeyPress);
                    self.handle_character(data.as_string(false)?.as_ref())?;
                } else {
                    self.feedback(FeedbackKind::KeyPressFunction);
                    self.handle_function(data)?;
                }
            }
        }
        if code != KeyCode::Shift {
            self.shift.mark_key_released();
        }
        Ok(())
    }
    pub fn on_key_repeat(&self, data: &dyn ITextKeyData) -> EngineResult<()> {
        self.feedback(FeedbackKind::KeyRepeat);
        match data.data().code() {
            KeyCode::Delete => {
                *self.last_auto_correction.borrow_mut() = None;
                self.edit(|e| e.delete_backward())?;
            }
            KeyCode::ForwardDelete => self.edit(|e| e.delete_forward())?,
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown => {
                self.handle_function(data)?
            }
            KeyCode::Space if !data.produces_text()? => {}
            _ => {
                if data.produces_text()? && data.data().code() != KeyCode::Space {
                    self.handle_character(data.as_string(false)?.as_ref())?;
                }
            }
        }
        Ok(())
    }
    pub fn on_key_cancel(&self, data: &dyn ITextKeyData) {
        if data.data().code() == KeyCode::Shift {
            self.shift.cancel();
        }
    }
    pub fn is_repeatable(data: &dyn ITextKeyData) -> bool {
        matches!(
            data.data().code(),
            KeyCode::Delete
                | KeyCode::ForwardDelete
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown
        )
    }
    pub fn composer(&self) -> Arc<dyn IComposer> {
        self.resources
            .get_composer(&self.active_subtype().composer())
    }
    pub fn double_space_window(&self) -> std::time::Duration {
        self.double_space_window.get()
    }
    pub fn set_double_space_window(&self, value: std::time::Duration) {
        self.double_space_window.set(value);
    }
    pub fn is_char_half_width(&self) -> bool {
        self.half_width.get()
    }
    pub fn is_kana_kata(&self) -> bool {
        self.kana_kata.get()
    }
    fn handle_character(&self, text: Option<&Utf16Text>) -> EngineResult<()> {
        if self.editor().is_none() {
            return Ok(());
        }
        let text = text.ok_or(crate::layouts::KeyDataError::NullReference)?;
        if text.is_empty() {
            return Ok(());
        }
        let rule = self.punctuation_rule();
        let is_word_char = TextUtils::is_word_core_char(text.units()[0]);
        if !is_word_char && !matches!(text.units(), [39] | [0x2019] | [45] | [95]) {
            self.apply_auto_correction(text)?;
            self.learn_current_word()?;
        }
        if self.last_insert_was_auto_space.get()
            && contains(
                &rule
                    .symbols_preceding_auto_space()
                    .ok_or(EngineError::NullReference)?,
                text,
            )
            && self.editor_required()?.get_text_before_cursor(1)? == Utf16Text::from(" ")
        {
            self.without_target_events(|| {
                self.editor_required()?.replace_before_cursor(1, text)?;
                if self.settings.auto_space_punctuation() && !self.is_followed_by_space()? {
                    self.editor_required()?.commit_text(&" ".into())?;
                    self.last_insert_was_auto_space.set(true);
                } else {
                    self.last_insert_was_auto_space.set(false);
                }
                Ok(())
            })?;
            self.shift.after_character();
            return self.after_edit(Some(text));
        }
        self.last_insert_was_auto_space.set(false);
        let composer = self.composer();
        self.without_target_events(|| {
            let preceding = if composer.to_read() > 0 {
                self.editor_required()?
                    .get_text_before_cursor(composer.to_read())?
            } else {
                Utf16Text::default()
            };
            let (delete, insert) = composer.get_actions(&preceding, text);
            if delete > 0 {
                self.editor_required()?
                    .replace_before_cursor(delete, &insert)?;
            } else if !insert.is_empty() {
                self.editor_required()?.commit_text(&insert)?;
            }
            Ok(())
        })?;
        self.shift.after_character();
        self.after_edit(Some(text))
    }
    pub(super) fn is_followed_by_space(&self) -> EngineResult<bool> {
        let after = self
            .editor()
            .map(|e| e.get_text_after_cursor(1))
            .transpose()?
            .unwrap_or_default();
        Ok(after
            .units()
            .first()
            .is_some_and(|c| TextUtils::is_white_space(*c)))
    }
    pub(super) fn handle_space(&self) -> EngineResult<()> {
        if self.editor().is_none() {
            return Ok(());
        }
        let now = self.clock().ok_or(EngineError::NullReference)?();
        // The source reloads _editor after Clock(), which may detach or replace it.
        let before = self
            .editor()
            .ok_or(EngineError::NullReference)?
            .get_text_before_cursor(3)?;
        if self.settings.double_space_period()
            && self.attributes().supports_nlp()
            && crate::editor::within_window(
                now,
                self.last_space_time.get(),
                self.double_space_window.get(),
            )
            && before.len() >= 2
            && before.units()[before.len() - 1] == 32
            && TextUtils::is_word_core_char(before.units()[before.len() - 2])
        {
            self.without_target_events(|| {
                Ok(self
                    .editor_required()?
                    .replace_before_cursor(1, &". ".into())?)
            })?;
            self.last_space_time.set(None);
            self.last_insert_was_auto_space.set(true);
            self.after_edit(Some(&". ".into()))?;
            return self.update_auto_capitalization();
        }
        self.apply_auto_correction(&" ".into())?;
        self.learn_current_word()?;
        self.last_insert_was_auto_space.set(false);
        self.without_target_events(|| Ok(self.editor_required()?.commit_text(&" ".into())?))?;
        self.last_space_time.set(Some(now));
        if matches!(self.mode(), KeyboardMode::Symbols | KeyboardMode::Symbols2) {
            self.set_mode(KeyboardMode::Characters)?;
        }
        self.shift.after_character();
        self.after_edit(Some(&" ".into()))
    }
    fn handle_enter(&self) -> EngineResult<()> {
        if self.editor().is_none() || self.target().is_none() {
            return Ok(());
        }
        self.finish_glide(false)?;
        self.apply_auto_correction(&"\n".into())?;
        self.learn_current_word()?;
        self.last_insert_was_auto_space.set(false);
        let action = self.attributes().effective_enter_action();
        if action == EnterAction::NewLine && self.attributes().is_multiline() {
            self.without_target_events(|| {
                Ok(self.editor_required()?.commit_text(&"\n".into())?)
            })?;
            return self.after_edit(Some(&"\n".into()));
        }
        let handled = self
            .target()
            .ok_or(EngineError::NoEditor)?
            .perform_enter_action(action);
        if !handled && self.attributes().is_multiline() {
            self.without_target_events(|| {
                Ok(self.editor_required()?.commit_text(&"\n".into())?)
            })?;
            self.after_edit(Some(&"\n".into()))
        } else {
            self.update_suggestions()
        }
    }
    fn handle_delete(&self) -> EngineResult<()> {
        if self.editor().is_none() {
            return Ok(());
        }
        self.finish_glide(false)?;
        let correction = self.last_auto_correction.borrow().clone();
        if let Some(ac) = correction
            && self.settings.undo_auto_correct_on_backspace()
            && !self.editor_required()?.has_selection()?
        {
            let expected = concat_nullable(ac.corrected.as_ref(), Some(&ac.separator));
            let before = self
                .editor_required()?
                .get_text_before_cursor(expected.len() as i32)?;
            if before == expected && self.editor_required()?.selection_start() == ac.end {
                self.without_target_events(|| {
                    Ok(self
                        .editor_required()?
                        .replace_before_cursor(expected.len() as i32, &ac.original)?)
                })?;
                self.rejected_corrections
                    .borrow_mut()
                    .insert(OrdinalKey(ac.original));
                *self.last_auto_correction.borrow_mut() = None;
                return self.after_edit(None);
            }
        }
        *self.last_auto_correction.borrow_mut() = None;
        if self.last_insert_was_auto_space.get() {
            self.last_insert_was_auto_space.set(false);
        }
        self.edit(|e| e.delete_backward())
    }
    fn commit_raw(&self, text: &Utf16Text) -> EngineResult<()> {
        if self.editor().is_none() {
            return Ok(());
        }
        self.without_target_events(|| Ok(self.editor_required()?.commit_text(text)?))?;
        self.after_edit(Some(text))
    }
    pub fn input_emoji(&self, emoji: &Utf16Text) -> EngineResult<()> {
        self.input_emoji_nullable(Some(emoji))
    }
    pub fn input_emoji_nullable(&self, emoji: Option<&Utf16Text>) -> EngineResult<()> {
        self.feedback(FeedbackKind::KeyPress);
        self.input_text_nullable(emoji)?;
        if !self.is_incognito() && self.settings.emoji_history_enabled() {
            self.emoji_history.add_nullable(emoji)?;
        }
        Ok(())
    }
    pub(super) fn edit(
        &self,
        action: impl FnOnce(&EditorController) -> EditorResult<bool>,
    ) -> EngineResult<()> {
        if self.editor().is_none() {
            return Ok(());
        }
        let changed =
            self.without_target_events(|| Ok(action(self.editor_required()?.as_ref())?))?;
        if changed {
            self.after_edit(None)?;
        }
        Ok(())
    }
    pub(super) fn editor_required(&self) -> EngineResult<Rc<EditorController>> {
        self.editor().ok_or(EngineError::NoEditor)
    }
    pub fn feedback_requested(&self) -> &Event<FeedbackKind> {
        &self.feedback_requested
    }
    pub(super) fn feedback(&self, kind: FeedbackKind) {
        self.feedback_requested.raise(&kind);
    }
    pub fn feedback_long_press(&self) {
        self.feedback(FeedbackKind::LongPress);
    }
    pub fn feedback_gesture_step(&self) {
        self.feedback(FeedbackKind::GestureStep);
    }
}
pub(super) fn contains(haystack: &Utf16Text, needle: &Utf16Text) -> bool {
    needle.is_empty()
        || haystack
            .units()
            .windows(needle.len())
            .any(|w| w == needle.units())
}
pub(super) fn concat(left: &Utf16Text, right: &Utf16Text) -> Utf16Text {
    let mut text = left.units().to_vec();
    text.extend_from_slice(right.units());
    Utf16Text::from_units(text)
}

/// Source string concatenation treats a null operand as empty; this is not replacement policy.
pub(super) fn concat_nullable(left: Option<&Utf16Text>, right: Option<&Utf16Text>) -> Utf16Text {
    let mut text = Vec::new();
    if let Some(v) = left {
        text.extend_from_slice(v.units())
    }
    if let Some(v) = right {
        text.extend_from_slice(v.units())
    }
    Utf16Text::from_units(text)
}
