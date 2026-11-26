//! Spell checking of a text box (#1261): `IsSpellCheckEnabled` through the spell checker of the
//! platform ([`ISpellChecker`]), the wavy underlines of its presenter and the proofing items of
//! its context flyout.
//!
//! WinUI's TextBox checks spelling through the text services of Windows (TextBoxBase.cpp,
//! `OnIsSpellCheckEnabledChanged`), and its proofing menu (`UpdateProofingMenu`) has up to three
//! suggestions, a separator, "Add to dictionary" and "Ignore". FerroUI checks the committed text
//! of the box at the background priority of the dispatcher after each change, keeps the
//! underlines of the words an edit does not touch until the result of the next check arrives, and
//! puts the proofing items at the top of the box's context menu flyout while it is open for a
//! misspelled word (WinUI's TextCommandBarFlyout opens the proofing menu from its own button).

use super::TextBox;
use crate::documents::TextElement;
use crate::{Control, MenuFlyout, MenuItem, Separator};
use ferroui_base::controls::ResourceKey;
use ferroui_base::input::ContextRequestedEventArgs;
use ferroui_base::platform::{spell_checker, ISpellChecker, SpellingError};
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{BoxedValue, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The number of suggestions of the proofing items (WinUI's `maxSuggestionCount`).
const MAX_SUGGESTION_COUNT: usize = 3;

/// The state of a text box that checks its spelling.
#[derive(Default)]
pub(crate) struct SpellCheckState {
    /// Advanced by every change that makes a pending result stale.
    generation: Cell<u64>,
    /// Whether a check is posted to the dispatcher.
    pending: Cell<bool>,
    /// The proofing items put into the context flyout that is open.
    proofing_items: RefCell<Option<ProofingItems>>,
}

/// The proofing items of an open context flyout, removed when it closes.
struct ProofingItems {
    flyout: Ref<MenuFlyout>,
    items: Vec<Option<BoxedValue>>,
    _closed: Subscription,
}

impl TextBox {
    /// Whether the spelling of the text box is checked: `IsSpellCheckEnabled`, not a password
    /// box, editable, and a spell checker is registered.
    fn spell_check_applies(&self) -> bool {
        self.is_spell_check_enabled() && self.password_char() == '\0' && !self.is_read_only()
    }

    /// Starts or stops checking the spelling after a change of what decides it (the property,
    /// the mask, read-only, the template, the visual tree).
    pub(super) fn update_spell_check(&self) {
        let checker = self.spell_check_applies().then(spell_checker).flatten();
        if checker.is_none() || !self.is_attached_to_visual_tree() {
            self.stop_spell_check();
            return;
        }
        self.schedule_spell_check();
    }

    fn stop_spell_check(&self) {
        let state = self.spell_check.borrow_mut().take();
        if let Some(state) = state {
            state.remove_proofing_items();
        }
        if let Some(presenter) = self.presenter() {
            presenter.set_spelling_errors(None);
        }
    }

    /// Keeps the underlines of the words an edit of the text does not touch (moved by the
    /// length the edit adds or removes) and checks the new text.
    pub(super) fn on_spell_check_text_changed(&self, old_text: &str, new_text: &str) {
        if self.spell_check.borrow().is_none() {
            return;
        }
        if let Some(presenter) = self.presenter() {
            if let Some(errors) = presenter.spelling_errors() {
                let old: Vec<u16> = old_text.encode_utf16().collect();
                let new: Vec<u16> = new_text.encode_utf16().collect();
                presenter
                    .set_spelling_errors(Some(adjust_spelling_errors(&errors, &old, &new).into()));
            }
        }
        self.schedule_spell_check();
    }

    fn schedule_spell_check(&self) {
        let mut state = self.spell_check.borrow_mut();
        let state = state.get_or_insert_with(Default::default);
        state.generation.set(state.generation.get().wrapping_add(1));
        if state.pending.replace(true) {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.run_spell_check();
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }

    fn run_spell_check(&self) {
        let generation = {
            let state = self.spell_check.borrow();
            let Some(state) = state.as_ref() else { return };
            state.pending.set(false);
            state.generation.get()
        };
        let Some(checker) = self.spell_check_applies().then(spell_checker).flatten() else {
            self.stop_spell_check();
            return;
        };
        let text: Rc<[u16]> = self.text_utf16().into();
        let language = self.spell_check_language();
        let weak = self.to_ref().downgrade();
        let length = text.len();
        checker.check_spelling(
            text,
            language.as_deref(),
            Box::new(move |errors| {
                let Some(this) = weak.upgrade() else { return };
                let current = this
                    .spell_check
                    .borrow()
                    .as_ref()
                    .map(|state| state.generation.get());
                if current != Some(generation) {
                    return;
                }
                let caret_index = this.caret_index().max(0) as usize;
                let typing = this.is_focused() && this.selection_start() == this.selection_end();
                let errors: Vec<SpellingError> = errors
                    .into_iter()
                    .filter(|error| error.length > 0 && error.end() <= length)
                    // The word the caret ends is still being typed.
                    .filter(|error| !(typing && error.end() == caret_index))
                    .collect();
                if let Some(presenter) = this.presenter() {
                    presenter.set_spelling_errors(Some(errors.into()));
                }
            }),
        );
    }

    /// The language the text is checked in: the BCP 47 tag of `TextElement.Language`, `None`
    /// for the invariant culture (the checker then uses the language of the user's settings).
    fn spell_check_language(&self) -> Option<String> {
        let language = self.get_value(TextElement::language_property());
        Some(crate::documents::text_language::language_tag(&language)).filter(|tag| !tag.is_empty())
    }

    /// The misspelled word at a text position of the committed text, with its text.
    fn spelling_error_at(&self, position: usize) -> Option<(SpellingError, String)> {
        let errors = self.presenter()?.spelling_errors()?;
        let error = *errors
            .iter()
            .find(|error| error.start <= position && position <= error.end())?;
        let text = self.text_utf16();
        let word = String::from_utf16_lossy(text.get(error.start..error.end())?);
        Some((error, word))
    }

    /// Puts the proofing items of the misspelled word the context is requested for (at the
    /// pointer, or else at the caret) at the top of the box's menu flyout. Runs before the
    /// flyout opens (a class handler).
    pub(super) fn on_spell_check_context_requested(&self, e: &ContextRequestedEventArgs) {
        if e.handled() {
            return;
        }
        match self.spell_check.borrow().as_ref() {
            Some(state) => state.remove_proofing_items(),
            None => return,
        }
        let Some(presenter) = self.presenter() else {
            return;
        };
        if presenter
            .preedit_text()
            .is_some_and(|text| !text.is_empty())
        {
            return;
        }
        let position = match e.try_get_position(Some(&presenter)) {
            Some(point) => presenter
                .text_layout()
                .hit_test_point(point)
                .text_position(),
            None => self.caret_index(),
        };
        let Some((error, word)) = self.spelling_error_at(position.max(0) as usize) else {
            return;
        };
        let Some(checker) = spell_checker() else {
            return;
        };
        let Some(flyout) = self
            .context_flyout()
            .and_then(|flyout| flyout.cast::<MenuFlyout>())
        else {
            return;
        };
        if flyout.items_source().is_some() {
            return;
        }

        let items = self.proofing_items(&checker, error, &word);
        for (index, item) in items.iter().enumerate() {
            flyout.items().insert(index, item.clone());
        }
        let weak = self.to_ref().downgrade();
        let closed = flyout.closed(move || {
            if let Some(this) = weak.upgrade() {
                // Removed after the close has finished.
                let weak = this.to_ref().downgrade();
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(this) = weak.upgrade() {
                            if let Some(state) = this.spell_check.borrow().as_ref() {
                                state.remove_proofing_items();
                            }
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
            }
        });
        if let Some(state) = self.spell_check.borrow().as_ref() {
            *state.proofing_items.borrow_mut() = Some(ProofingItems {
                flyout,
                items,
                _closed: closed,
            });
        }
    }

    /// The proofing items of a misspelled word: up to three suggestions and a separator, "Add
    /// to dictionary", "Ignore", and a separator before the commands of the flyout.
    fn proofing_items(
        &self,
        checker: &Rc<dyn ISpellChecker>,
        error: SpellingError,
        word: &str,
    ) -> Vec<Option<BoxedValue>> {
        let language = self.spell_check_language();
        let mut items = Vec::new();
        let suggestions = checker.suggestions(word, language.as_deref());
        for suggestion in suggestions.into_iter().take(MAX_SUGGESTION_COUNT) {
            let weak = self.to_ref().downgrade();
            let replacement = suggestion.clone();
            items.push(self.proofing_item(suggestion, move || {
                if let Some(this) = weak.upgrade() {
                    this.replace_misspelled_word(error, &replacement);
                }
            }));
        }
        if !items.is_empty() {
            items.push(Some(Control::boxed(Separator::new())));
        }
        let add = {
            let (weak, checker, word) =
                (self.to_ref().downgrade(), checker.clone(), word.to_owned());
            move || {
                checker.add_to_dictionary(&word);
                if let Some(this) = weak.upgrade() {
                    this.forget_misspelled_word(&word);
                }
            }
        };
        items.push(self.proofing_item(
            self.proofing_string("StringTextFlyoutAddToDictionaryText", "Add to dictionary"),
            add,
        ));
        let ignore = {
            let (weak, checker, word) =
                (self.to_ref().downgrade(), checker.clone(), word.to_owned());
            move || {
                checker.ignore_word(&word);
                if let Some(this) = weak.upgrade() {
                    this.forget_misspelled_word(&word);
                }
            }
        };
        items.push(self.proofing_item(
            self.proofing_string("StringTextFlyoutIgnoreText", "Ignore"),
            ignore,
        ));
        items.push(Some(Control::boxed(Separator::new())));
        items
    }

    fn proofing_item(&self, header: String, click: impl Fn() + 'static) -> Option<BoxedValue> {
        let item = MenuItem::new();
        item.set_header(Some(Rc::new(header)));
        // The handler lives as long as the item.
        let _token = item.click(move |_, _| click());
        Some(Control::boxed(item))
    }

    /// A string of the theme (`key`), or its English text.
    fn proofing_string(&self, key: &str, fallback: &str) -> String {
        self.try_find_resource(
            &ResourceKey::from(key),
            self.actual_theme_variant().as_ref(),
        )
        .flatten()
        .and_then(|value| {
            let value: &dyn ferroui_base::AnyValue = &*value;
            value.downcast_ref::<String>().cloned()
        })
        .unwrap_or_else(|| fallback.to_owned())
    }

    /// Replaces a misspelled word by a suggestion, as typing it over the selected word would
    /// (one step of undo).
    fn replace_misspelled_word(&self, error: SpellingError, replacement: &str) {
        let text = self.text_utf16();
        if error.end() > text.len() {
            return;
        }
        self.set_selection_start(error.start as i32);
        self.set_selection_end(error.end() as i32);
        self.handle_text_input(Some(replacement));
    }

    /// Removes the underlines of every occurrence of `word` and checks the text again.
    fn forget_misspelled_word(&self, word: &str) {
        if let Some(presenter) = self.presenter() {
            if let Some(errors) = presenter.spelling_errors() {
                let text = self.text_utf16();
                let word: Vec<u16> = word.encode_utf16().collect();
                let kept: Vec<SpellingError> = errors
                    .iter()
                    .copied()
                    .filter(|error| text.get(error.start..error.end()) != Some(&word[..]))
                    .collect();
                presenter.set_spelling_errors(Some(kept.into()));
            }
        }
        if self.spell_check.borrow().is_some() {
            self.schedule_spell_check();
        }
    }
}

impl SpellCheckState {
    fn remove_proofing_items(&self) {
        let items = self.proofing_items.borrow_mut().take();
        if let Some(items) = items {
            for item in &items.items {
                items.flyout.items().remove(item);
            }
        }
    }
}

/// The misspelled words of `old` that an edit to `new` does not touch, moved by the length the
/// edit adds or removes. The edit is the range between the common prefix and the common suffix
/// of the two texts; a word that overlaps it is dropped until the next check, and so is a word
/// it touches unless a separator (not a letter, a digit or an apostrophe) stays between them.
pub(crate) fn adjust_spelling_errors(
    errors: &[SpellingError],
    old: &[u16],
    new: &[u16],
) -> Vec<SpellingError> {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let max_suffix = old.len().min(new.len()) - prefix;
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    let (old_end, new_end) = (old.len() - suffix, new.len() - suffix);
    if old_end == prefix && new_end == prefix {
        return errors.to_vec();
    }
    let is_separator = |unit: Option<&u16>| {
        unit.is_some_and(|&unit| {
            char::from_u32(unit as u32)
                .is_some_and(|c| !c.is_alphanumeric() && c != '\'' && c != '\u{2019}')
        })
    };
    errors
        .iter()
        .filter_map(|error| {
            if error.end() < prefix
                || (error.end() == prefix
                    && is_separator(new.get(prefix))
                    && (old_end == prefix || is_separator(old.get(prefix))))
            {
                Some(*error)
            } else if error.start > old_end
                || (error.start == old_end
                    && new_end > 0
                    && is_separator(new.get(new_end - 1))
                    && (old_end == prefix || is_separator(old.get(old_end - 1))))
            {
                Some(SpellingError::new(
                    error.start - old_end + new_end,
                    error.length,
                ))
            } else {
                None
            }
        })
        .collect()
}
