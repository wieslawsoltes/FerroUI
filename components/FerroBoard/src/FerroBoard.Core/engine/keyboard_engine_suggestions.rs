//! KeyboardEngine.Suggestions.cs provider resolution and UpdateSuggestions, including source guards.
use super::*;
use crate::{
    clipboard::OffsetDateTime,
    emoji::EmojiInfo,
    text::{
        DictionarySuggestionProvider, KeyProximity, SuggestionKind, SuggestionRequest,
        SuggestionRequestValues, SuggestionValues, TextUtils,
    },
};
#[path = "shortcode_numbers.rs"]
mod numbers;
impl From<EmojiError> for EngineError {
    fn from(e: EmojiError) -> Self {
        Self::Emoji(e)
    }
}
impl From<SuggestionError> for EngineError {
    fn from(e: SuggestionError) -> Self {
        Self::Suggestion(e)
    }
}
impl From<NormalizationError> for EngineError {
    fn from(e: NormalizationError) -> Self {
        Self::Normalization(e)
    }
}
impl KeyboardEngine {
    pub fn suggestions(&self) -> SuggestionList {
        self.suggestions.borrow().clone()
    }
    pub fn suggestions_changed(&self) -> &Event<Self> {
        &self.suggestions_changed
    }
    pub fn suggestion_provider_factory(&self) -> Option<SuggestionProviderFactory> {
        self.provider_factory.borrow().clone()
    }
    pub fn set_suggestion_provider_factory(&self, factory: Option<SuggestionProviderFactory>) {
        let old = std::mem::replace(&mut *self.provider_factory.borrow_mut(), factory);
        drop(old); // Captured native destructors may reenter factory access.
    }
    pub fn suggestion_provider(&self) -> EngineResult<Option<Rc<dyn ISuggestionProvider>>> {
        if let Some(factory) = self.suggestion_provider_factory()
            && let Some(custom) = factory(Some(&self.active_subtype()))?
        {
            return Ok(Some(custom));
        }
        let language = self
            .active_subtype()
            .language_tag()
            .ok_or(EngineError::NullReference)?;
        let language = Utf16Text::from_units(
            language
                .units()
                .split(|&c| c == 45)
                .next()
                .unwrap()
                .to_vec(),
        );
        let key = OrdinalKey(language.clone());
        let provider = self.providers.borrow().get(&key).cloned();
        if provider.is_some() {
            return Ok(provider);
        }
        let Some(dictionary) = self.language_models.get_dictionary(&language)? else {
            return Ok(None);
        };
        let provider: Rc<dyn ISuggestionProvider> = Rc::new(DictionarySuggestionProvider::new(
            dictionary,
            Some(self.language_models.get_user_dictionary(&language)),
        ));
        self.providers.borrow_mut().insert(key, provider.clone());
        Ok(Some(provider))
    }
    pub fn update_suggestions(&self) -> EngineResult<()> {
        if self.glide_active.get() {
            return Ok(());
        }
        if self.editor().is_none() || self.ui_mode() != KeyboardUiMode::Text {
            self.set_suggestions(SuggestionList::default())?;
            return Ok(());
        }
        let mut list = Vec::new();
        let max = self.settings.suggestion_count();
        let before = self
            .editor()
            .ok_or(EngineError::NoEditor)?
            .get_text_before_cursor(64)?;
        if self.settings.emoji_suggestions()
            && !self.attributes().is_password()
            && let Some(query) = shortcode(&before)
        {
            let query = Utf16Text::from_units(
                query
                    .units()
                    .iter()
                    .map(|&c| if c == 95 { 32 } else { c })
                    .collect::<Vec<_>>(),
            );
            for emoji in self.emoji_catalog()?.search(&query, max) {
                list.push(Some(self.emoji_suggestion(&emoji, 1.0)?));
            }
            self.set_suggestions(SuggestionList::from_nullable(list))?;
            return Ok(());
        }
        let nlp = self.settings.show_suggestions() && self.attributes().supports_nlp();
        let word = if self
            .editor()
            .ok_or(EngineError::NoEditor)?
            .is_composing_word()?
        {
            self.editor().ok_or(EngineError::NoEditor)?.current_word()?
        } else {
            Utf16Text::default()
        };
        if word.is_empty()
            && self.settings.clipboard_suggestion()
            && !self.is_incognito()
            && let Some(clip) = self.clipboard_history.primary()
            && self.last_clipboard_suggestion.borrow().as_ref() != Some(&clip.text)
            && OffsetDateTime::now_utc().unix_timestamp_nanos()
                - clip.timestamp().unix_timestamp_nanos()
                < 60_000_000_000
        {
            list.push(Some(Rc::new(Suggestion::from_values(SuggestionValues {
                confidence: 1.0,
                description: Some("clipboard".into()),
                ..SuggestionValues::new(clip.text.clone(), SuggestionKind::Clipboard)
            }))));
        }
        if nlp && let Some(provider) = self.suggestion_provider()? {
            if !word.is_empty() || self.settings.next_word_prediction() {
                let end = before
                    .len()
                    .checked_sub(word.len())
                    .ok_or(EditorError::SubstringOutOfRange)?;
                let request = SuggestionRequest::from_values(SuggestionRequestValues {
                    word: Some(word.clone()),
                    preceding_text: Some(Utf16Text::from_units(before.units()[..end].to_vec())),
                    max_count: max,
                    allow_auto_correct: self.settings.auto_correct()
                        && self.attributes().allow_auto_correct()
                        && !self
                            .rejected_corrections
                            .borrow()
                            .contains(&OrdinalKey(word.clone())),
                    culture: Some(self.culture()),
                    key_proximity: self.create_key_proximity()?,
                });
                // This is the source's intentional provider failure isolation. Request construction,
                // provider factory and emoji failures above/below are deliberately outside it.
                // The original try/catch covers invocation, null results and AddRange itself.
                // Preserve partial appends from a custom fallible native iterator.
                // Iterators release their captures through Rust Drop, including early failure.
                let _ = provider.suggest(Some(&request)).and_then(|suggestions| {
                    suggestions
                        .ok_or(SuggestionError::Collection(
                            crate::layouts::ResourceTextListError::ArgumentNull,
                        ))?
                        .append_to(&mut list)
                });
            }
            if self.settings.emoji_suggestions() && word.len() >= 3 {
                let lower = TextCulture::to_lower_invariant(&word);
                let mut found = None;
                for emoji in self.emoji_catalog()?.search(&word, 1) {
                    let name_matches = emoji
                        .name
                        .units()
                        .split(|&c| c == 32)
                        .any(|part| part == lower.units());
                    let mut keyword_matches = false;
                    let keywords = emoji.keywords.lock().clone();
                    for keyword in &keywords {
                        if keyword == &lower {
                            keyword_matches = true;
                            break;
                        }
                    }
                    if name_matches || keyword_matches {
                        found = Some(emoji);
                        break;
                    }
                }
                if let Some(emoji) = found
                    && !list.is_empty()
                {
                    if list.len() as i32 >= max {
                        list.pop();
                    }
                    list.push(Some(self.emoji_suggestion(&emoji, 0.5)?));
                }
            }
        }
        let count = max.max(1)
            + i32::from(
                list.first()
                    .and_then(Option::as_ref)
                    .is_some_and(|s| s.kind() == SuggestionKind::Clipboard),
            );
        list.truncate(count as usize);
        self.set_suggestions(SuggestionList::from_nullable(list))?;
        Ok(())
    }
    pub(super) fn emoji_suggestion(
        &self,
        emoji: &EmojiInfo,
        confidence: f64,
    ) -> EngineResult<Rc<Suggestion>> {
        let tone = self
            .emoji_history
            .get_preferred_tone(&emoji.value)
            .unwrap_or_else(|| self.settings.emoji_skin_tone());
        Ok(Rc::new(Suggestion::from_values(SuggestionValues {
            confidence,
            description: Some(emoji.name.clone()),
            ..SuggestionValues::new(emoji.with_skin_tone(tone)?, SuggestionKind::Emoji)
        })))
    }
    pub(super) fn set_suggestions(&self, suggestions: SuggestionList) -> EngineResult<()> {
        if self.suggestions().len()? == 0 && suggestions.len()? == 0 {
            return Ok(());
        }
        *self.suggestions.borrow_mut() = suggestions;
        self.suggestions_changed.raise(self);
        Ok(())
    }
    fn create_key_proximity(&self) -> EngineResult<Option<KeyProximity>> {
        let keyboard = self.keyboard.borrow().clone();
        let Some(keyboard) = keyboard else {
            return Ok(None);
        };
        if self.mode() != KeyboardMode::Characters || self.width.get() <= 0.0 {
            return Ok(None);
        }
        let mut centers = HashMap::new();
        let mut width: f64 = 0.0;
        for key in keyboard.character_keys()? {
            let output = key
                .data()
                .as_string(false)?
                .ok_or(crate::layouts::KeyDataError::NullReference)?;
            if output.len() == 1 {
                let c = TextUtils::base_char(output.units()[0])?;
                let bounds = key.touch_bounds();
                centers.insert(c, (bounds.center_x(), bounds.center_y()));
                // Math.Max propagates NaNs; f64::max alone would drop one.
                width = if width.is_nan() || bounds.width.is_nan() {
                    f64::NAN
                } else {
                    width.max(bounds.width)
                };
            }
        }
        if centers.is_empty() || width <= 0.0 {
            return Ok(None);
        }
        let threshold = width * 1.5;
        Ok(Some(Rc::new(move |a, b| {
            if let (Some(pa), Some(pb)) = (centers.get(&a), centers.get(&b)) {
                let dx = pa.0 - pb.0;
                let dy = pa.1 - pb.1;
                if (dx * dx + dy * dy).sqrt() <= threshold {
                    return 0.6;
                }
            }
            1.0
        })))
    }
}
/// Exact source regex suffix: UTF-16 categories L/N, >=2 code units, whitespace boundary,
/// with .NET `$` permitting one trailing LF. Surrogates and combining marks aren't L/N.
pub(super) fn shortcode(text: &Utf16Text) -> Option<Utf16Text> {
    let units = text.units();
    let end = units.len() - usize::from(units.last() == Some(&10));
    let colon = units[..end].iter().rposition(|&c| c == 58)?;
    let query = &units[colon + 1..end];
    if colon > 0 && !TextUtils::is_white_space(units[colon - 1]) || query.len() < 2 {
        return None;
    }
    if !query.iter().all(|&c| {
        crate::text::is_letter_unit(c) || numbers::is_number(c) || matches!(c, 95 | 43 | 45)
    }) {
        return None;
    }
    Some(Utf16Text::from_units(query.to_vec()))
}

