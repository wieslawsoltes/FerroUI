use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_edit_core::document::{DocumentChangeEventArgs, TextDocument, TextSnapshot};
use ferroui_textmate::{Grammar, StateStack, Theme, Token, TokenStyle, TokenizeOptions};
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

#[derive(Clone)]
struct CachedLine {
    state: StateStack,
    tokens: Vec<Token>,
}

#[derive(Default)]
struct TokenCache {
    snapshot: Option<TextSnapshot>,
    lines: Vec<Option<CachedLine>>,
    invalid_from: Option<usize>,
    dirty_until: usize,
}

/// A grammar and theme attached to an editor. Persistent document snapshots
/// align retained line states across edits; tokenization stops when an unchanged
/// suffix has the same tokens and continuation state. Text is retrieved lazily
/// for the lines actually processed, without copying the document to a string.
pub struct TextMateInstallation {
    grammar: RefCell<Grammar>,
    theme: RefCell<Theme>,
    cache: RefCell<TokenCache>,
    error: RefCell<Option<String>>,
    options: Cell<TokenizeOptions>,
    generation: Cell<u64>,
    last_work_count: Cell<usize>,
    changed: Rc<HandlerList<dyn Fn()>>,
}

impl TextMateInstallation {
    pub fn new(grammar: Grammar, theme: Theme) -> Self {
        Self {
            grammar: RefCell::new(grammar),
            theme: RefCell::new(theme),
            cache: RefCell::new(TokenCache::default()),
            error: RefCell::new(None),
            options: Cell::new(TokenizeOptions::default()),
            generation: Cell::new(0),
            last_work_count: Cell::new(0),
            changed: Rc::new(HandlerList::new()),
        }
    }

