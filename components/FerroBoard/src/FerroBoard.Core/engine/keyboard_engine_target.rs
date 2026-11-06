//! Attach/Detach, external target notifications, and arbitrary InputText from the unchanged engine.
use super::*;
use crate::layouts::PunctuationRule;
use std::sync::Arc;

impl From<EditorError> for EngineError {
    fn from(e: EditorError) -> Self {
        Self::Editor(e)
    }
}
impl KeyboardEngine {
    pub fn target(&self) -> Option<Rc<dyn ITextInputTarget>> {
        self.target.borrow().clone()
    }
    pub fn editor(&self) -> Option<Rc<EditorController>> {
        self.editor.borrow().clone()
    }
    pub fn attributes(&self) -> Arc<InputAttributes> {
        self.target()
            .map_or_else(InputAttributes::shared_default, |t| t.attributes())
    }
    pub fn is_incognito(&self) -> bool {
        self.settings.incognito_mode()
            || self.attributes().is_private()
            || self.attributes().is_password()
    }
    pub fn clock(&self) -> Option<EditorClock> {
        self.clock.borrow().clone()
    }
    pub fn set_clock(&self, clock: EditorClock) {
        self.set_clock_nullable(Some(clock));
    }
    /// Runtime null is preserved, including the snapshot copied into a newly attached editor.
    pub fn set_clock_nullable(&self, clock: Option<EditorClock>) {
        let previous = self.clock.replace(clock);
        drop(previous);
    }
    pub fn punctuation_rule(&self) -> Arc<PunctuationRule> {
        self.resources
            .get_punctuation_rule(&self.active_subtype().punctuation_rule())
    }
    pub fn emoji_catalog(&self) -> EngineResult<Arc<EmojiCatalog>> {
        Ok(EmojiCatalog::load_nullable(
            self.active_subtype().language_tag().as_ref(),
        )?)
    }
    pub fn emoji_history(&self) -> Rc<EmojiHistory> {
        self.emoji_history.clone()
    }
    pub fn clipboard_history(&self) -> Rc<ClipboardHistory> {
        self.clipboard_history.clone()
    }
    pub fn text_committed(&self) -> &Event<Utf16Text> {
        &self.text_committed
    }

    pub fn attach(self: &Rc<Self>, target: Rc<dyn ITextInputTarget>) -> EngineResult<()> {
        if self.target().is_some_and(|t| Rc::ptr_eq(&t, &target)) {
            return Ok(());
        }
        self.detach()?;
        *self.target.borrow_mut() = Some(target.clone());
        let editor = Rc::new(EditorController::new(target.clone()));
        editor.set_clock_nullable(self.clock());
        *self.editor.borrow_mut() = Some(editor);
        let receiver = Rc::downgrade(self);
        self.target_token
            .set(Some(target.changed().add(Rc::new(move |_| {
                if let Some(receiver) = receiver.upgrade() {
                    receiver
                        .on_target_changed()
                        .expect("KeyboardEngine target callback failed");
                }
            }))));
        *self.last_auto_correction.borrow_mut() = None;
        self.last_insert_was_auto_space.set(false);
        let initial_mode = target.attributes().initial_mode();
        self.panel.set(KeyboardUiMode::Text);
        self.mode.set(initial_mode);
        self.shift.assign_state(ShiftState::Unshifted);
        self.invalidate_keyboard()?;
        self.update_auto_capitalization()?;
        self.update_suggestions()?;
        self.state_changed.raise(self);
        Ok(())
    }
    pub fn detach(&self) -> EngineResult<()> {
        if self.target().is_none() {
            return Ok(());
        }
        self.finish_glide(false)?;
        if let Some(token) = self.target_token.take() {
            self.target()
                .expect("source target remains attached")
                .changed()
                .remove(token);
        }
        *self.target.borrow_mut() = None;
        *self.editor.borrow_mut() = None;
        self.set_suggestions(SuggestionList::default())?;
        self.state_changed.raise(self);
        Ok(())
    }
    fn on_target_changed(&self) -> EngineResult<()> {
        if self.suppress_target_changed.get() {
            return Ok(());
        }
        *self.last_auto_correction.borrow_mut() = None;
        self.last_insert_was_auto_space.set(false);
        if let Some(editor) = self.editor() {
            editor.end_selection_mode();
        }
        if let Some(editor) = self.editor() {
            editor.break_undo_group();
        }
        self.update_auto_capitalization()?;
        self.update_suggestions()
    }
    pub(super) fn without_target_events<T>(
        &self,
        action: impl FnOnce() -> EngineResult<T>,
    ) -> EngineResult<T> {
        struct Restore<'a>(&'a Cell<bool>, bool);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }
        let restore = Restore(
            &self.suppress_target_changed,
            self.suppress_target_changed.replace(true),
        );
        let result = action();
        drop(restore);
        result
    }
    pub fn input_text(&self, text: &Utf16Text) -> EngineResult<()> {
        self.input_text_nullable(Some(text))
    }
    pub fn input_text_nullable(&self, text: Option<&Utf16Text>) -> EngineResult<()> {
        if self.editor().is_none() {
            return Ok(());
        }
        let text = text.ok_or(EngineError::NullReference)?;
        if text.is_empty() {
            return Ok(());
        }
        *self.last_auto_correction.borrow_mut() = None;
        self.last_insert_was_auto_space.set(false);
        self.without_target_events(|| {
            self.editor()
                .ok_or(EngineError::NoEditor)?
                .commit_text(text)?;
            Ok(())
        })?;
        self.after_edit(Some(text))
    }
    pub(super) fn after_edit(&self, committed: Option<&Utf16Text>) -> EngineResult<()> {
        if let Some(text) = committed {
            self.text_committed.raise(text);
        }
        self.update_auto_capitalization()?;
        self.update_suggestions()
    }
}