impl KeyboardEngine {
    pub(super) fn apply_auto_correction(&self, separator: &Utf16Text) -> EngineResult<()> {
        if self.editor().is_none()
            || !self.settings.auto_correct()
            || !self.attributes().allow_auto_correct()
            || !self.attributes().supports_nlp()
            || !self.settings.show_suggestions()
        {
            return Ok(());
        }
        if !self.editor_required()?.is_composing_word()? {
            return Ok(());
        }
        let word = self.editor_required()?.current_word()?;
        if self
            .rejected_corrections
            .borrow()
            .contains(&OrdinalKey(word.clone()))
        {
            return Ok(());
        }
        let correction = self.suggestions().first(|s| {
            let s = s.ok_or(SuggestionError::NullReference)?;
            Ok(s.is_auto_commit() && s.kind() == SuggestionKind::Correction)
        })?;
        let Some(correction) = correction.filter(|c| c.text().as_ref() != Some(&word)) else {
            return Ok(());
        };
        self.without_target_events(|| {
            Ok(self
                .editor_required()?
                .replace_current_word_nullable(correction.text().as_ref())?)
        })?;
        *self.last_auto_correction.borrow_mut() = Some(AutoCorrection {
            original: word,
            corrected: correction.text(),
            separator: separator.clone(),
            end: self
                .editor_required()?
                .selection_start()
                .wrapping_add(separator.len() as i32),
        });
        Ok(())
    }
    pub(super) fn learn_current_word(&self) -> EngineResult<()> {
        if self.editor().is_none()
            || self.is_incognito()
            || !self.settings.learn_words()
            || !self.attributes().supports_nlp()
            || !self.editor_required()?.is_composing_word()?
        {
            return Ok(());
        }
        let word = self.editor_required()?.current_word()?;
        let before = self.editor_required()?.get_text_before_cursor(96)?;
        let end = before
            .len()
            .checked_sub(word.len())
            .ok_or(EditorError::SubstringOutOfRange)?;
        let context = &before.units()[..end];
        let end = context
            .iter()
            .rposition(|&c| !TextUtils::is_white_space(c))
            .map_or(0, |i| i + 1);
        let context = &context[..end];
        let previous = if context
            .last()
            .is_some_and(|c| TextUtils::is_word_core_char(*c))
        {
            Some(TextUtils::get_current_word(&Utf16Text::from_units(
                context.to_vec(),
            )))
        } else {
            None
        };
        if let Some(provider) = self.suggestion_provider()? {
            provider.learn(Some(&word), previous.as_ref())?;
        }
        Ok(())
    }
    pub fn commit_suggestion(&self, suggestion: &Suggestion) -> EngineResult<()> {
        self.commit_suggestion_nullable(Some(suggestion))
    }
    pub fn commit_suggestion_nullable(&self, suggestion: Option<&Suggestion>) -> EngineResult<()> {
        if self.editor().is_none() {
            return Ok(());
        }
        self.feedback(FeedbackKind::KeyPress);
        *self.last_auto_correction.borrow_mut() = None;
        let suggestion = suggestion.ok_or(EngineError::NullReference)?;
        match suggestion.kind() {
            SuggestionKind::Clipboard => {
                *self.last_clipboard_suggestion.borrow_mut() = suggestion.text();
                return self.input_text_nullable(suggestion.text().as_ref());
            }
            SuggestionKind::Emoji => {
                let before = self.editor_required()?.get_text_before_cursor(64)?;
                let query = shortcode(&before);
                self.without_target_events(|| {
                    if let Some(query) = query {
                        self.editor_required()?.replace_before_cursor_nullable(
                            (query.len() as i32).wrapping_add(1),
                            suggestion.text().as_ref(),
                        )?;
                    } else {
                        // Receiver evaluation precedes argument evaluation in the source.
                        let editor = self.editor_required()?;
                        let prefix = if self.editor_required()?.is_composing_word()? {
                            " ".into()
                        } else {
                            Utf16Text::default()
                        };
                        editor.commit_text(&super::input::concat_nullable(
                            Some(&prefix),
                            suggestion.text().as_ref(),
                        ))?;
                    }
                    Ok(())
                })?;
                if !self.is_incognito() && self.settings.emoji_history_enabled() {
                    self.emoji_history
                        .add_nullable(suggestion.text().as_ref())?;
                }
                return self.after_edit(suggestion.text().as_ref());
            }
            _ => {}
        }
        let word = if self.editor_required()?.is_composing_word()? {
            self.editor_required()?.current_word()?
        } else {
            Utf16Text::default()
        };
        if suggestion.kind() == SuggestionKind::Typed {
            self.rejected_corrections
                .borrow_mut()
                .insert(OrdinalKey(word.clone()));
        }
        self.without_target_events(|| {
            if suggestion.kind() == SuggestionKind::Prediction && word.is_empty() {
                let before = self.editor_required()?.get_text_before_cursor(1)?;
                let prefix = if before
                    .units()
                    .first()
                    .is_some_and(|c| !TextUtils::is_white_space(*c))
                {
                    " ".into()
                } else {
                    Utf16Text::default()
                };
                self.editor_required()?
                    .commit_text(&super::input::concat_nullable(
                        Some(&prefix),
                        suggestion.text().as_ref(),
                    ))?;
            } else {
                // Both source branches replace the current word, but its glide condition reads
                // CurrentWord (which may invoke an extension target) before choosing that branch.
                let last = self.last_glide_word.borrow().clone();
                let _last_glide_match = suggestion.kind() == SuggestionKind::Glide
                    && last.is_some()
                    && Some(self.editor_required()?.current_word()?) == last;
                self.editor_required()?
                    .replace_current_word_nullable(suggestion.text().as_ref())?;
            }
            Ok(())
        })?;
        if suggestion.kind() == SuggestionKind::Glide {
            *self.last_glide_word.borrow_mut() = suggestion.text();
            return self.after_edit(suggestion.text().as_ref());
        }
        self.learn_current_word()?;
        if !self.is_followed_by_space()? {
            self.without_target_events(|| Ok(self.editor_required()?.commit_text(&" ".into())?))?;
            self.last_insert_was_auto_space.set(true);
        }
        self.shift.after_character();
        self.after_edit(suggestion.text().as_ref())
    }
    pub fn remove_suggestion(&self, suggestion: &Suggestion) -> EngineResult<()> {
        self.remove_suggestion_nullable(Some(suggestion))
    }
    pub fn remove_suggestion_nullable(&self, suggestion: Option<&Suggestion>) -> EngineResult<()> {
        if let Some(provider) = self.suggestion_provider()? {
            provider.forget(
                suggestion
                    .ok_or(EngineError::NullReference)?
                    .text()
                    .as_ref(),
            )?;
        }
        self.update_suggestions()
    }
}

impl From<crate::layouts::ResourceTextListError> for EngineError {
    fn from(e: crate::layouts::ResourceTextListError) -> Self {
        Self::Suggestion(SuggestionError::Collection(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    #[test]
    fn source_regex_all_bmp_units_and_utf16_boundary_cases() {
        let rows = include_str!("../../../tests/fixtures/engine-target.jsonl")
            .lines()
            .map(|s| serde_json::from_str::<Value>(s).unwrap())
            .collect::<Vec<_>>();
        let flags = (0..=65535)
            .map(|c| u8::from(shortcode(&Utf16Text::from_units(vec![58, 97, 97, c])).is_some()))
            .collect::<Vec<_>>();
        assert_eq!(
            format!("{:x}", Sha256::digest(flags)),
            rows.iter().find(|r| r["kind"] == "regex-bmp").unwrap()["sha256"]
        );
        for row in rows.iter().filter(|r| r["kind"] == "regex") {
            let text = Utf16Text::from_units(
                row["text"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_u64().unwrap() as u16)
                    .collect::<Vec<_>>(),
            );
            assert_eq!(
                shortcode(&text)
                    .map(|s| json!(s.units()))
                    .unwrap_or(Value::Null),
                row["query"],
                "{row}"
            );
        }
    }
}