    pub fn theme(&self) -> Ref<'_, Theme> {
        self.theme.borrow()
    }
    pub fn grammar(&self) -> Ref<'_, Grammar> {
        self.grammar.borrow()
    }
    pub fn last_error(&self) -> Option<String> {
        self.error.borrow().clone()
    }
    pub fn tokenize_options(&self) -> TokenizeOptions {
        self.options.get()
    }

    /// Number of lines scanned by the most recent advance. Reused suffix lines
    /// do not consume the caller's line budget.
    pub fn last_work_count(&self) -> usize {
        self.last_work_count.get()
    }

    /// Theme and grammar changes notify the host after all internal borrows
    /// have ended, so a subscriber may invalidate or inspect the editor.
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let handlers = self.changed.clone();
        let token = handlers.add(Rc::new(handler));
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    pub fn set_theme(&self, theme: Theme) {
        *self.theme.borrow_mut() = theme;
        self.raise_changed();
    }

    pub fn set_grammar(&self, grammar: Grammar) {
        *self.grammar.borrow_mut() = grammar;
        {
            let mut cache = self.cache.borrow_mut();
            cache.lines.fill(None);
            cache.invalid_from = (!cache.lines.is_empty()).then_some(0);
            cache.dirty_until = cache.lines.len();
        }
        self.bump_generation();
        self.error.borrow_mut().take();
        self.raise_changed();
    }

    pub fn set_tokenize_options(&self, options: TokenizeOptions) {
        self.options.set(options);
        self.bump_generation();
        self.error.borrow_mut().take();
        self.raise_changed();
    }

    pub fn invalidate_from(&self, line: usize) {
        let first = line.saturating_sub(1);
        {
            let mut cache = self.cache.borrow_mut();
            if first < cache.lines.len() {
                cache.invalid_from = Some(cache.invalid_from.map_or(first, |old| old.min(first)));
                cache.dirty_until = cache.dirty_until.max(first + 1);
            }
        }
        self.bump_generation();
        self.error.borrow_mut().take();
    }

    /// Processes at most line_budget lines and returns whether all states
    /// through the requested one-based line are valid. Work within each line is
    /// bounded by tokenize_options. No wall clock or worker is assumed.
    pub fn advance_to(&self, document: &TextDocument, line: usize, line_budget: usize) -> bool {
        self.advance(document, line, line_budget, None)
    }

    /// Cancellation is checked by the grammar between regex searches. A
    /// cancelled line is never published as a complete continuation state.
    pub fn advance_to_cancellable(
        &self,
        document: &TextDocument,
        line: usize,
        line_budget: usize,
        cancelled: &dyn Fn() -> bool,
    ) -> bool {
        self.advance(document, line, line_budget, Some(cancelled))
    }

    fn advance(
        &self,
        document: &TextDocument,
        line: usize,
        line_budget: usize,
        cancelled: Option<&dyn Fn() -> bool>,
    ) -> bool {
        self.last_work_count.set(0);
        if let Err(error) = self.synchronize(document.create_snapshot()) {
            *self.error.borrow_mut() = Some(error);
            return false;
        }
        let target = line.min(document.line_count());
        let grammar = self.grammar.borrow().clone();
        let options = self.options.get();
        for _ in 0..line_budget {
            let (index, previous, snapshot) = {
                let cache = self.cache.borrow();
                let Some(index) = cache.invalid_from.filter(|index| *index < target) else {
                    return true;
                };
                let previous = if index == 0 {
                    None
                } else {
                    cache.lines[index - 1]
                        .as_ref()
                        .map(|line| line.state.clone())
                };
                if index != 0 && previous.is_none() {
                    *self.error.borrow_mut() =
                        Some("Missing predecessor tokenization state".into());
                    return false;
                }
                (
                    index,
                    previous,
                    cache
                        .snapshot
                        .as_ref()
                        .expect("synchronized snapshot")
                        .clone(),
                )
            };
            let text = snapshot
                .get_line_by_number(index + 1)
                .and_then(|line| snapshot.get_text(line.offset(), line.length()));
            let text = match text {
                Ok(text) => text,
                Err(error) => {
                    *self.error.borrow_mut() = Some(error.to_string());
                    return false;
                }
            };
            let generation = self.generation.get();
            self.last_work_count.set(self.last_work_count.get() + 1);
            // No borrow of installation state is held across user cancellation.
            let was_cancelled = Cell::new(false);
            let cancellation = || {
                if was_cancelled.get() {
                    return true;
                }
                let value = cancelled.is_some_and(|callback| callback());
                was_cancelled.set(value);
                value
            };
            let result = grammar.tokenize_line_with_options(
                &text,
                previous.as_ref(),
                options,
                cancelled.map(|_| &cancellation as &dyn Fn() -> bool),
            );
            if generation != self.generation.get() {
                return false;
            }
            match result {
                Ok(result) if !result.stopped_early => {
                    let mut cache = self.cache.borrow_mut();
                    let converged = index >= cache.dirty_until
                        && cache.lines[index].as_ref().is_some_and(|old| {
                            old.state == result.rule_stack && old.tokens == result.tokens
                        });
                    cache.lines[index] = Some(CachedLine {
                        state: result.rule_stack,
                        tokens: result.tokens,
                    });
                    cache.invalid_from = if converged {
                        cache.dirty_until = 0;
                        cache.lines[index + 1..]
                            .iter()
                            .position(Option::is_none)
                            .map(|relative| index + 1 + relative)
                    } else {
                        (index + 1 < cache.lines.len()).then_some(index + 1)
                    };
                    if cache.invalid_from.is_none() {
                        cache.dirty_until = 0;
                    }
                    self.error.borrow_mut().take();
                }
                Ok(_) => {
                    if !was_cancelled.get() {
                        *self.error.borrow_mut() =
                            Some(format!("Grammar work limit reached on line {}", index + 1));
                    }
                    return false;
                }
                Err(error) => {
                    *self.error.borrow_mut() = Some(error.to_string());
                    return false;
                }
            }
        }
        self.cache
            .borrow()
            .invalid_from
            .is_none_or(|index| index >= target)
    }

    pub fn styled_tokens(&self, line: usize) -> Vec<(usize, usize, TokenStyle)> {
        let cache = self.cache.borrow();
        let Some(index) = line.checked_sub(1) else {
            return Vec::new();
        };
        if cache.invalid_from.is_some_and(|first| index >= first) {
            return Vec::new();
        }
        let Some(Some(line)) = cache.lines.get(index) else {
            return Vec::new();
        };
        let theme = self.theme.borrow();
        line.tokens
            .iter()
            .map(|token| {
                (
                    token.start_index,
                    token.end_index,
                    theme.match_scopes(&token.scopes),
                )
            })
            .collect()
    }

    fn synchronize(&self, snapshot: TextSnapshot) -> Result<(), String> {
        let mut cache = self.cache.borrow_mut();
        let previous = cache.snapshot.as_ref().cloned();
        let versions = previous
            .as_ref()
            .and_then(|old| Some((old.version()?, snapshot.version()?)));
        if versions.as_ref().is_some_and(|(old, new)| old == new) {
            return Ok(());
        }
        if let (Some(previous), Some((old, new))) = (previous, versions) {
            if old.belongs_to_same_document_as(&new) {
                let changes = old
                    .get_changes_to(&new)
                    .map_err(|error| error.to_string())?;
                if let Some((old_start, old_end, new_start, new_end)) = changed_bounds(&changes) {
                    // Include the preceding character so edits at CR/LF joins
                    // cannot leave a stale line boundary in the retained cache.
                    let first = previous
                        .get_line_by_offset(old_start.saturating_sub(1))
                        .map_err(|e| e.to_string())?
                        .line_number()
                        - 1;
                    let new_first = snapshot
                        .get_line_by_offset(new_start.saturating_sub(1))
                        .map_err(|e| e.to_string())?
                        .line_number()
                        - 1;
                    let first = first.min(new_first);
                    let old_last = previous
                        .get_line_by_offset(old_end.min(previous.text_length()))
                        .map_err(|e| e.to_string())?
                        .line_number();
                    let new_last = snapshot
                        .get_line_by_offset(new_end.min(snapshot.text_length()))
                        .map_err(|e| e.to_string())?
                        .line_number();
                    let removed = old_last - first;
                    let inserted = new_last - first;
                    cache
                        .lines
                        .splice(first..old_last, std::iter::repeat_n(None, inserted));
                    cache.lines.resize_with(snapshot.line_count(), || None);
                    let previous_invalid = cache.invalid_from.map(|index| {
                        if index >= old_last {
                            index - removed + inserted
                        } else {
                            index.min(first)
                        }
                    });
                    cache.invalid_from =
                        Some(previous_invalid.map_or(first, |index| index.min(first)));
                    cache.dirty_until = if cache.dirty_until >= old_last {
                        cache.dirty_until - removed + inserted
                    } else {
                        cache.dirty_until.min(first)
                    };
                    cache.dirty_until = cache.dirty_until.max(new_last);
                }
                cache.snapshot = Some(snapshot);
                self.bump_generation();
                return Ok(());
            }
        }
        cache.lines = vec![None; snapshot.line_count()];
        cache.invalid_from = Some(0);
        cache.dirty_until = 0;
        cache.snapshot = Some(snapshot);
        self.bump_generation();
        Ok(())
    }

    fn bump_generation(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
    }
    fn raise_changed(&self) {
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }
}

