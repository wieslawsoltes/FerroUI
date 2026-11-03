//! KeyboardEngine.Suggestions.cs glide session methods. Classifier algorithms live in glide/.
use super::*;
use crate::text::{SuggestionKind, SuggestionValues, TextUtils};
impl KeyboardEngine {
    pub fn is_gliding(&self) -> bool {
        self.glide_active.get()
    }
    pub fn can_glide(&self) -> bool {
        self.settings.glide_typing()
            && self.mode() == KeyboardMode::Characters
            && self.ui_mode() == KeyboardUiMode::Text
            && self.attributes().supports_nlp()
            && self
                .language_models
                .has_dictionary_nullable(self.active_subtype().language_tag().as_ref())
                .unwrap_or_else(|e| std::panic::panic_any(e))
    }
    pub fn begin_glide(&self, x: f64, y: f64) -> EngineResult<()> {
        if !self.can_glide() {
            return Ok(());
        }
        if !self.glide.is_ready() {
            self.update_glide_words()?;
            let keyboard = self.keyboard.borrow().clone();
            if let Some(keyboard) = keyboard {
                let keys = keyboard.character_keys()?;
                self.glide
                    .set_computed_layout(keys.iter().map(|k| k.as_ref()))?;
            }
        }
        self.glide_active.set(true);
        self.glide_preview_points.set(0);
        self.glide.clear();
        self.glide.add_point(x, y);
        self.state_changed.raise(self);
        Ok(())
    }
    pub fn add_glide_point(&self, x: f64, y: f64) -> EngineResult<()> {
        if !self.glide_active.get() {
            return Ok(());
        }
        self.glide.add_point(x, y);
        if self.settings.glide_preview() {
            self.glide_preview_points
                .set(self.glide_preview_points.get().wrapping_add(1));
            if self.glide_preview_points.get() >= 6 {
                self.glide_preview_points.set(0);
                let words = self
                    .glide
                    .get_suggestions(self.settings.suggestion_count())?;
                *self.suggestions.borrow_mut() = SuggestionList::new(
                    words
                        .iter()
                        .map(|w| {
                            Rc::new(Suggestion::from_values(SuggestionValues {
                                confidence: 1.0,
                                ..SuggestionValues::new(
                                    self.apply_glide_case(w),
                                    SuggestionKind::Glide,
                                )
                            }))
                        })
                        .collect(),
                );
                self.suggestions_changed.raise(self);
            }
        }
        Ok(())
    }
    pub fn finish_glide(&self, commit: bool) -> EngineResult<()> {
        if !self.glide_active.get() {
            return Ok(());
        }
        self.glide_active.set(false);
        if !commit || self.editor().is_none() {
            self.glide.clear();
            self.update_suggestions()?;
            self.state_changed.raise(self);
            return Ok(());
        }
        let words = self
            .glide
            .get_suggestions(self.settings.suggestion_count().max(3))?
            .iter()
            .map(|w| self.apply_glide_case(w))
            .collect::<Vec<_>>();
        self.glide.clear();
        let Some(best) = words.first() else {
            self.update_suggestions()?;
            self.state_changed.raise(self);
            return Ok(());
        };
        *self.last_auto_correction.borrow_mut() = None;
        self.without_target_events(|| {
            let before = self.editor_required()?.get_text_before_cursor(1)?;
            let needs = before.units().first().is_some_and(|c| {
                !TextUtils::is_white_space(*c) && ![40, 91, 123, 0xbf, 0xa1, 34, 39].contains(c)
            });
            let prefix = if needs {
                " ".into()
            } else {
                Utf16Text::default()
            };
            self.editor_required()?
                .commit_text(&super::input::concat(&prefix, best))?;
            Ok(())
        })?;
        *self.last_glide_word.borrow_mut() = Some(best.clone());
        self.last_insert_was_auto_space.set(false);
        self.feedback(FeedbackKind::KeyPress);
        self.text_committed.raise(best);
        self.shift.after_character();
        self.update_auto_capitalization()?;
        *self.suggestions.borrow_mut() = SuggestionList::new(
            words
                .into_iter()
                .map(|w| {
                    Rc::new(Suggestion::from_values(SuggestionValues {
                        confidence: 1.0,
                        ..SuggestionValues::new(w, SuggestionKind::Glide)
                    }))
                })
                .collect(),
        );
        self.suggestions_changed.raise(self);
        self.state_changed.raise(self);
        Ok(())
    }
    fn apply_glide_case(&self, word: &Utf16Text) -> Utf16Text {
        match self.shift_state() {
            ShiftState::CapsLock => self.culture().to_upper(word),
            ShiftState::ShiftedManual | ShiftState::ShiftedAutomatic => {
                TextUtils::capitalize(word, self.culture())
            }
            _ => word.clone(),
        }
    }
}