/// Bounds the union of sequential edits in both the original and final
/// coordinate spaces. The prefix before the union is unchanged; the suffix is
/// translated by one cumulative delta. This avoids reconstructing intermediate
/// document strings when several edits arrive before the next viewport pass.
fn changed_bounds(changes: &[DocumentChangeEventArgs]) -> Option<(usize, usize, usize, usize)> {
    let first = changes.first()?;
    let (mut old_start, mut old_end) = (first.offset, first.offset + first.removal_length());
    let (mut new_start, mut new_end) = (first.offset, first.offset + first.insertion_length());
    for change in &changes[1..] {
        let end = change.offset + change.removal_length();
        if change.offset < new_start {
            old_start = change.offset;
        }
        if end > new_end {
            old_end += end - new_end;
        }
        new_start = new_start.min(change.offset);
        new_end = new_end.max(end) - change.removal_length() + change.insertion_length();
    }
    Some((old_start, old_end, new_start, new_end))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_textmate::{Color, Registry};

    fn grammar() -> Grammar {
        let mut registry = Registry::new();
        registry.add_grammar_json(r##"{"scopeName":"source.test","patterns":[{"begin":"/\\*","end":"\\*/","name":"comment"},{"match":"\\d+","name":"constant.numeric"}]}"##).unwrap();
        registry.load_grammar("source.test").unwrap()
    }
    fn theme(color: &str) -> Theme {
        Theme::from_json(&format!(
            r##"{{"tokenColors":[{{"scope":"comment","settings":{{"foreground":"{color}"}}}}]}}"##
        ))
        .unwrap()
    }
    fn assert_matches_fresh(installation: &TextMateInstallation, document: &TextDocument) {
        let fresh = TextMateInstallation::new(grammar(), theme("#ff0000"));
        assert!(fresh.advance_to(document, document.line_count(), usize::MAX));
        for line in 1..=document.line_count() {
            assert_eq!(
                installation.styled_tokens(line),
                fresh.styled_tokens(line),
                "line {line}"
            );
        }
    }

    #[test]
    fn line_insertions_and_removals_reuse_the_converged_suffix() {
        let document = TextDocument::from_text(&vec!["plain"; 100].join("\n"));
        let installation = TextMateInstallation::new(grammar(), theme("#ff0000"));
        assert!(installation.advance_to(&document, 100, 100));
        document.insert(6, "added\n").unwrap();
        installation.invalidate_from(2);
        assert!(installation.advance_to(&document, 101, 4));
        assert!(installation.last_work_count() <= 4);
        assert_matches_fresh(&installation, &document);
        document.remove(6, 6).unwrap();
        installation.invalidate_from(2);
        assert!(installation.advance_to(&document, 100, 4));
        assert_matches_fresh(&installation, &document);
    }

    #[test]
    fn changed_multiline_state_hides_invalid_tokens_until_completed() {
        let document = TextDocument::from_text("/*\ninside\n*/\nplain\nlast");
        let installation = TextMateInstallation::new(grammar(), theme("#ff0000"));
        assert!(installation.advance_to(&document, 5, 5));
        document.remove(10, 2).unwrap();
        installation.invalidate_from(3);
        assert!(!installation.advance_to(&document, 5, 1));
        assert!(installation.styled_tokens(5).is_empty());
        assert!(installation.advance_to(&document, 5, 10));
        assert_eq!(
            installation.styled_tokens(5)[0].2.foreground,
            Color::parse("#ff0000")
        );
        assert_matches_fresh(&installation, &document);
    }

    #[test]
    fn coalesced_edits_and_crlf_boundary_changes_keep_cache_alignment() {
        let document = TextDocument::from_text("head\r\n/*\r\nbody\r\n*/\r\nlast");
        let installation = TextMateInstallation::new(grammar(), theme("#ff0000"));
        assert!(installation.advance_to(&document, 5, 5));
        document.insert(0, "prefix\n").unwrap();
        document.remove(12, 1).unwrap();
        document.insert(document.text_length(), "\nend").unwrap();
        installation.invalidate_from(1);
        assert!(installation.advance_to(&document, document.line_count(), 100));
        assert_matches_fresh(&installation, &document);
    }

    #[test]
    fn theme_and_grammar_changes_notify_without_retaining_borrows() {
        let document = TextDocument::from_text("/* comment */");
        let installation = Rc::new(TextMateInstallation::new(grammar(), theme("#ff0000")));
        assert!(installation.advance_to(&document, 1, 1));
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let weak = Rc::downgrade(&installation);
        let subscription = installation.changed(move || {
            let installation = weak.upgrade().unwrap();
            let _ = installation.theme();
            let _ = installation.grammar();
            observed.set(observed.get() + 1);
        });
        installation.set_theme(theme("#0000ff"));
        assert!(installation.advance_to(&document, 1, 0));
        assert_eq!(
            installation.styled_tokens(1)[0].2.foreground,
            Color::parse("#0000ff")
        );
        installation.set_grammar(grammar());
        assert!(!installation.advance_to(&document, 1, 0));
        assert_eq!(calls.get(), 2);
        subscription.dispose();
    }

    #[test]
    fn cancellation_and_work_limits_never_publish_partial_lines() {
        let document = TextDocument::from_text("/* comment */");
        let installation = TextMateInstallation::new(grammar(), theme("#ff0000"));
        assert!(!installation.advance_to_cancellable(&document, 1, 1, &|| true));
        assert!(installation.styled_tokens(1).is_empty());
        assert!(installation.last_error().is_none());
        installation.set_tokenize_options(TokenizeOptions {
            max_steps: 0,
            ..Default::default()
        });
        assert!(!installation.advance_to(&document, 1, 1));
        assert!(installation.last_error().is_some());
        installation.set_tokenize_options(TokenizeOptions::default());
        assert!(installation.advance_to(&document, 1, 1));
        assert!(installation.last_error().is_none());
    }

    #[test]
    fn queued_edits_match_fresh_tokenization_across_line_boundary_changes() {
        let document = TextDocument::from_text("plain\r\n/*\ninside\r*/\nlast");
        let installation = TextMateInstallation::new(grammar(), theme("#ff0000"));
        assert!(installation.advance_to(&document, document.line_count(), 100));
        let mut seed = 0x5796_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 32) as usize
        };
        let replacements = ["", "a", "\r", "\n", "\r\n", "/*", "*/", "body"];
        for _ in 0..200 {
            for _ in 0..3 {
                let offset = next() % (document.text_length() + 1);
                let removed = (next() % 4).min(document.text_length() - offset);
                document
                    .replace(offset, removed, replacements[next() % replacements.len()])
                    .unwrap();
                let line = document
                    .get_line_by_offset(offset.min(document.text_length()))
                    .unwrap()
                    .line_number();
                installation.invalidate_from(line);
            }
            let mut complete = false;
            for _ in 0..document.line_count() + 2 {
                if installation.advance_to(&document, document.line_count(), 2) {
                    complete = true;
                    break;
                }
                assert!(installation.last_error().is_none());
            }
            assert!(complete);
            assert_matches_fresh(&installation, &document);
        }
    }
}
