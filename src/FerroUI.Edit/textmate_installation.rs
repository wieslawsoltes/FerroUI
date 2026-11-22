use crate::bracket_balance::{BracketLine, UnmatchedClosers};
use crate::input_languages::{InputLanguages, InputToken, LanguageScope};
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_edit_core::document::{DocumentChangeEventArgs, TextDocument, TextSnapshot};
use ferroui_textmate::{Grammar, GrammarConfiguration, LanguageConfiguration, StandardTokenType, StateStack, Theme, Token, TokenStyle, TokenStyleOverrides, TokenizeOptions};
use crate::language_pairs::GeneratedPair;
use crate::package_snippets::{PackageSnippets, SnippetVariableProvider};
use crate::visual_line::VISUAL_LINE_LENGTH_LIMIT;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

#[derive(Clone)]
struct CachedLine {
    utf16_length: usize,
    state: StateStack,
    tokens: Vec<Token>,
    brackets: BracketLine,
}

enum ScanFailure {
    Retryable(String),
    Line { index: usize, message: String },
}

#[derive(Default)]
struct TokenCache {
    snapshot: Option<TextSnapshot>,
    lines: Vec<Option<CachedLine>>,
    invalid_from: Option<usize>,
    dirty_until: usize,
    unmatched_closers: UnmatchedClosers,
}

impl TokenCache {
    fn reset_lines(&mut self) {
        self.lines.fill(None);
        self.invalid_from = (!self.lines.is_empty()).then_some(0);
        self.dirty_until = self.lines.len();
        self.unmatched_closers = UnmatchedClosers::default();
    }
}

/// A grammar and theme attached to an editor. Persistent document snapshots
/// align retained line states across edits; tokenization stops when an unchanged
/// suffix has the same tokens and continuation state. Text is retrieved lazily
/// for the lines actually processed, without copying the document to a string.
pub struct TextMateInstallation {
    grammar: RefCell<Grammar>,
    theme: RefCell<Theme>,
    input_languages: RefCell<Rc<InputLanguages>>,
    pub(crate) package_snippets: RefCell<Rc<PackageSnippets>>,
    pub(crate) snippet_variables: RefCell<Option<Rc<SnippetVariableProvider>>>,
    pub(crate) resolving_snippet_variables: Cell<bool>,
    pub(crate) generated_pairs: RefCell<Vec<GeneratedPair>>,
    cache: RefCell<TokenCache>,
    error: RefCell<Option<ScanFailure>>,
    options: Cell<TokenizeOptions>,
    max_tokenized_line_length: Cell<usize>,
    generation: Cell<u64>,
    last_work_count: Cell<usize>,
    changed: Rc<HandlerList<dyn Fn()>>,
}

impl TextMateInstallation {
    pub fn new(grammar: Grammar, theme: Theme) -> Self {
        Self {
            grammar: RefCell::new(grammar),
            theme: RefCell::new(theme),
            input_languages: RefCell::new(Rc::new(InputLanguages::default())),
            package_snippets: RefCell::new(Rc::new(PackageSnippets::default())),
            snippet_variables: RefCell::new(None),
            resolving_snippet_variables: Cell::new(false),
            generated_pairs: RefCell::new(Vec::new()),
            cache: RefCell::new(TokenCache::default()),
            error: RefCell::new(None),
            options: Cell::new(TokenizeOptions::default()),
            max_tokenized_line_length: Cell::new(VISUAL_LINE_LENGTH_LIMIT),
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
        self.error.borrow().as_ref().map(|failure| match failure {
            ScanFailure::Retryable(message) | ScanFailure::Line { message, .. } => message.clone(),
        })
    }
    pub fn tokenize_options(&self) -> TokenizeOptions {
        self.options.get()
    }

    /// Maximum physical line length in UTF-16 code units for editor TextMate
    /// scans. Defaults to the visual-line cutoff for optional generators.
    pub fn max_tokenized_line_length(&self) -> usize {
        self.max_tokenized_line_length.get()
    }

    /// Number of lines scanned by the most recent advance. Reused suffix lines
    /// do not consume the caller's line budget.
    pub fn last_work_count(&self) -> usize {
        self.last_work_count.get()
    }

    /// Theme and grammar changes notify the host after all internal borrows
    /// have ended, so a subscriber may invalidate or inspect the editor.
    pub fn changed(&self, handler: impl Fn() + 'static) -> Subscription {
        let handlers = self.changed.clone();
        let token = handlers.add(Rc::new(handler));
        Disposable::create(move || {
            handlers.remove(token);
        })
        .into()
    }

    pub fn set_theme(&self, theme: Theme) {
        *self.theme.borrow_mut() = theme;
        self.raise_changed();
    }

    /// Sets the editing rules supplied by the installed language. Passing `None`
    /// restores the editor's own indentation strategy for every line break.
    pub fn set_language_configuration(
        &self,
        configuration: Option<&LanguageConfiguration>,
    ) -> ferroui_textmate::Result<()> {
        self.set_language_configurations(GrammarConfiguration::default(), configuration, [])
    }

    /// Installs the document language and editing rules for the supplied
    /// embedded language IDs. The IDs must use the grammar configuration's
    /// scope mapping. A mapped language can opt into ordinary typing by
    /// omitting its editing configuration.
    pub fn set_language_configurations<'a>(
        &self,
        grammar: GrammarConfiguration,
        root: Option<&'a LanguageConfiguration>,
        embedded: impl IntoIterator<Item = (u8, &'a LanguageConfiguration)>,
    ) -> ferroui_textmate::Result<()> {
        let input = InputLanguages::new(grammar, root, embedded)?;
        let has_brackets = input.brackets.is_some();
        let previous = self.input_languages.replace(Rc::new(input));
        if previous.grammar.initial_language != self.input_languages().grammar.initial_language
            || previous.grammar.embedded_languages != self.input_languages().grammar.embedded_languages
        {
            self.package_snippets.replace(Rc::new(PackageSnippets::default()));
        }
        self.generated_pairs.borrow_mut().clear();
        if has_brackets || previous.brackets.is_some() {
            self.cache.borrow_mut().reset_lines();
            self.bump_generation();
            self.error.borrow_mut().take();
        }
        self.raise_changed();
        Ok(())
    }

    pub(crate) fn input_languages(&self) -> Rc<InputLanguages> {
        self.input_languages.borrow().clone()
    }

    pub(crate) fn input_tokens(&self, document: &TextDocument, line: usize, input: &InputLanguages) -> Option<Vec<InputToken>> {
        if !self.cached_through(document, line) {
            return None;
        }
        let cache = self.cache.borrow();
        Some(input.tokens(&cache.lines.get(line.checked_sub(1)?)?.as_ref()?.tokens))
    }

    pub(crate) fn input_scope(&self, document: &TextDocument, input: &InputLanguages, line: usize, column: usize, length: usize) -> Option<LanguageScope> {
        let root = input.grammar.initial_language;
        if input.grammar.embedded_languages.is_empty() {
            return Some(LanguageScope { id: root, start: 0, end: length });
        }
        Some(LanguageScope::at(&self.input_tokens(document, line, input)?, root, column, length))
    }

    pub(crate) fn background_target(&self, document: &TextDocument, visible: usize) -> usize {
        if self.input_languages.borrow().brackets.is_some() {
            document.line_count()
        } else {
            visible.min(document.line_count())
        }
    }

    pub(crate) fn cached_through(&self, document: &TextDocument, target: usize) -> bool {
        let cache = self.cache.borrow();
        cache.snapshot.as_ref().and_then(TextSnapshot::version).is_some_and(|version| version == document.version())
            && cache.invalid_from.is_none_or(|first| first >= target.min(document.line_count()))
    }

    pub(crate) fn has_unmatched_closer_at_carets(&self, document: &TextDocument, opener: &str, carets: impl IntoIterator<Item = (u8, usize)>) -> bool {
        let input = self.input_languages();
        let Some(syntax) = &input.brackets else {
            return false;
        };
        let queries: Vec<_> = carets.into_iter().filter_map(|(language, offset)| Some((syntax.opener(language, opener)?, offset))).collect();
        // The answer is global. An incomplete prefix can establish neither
        // that a later closer is unmatched nor that all closers are paired.
        if queries.is_empty() || !self.advance_to(document, document.line_count(), 128) {
            return false;
        }
        let mut cache = self.cache.borrow_mut();
        let TokenCache { lines, unmatched_closers, snapshot, .. } = &mut *cache;
        queries.into_iter().any(|(id, offset)| {
            let last = unmatched_closers.latest(id, lines.iter().enumerate().filter_map(|(index, line)| {
                Some((index, &line.as_ref()?.brackets))
            }));
            last.is_some_and(|(index, column)| {
                snapshot.as_ref().and_then(|snapshot| snapshot.get_line_by_number(index + 1).ok())
                    .is_some_and(|line| line.offset() + column >= offset)
            })
        })
    }

    #[cfg(test)]
    fn has_unmatched_closing_bracket_after(&self, document: &TextDocument, opener: &str, offset: usize) -> bool {
        self.has_unmatched_closer_at_carets(document, opener, [(self.input_languages().grammar.initial_language, offset)])
    }

    pub fn set_grammar(&self, grammar: Grammar) {
        *self.grammar.borrow_mut() = grammar;
        self.input_languages.replace(Rc::new(InputLanguages::default()));
        self.package_snippets.replace(Rc::new(PackageSnippets::default()));
        self.generated_pairs.borrow_mut().clear();
        self.cache.borrow_mut().reset_lines();
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

    /// Changes the limit for highlighting and grammar-aware editing. An
    /// oversized line and its suffix have no verified continuation state.
    /// The verified prefix stays available. Highlighting can resume after
    /// the oversized line is shortened or the limit is raised.
    pub fn set_max_tokenized_line_length(&self, length: usize) {
        let previous = self.max_tokenized_line_length.replace(length);
        if previous == length {
            return;
        }
        if length < previous {
            let mut cache = self.cache.borrow_mut();
            let prefix_end = cache.invalid_from.unwrap_or(cache.lines.len());
            if let Some(first) = cache.lines[..prefix_end]
                .iter()
                .position(|line| line.as_ref().is_some_and(|line| line.utf16_length > length))
            {
                cache.invalid_from = Some(first);
            }
        }
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
        self.clear_error_from(first);
    }

    /// Processes at most line_budget lines and returns whether all states
    /// through the requested one-based line are valid. An oversized line stops
    /// the scan before regex matching. Eligible lines use tokenize_options. A
    /// line that reaches a work limit is retried after invalidation at or
    /// before that line, or when the tokenizer changes.
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
            *self.error.borrow_mut() = Some(ScanFailure::Retryable(error));
            return false;
        }
        let target = line.min(document.line_count());
        let grammar = self.grammar.borrow().clone();
        let input = self.input_languages();
        let options = self.options.get();
        for _ in 0..line_budget {
            let (index, previous, open_brackets, snapshot) = {
                let cache = self.cache.borrow();
                let Some(index) = cache.invalid_from.filter(|index| *index < target) else {
                    return true;
                };
                if matches!(self.error.borrow().as_ref(), Some(ScanFailure::Line { index: failed, .. }) if *failed == index) {
                    return false;
                }
                let previous = index.checked_sub(1).and_then(|index| cache.lines[index].as_ref());
                if index != 0 && previous.is_none() {
                    *self.error.borrow_mut() = Some(ScanFailure::Retryable(
                        "Missing predecessor tokenization state".into(),
                    ));
                    return false;
                }
                (
                    index,
                    previous.map(|line| line.state.clone()),
                    previous.map_or_else(Default::default, |line| line.brackets.stack.clone()),
                    cache
                        .snapshot
                        .as_ref()
                        .expect("synchronized snapshot")
                        .clone(),
                )
            };
            let line = match snapshot.get_line_by_number(index + 1) {
                Ok(line) => line,
                Err(error) => {
                    *self.error.borrow_mut() = Some(ScanFailure::Retryable(error.to_string()));
                    return false;
                }
            };
            let limit = self.max_tokenized_line_length();
            if line.length() > limit {
                *self.error.borrow_mut() = Some(ScanFailure::Line {
                    index,
                    message: format!(
                        "Line {} exceeds the TextMate line limit of {limit} UTF-16 code units",
                        index + 1,
                    ),
                });
                return false;
            }
            let text = match snapshot.get_text(line.offset(), line.length()) {
                Ok(text) => text,
                Err(error) => {
                    *self.error.borrow_mut() = Some(ScanFailure::Retryable(error.to_string()));
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
                    let brackets = if let Some(syntax) = &input.brackets {
                        let brackets = syntax.scan_line(&text, &input.tokens(&result.tokens), open_brackets);
                        let TokenCache { lines, unmatched_closers, .. } = &mut *cache;
                        unmatched_closers.replace_line(index, lines[index].as_ref().map(|line| &line.brackets), &brackets);
                        brackets
                    } else {
                        BracketLine::default()
                    };
                    let converged = index >= cache.dirty_until
                        && cache.lines[index].as_ref().is_some_and(|old| {
                            old.state == result.rule_stack && old.tokens == result.tokens
                                && old.brackets.stack == brackets.stack
                        });
                    cache.lines[index] = Some(CachedLine {
                        utf16_length: line.length(),
                        state: result.rule_stack,
                        tokens: result.tokens,
                        brackets,
                    });
                    cache.invalid_from = if converged {
                        cache.dirty_until = 0;
                        cache.lines[index + 1..]
                            .iter()
                            .position(|line| line.as_ref().is_none_or(|line| line.utf16_length > limit))
                            .map(|relative| index + 1 + relative)
                    } else {
                        // The cached lines after this one came from another state: a later
                        // pass can only converge past them.
                        cache.dirty_until = cache.dirty_until.max(index + 1);
                        (index + 1 < cache.lines.len()).then_some(index + 1)
                    };
                    if cache.invalid_from.is_none() {
                        cache.dirty_until = 0;
                    }
                    self.error.borrow_mut().take();
                }
                Ok(_) => {
                    if !was_cancelled.get() {
                        *self.error.borrow_mut() = Some(ScanFailure::Line {
                            index,
                            message: format!("Grammar work limit reached on line {}", index + 1),
                        });
                    }
                    return false;
                }
                Err(error) => {
                    let message = error.to_string();
                    *self.error.borrow_mut() = Some(match error {
                        ferroui_textmate::Error::RegexLimit(_) => ScanFailure::Line { index, message },
                        _ => ScanFailure::Retryable(message),
                    });
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
        self.token_styles(line, Theme::match_scopes)
    }

    pub(crate) fn token_style_overrides(&self, line: usize) -> Vec<(usize, usize, TokenStyleOverrides)> {
        let mut styles = self.token_styles(line, Theme::style_overrides);
        styles.retain(|(_, _, style)| *style != TokenStyleOverrides::default());
        styles
    }

    fn token_styles<S>(
        &self,
        line: usize,
        style: impl Fn(&Theme, &[String]) -> S,
    ) -> Vec<(usize, usize, S)> {
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
                    style(&theme, &token.scopes),
                )
            })
            .collect()
    }

    /// Classifies a prospective character with the grammar state entering its
    /// physical line. Typing remains bounded even far beyond the cached view.
    pub(crate) fn prospective_token_type(
        &self,
        document: &TextDocument,
        line: usize,
        text: &str,
        column: usize,
    ) -> Option<StandardTokenType> {
        let limit = self.max_tokenized_line_length();
        if text.len() > limit && text.encode_utf16().nth(limit).is_some() {
            return None;
        }
        let previous = self.state_before_input_line(document, line)?;
        let result = self.grammar.borrow().tokenize_line_with_options(
            text, previous.as_ref(), self.tokenize_options(), None,
        ).ok()?;
        if result.stopped_early {
            return None;
        }
        let input = self.input_languages();
        Some(result.tokens.iter().find(|token| token.start_index <= column && column < token.end_index)
            .map_or(StandardTokenType::Other, |token| input.grammar.token_type_for_scopes(&token.scopes)))
    }

    /// A successful result on the first line has no predecessor state.
    pub(crate) fn state_before_input_line(&self, document: &TextDocument, line: usize) -> Option<Option<StateStack>> {
        if line <= 1 {
            return Some(None);
        }
        if !self.advance_to(document, line - 1, 128) {
            return None;
        }
        Some(Some(self.cache.borrow().lines.get(line - 2)?.as_ref()?.state.clone()))
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
                    self.clear_error_from(first);
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
                    cache.unmatched_closers.splice(first, old_last, new_last);
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
        cache.unmatched_closers = UnmatchedClosers::default();
        cache.invalid_from = Some(0);
        cache.dirty_until = 0;
        cache.snapshot = Some(snapshot);
        self.bump_generation();
        self.error.borrow_mut().take();
        Ok(())
    }

    fn bump_generation(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
    }

    fn clear_error_from(&self, first: usize) {
        let mut error = self.error.borrow_mut();
        if !matches!(error.as_ref(), Some(ScanFailure::Line { index, .. }) if *index < first) {
            error.take();
        }
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

    #[test]
    fn bracket_index_repairs_state_after_a_distant_opener_and_reuses_aligned_lines() {
        let installation = TextMateInstallation::new(grammar(), theme("#005500"));
        let configuration = LanguageConfiguration::from_json(r#"{"brackets":[["(",")"],["[","]"]]}"#).unwrap();
        installation.set_language_configuration(Some(&configuration)).unwrap();
        let prefix = format!("(\n{}", "text\n".repeat(400));
        let document = TextDocument::from_text(&format!("{prefix})\n"));
        assert!(installation.advance_to(&document, document.line_count(), 500));
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "(", 0));

        document.remove(0, 1).unwrap();
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "(", 0));
        assert_eq!(installation.last_work_count(), 128);
        assert!(installation.advance_to(&document, document.line_count(), 500));
        assert!(installation.has_unmatched_closing_bracket_after(&document, "(", 0));
        let end = document.text_length();
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "(", end));

        document.insert(1, "another line\n").unwrap();
        assert!(installation.has_unmatched_closing_bracket_after(&document, "(", 0));
        assert!(installation.last_work_count() < 10);
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "(", document.text_length()));

        let closer = document.text_length() - 2;
        document.replace(closer, 1, "]").unwrap();
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "(", 0));
        assert!(installation.has_unmatched_closing_bracket_after(&document, "[", closer));
    }

    #[test]
    fn a_partial_pass_does_not_converge_before_the_lines_it_has_not_reached() {
        let installation = TextMateInstallation::new(grammar(), theme("#005500"));
        let document = TextDocument::from_text("a\n1\n2\n3\n4");
        assert!(installation.advance_to(&document, document.line_count(), 10));

        // The pass opened by the comment stops after two lines.
        document.insert(0, "/*").unwrap();
        installation.invalidate_from(1);
        installation.advance_to(&document, document.line_count(), 2);
        // An edit above repeats the same state on line two: the lines after it are still old.
        document.insert(2, "x").unwrap();
        installation.invalidate_from(1);
        assert!(installation.advance_to(&document, document.line_count(), 10));
        let fresh = TextMateInstallation::new(grammar(), theme("#005500"));
        assert!(fresh.advance_to(&document, document.line_count(), 10));
        for line in 0..document.line_count() {
            assert_eq!(installation.styled_tokens(line), fresh.styled_tokens(line), "line {line}");
        }
    }

    #[test]
    fn word_delimiters_and_shared_closers_follow_the_installed_language() {
        let installation = TextMateInstallation::new(grammar(), theme("#005500"));
        let configuration = LanguageConfiguration::from_json(r#"{
            "brackets":[["begin","end"],["case","end"],["{","}"],["${","}"]],
            "comments":{"blockComment":["/*","*/"]}
        }"#).unwrap();
        installation.set_language_configuration(Some(&configuration)).unwrap();
        let document = TextDocument::from_text("BEGINNER EnD\nCaSe\nBEGIN\nend\nEND\n/* } */\n${}");
        assert!(installation.has_unmatched_closing_bracket_after(&document, "Begin", 0));
        assert!(installation.has_unmatched_closing_bracket_after(&document, "case", 9));
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "begin", 13));
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "{", 0));
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "${", 0));
        installation.set_language_configuration(None).unwrap();
        assert!(!installation.has_unmatched_closing_bracket_after(&document, "begin", 0));
    }

    #[test]
    fn cached_unmatched_closers_agree_with_a_fresh_scan_after_edits() {
        let configuration = LanguageConfiguration::from_json(r#"{"brackets":[["(",")"],["[","]"]]}"#).unwrap();
        let installation = TextMateInstallation::new(grammar(), theme("#005500"));
        installation.set_language_configuration(Some(&configuration)).unwrap();
        let document = TextDocument::from_text("(\r\n\n[\r\n)\n]\n/*\n)\n*/\n)\r\n");
        let inserts = ["(", ")", "[", "]", "/*", "*/", "\r\n", "\r", "\n", "w", ""];
        let mut seed = 989_u64;
        for index in 0..180 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let length = document.text_length();
            let offset = (seed >> 32) as usize % (length + 1);
            let remove = (index % 3).min(length - offset);
            document.replace(offset, remove, inserts[index % inserts.len()]).unwrap();
            let fresh = TextMateInstallation::new(grammar(), theme("#005500"));
            fresh.set_language_configuration(Some(&configuration)).unwrap();
            assert!(installation.advance_to(&document, document.line_count(), 1_000));
            assert!(fresh.advance_to(&document, document.line_count(), 1_000));
            let length = document.text_length();
            for caret in [0, offset.min(length), length / 2, length] {
                for opener in ["(", "["] {
                    assert_eq!(
                        installation.has_unmatched_closing_bracket_after(&document, opener, caret),
                        fresh.has_unmatched_closing_bracket_after(&document, opener, caret),
                        "edit {index}, opener {opener:?}, caret {caret}"
                    );
                }
            }
        }
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
    fn work_limited_line_is_retried_only_when_the_line_or_its_context_can_change() {
        let digits = "1 ".repeat(100);
        let document = TextDocument::from_text(&format!("42\n{digits}\ninside\n*/"));
        let installation = TextMateInstallation::new(grammar(), theme("#ff0000"));
        installation.set_tokenize_options(TokenizeOptions {
            max_steps: 30,
            ..Default::default()
        });

        assert!(!installation.advance_to(&document, 4, 4));
        assert_eq!(installation.last_work_count(), 2);
        let error = installation.last_error().unwrap();
        assert!(error.contains("line 2"));
        assert!(!installation.styled_tokens(1).is_empty());
        assert!(installation.styled_tokens(2).is_empty());
        assert!(installation.styled_tokens(3).is_empty());

        assert!(installation.advance_to(&document, 1, 4));
        assert!(!installation.advance_to(&document, 4, 4));
        assert_eq!(installation.last_work_count(), 0);
        let suffix = document.get_line_by_number(4).unwrap();
        document.insert(suffix.offset() + 2, " end").unwrap();
        installation.invalidate_from(4);
        assert!(!installation.advance_to(&document, 4, 4));
        assert_eq!(installation.last_work_count(), 0);
        assert_eq!(installation.last_error().as_deref(), Some(error.as_str()));

        installation.invalidate_from(2);
        assert!(!installation.advance_to(&document, 4, 4));
        assert_eq!(installation.last_work_count(), 1);

        // Changing the incoming grammar state can make the same line cheap.
        document.replace(0, 2, "/*").unwrap();
        assert!(installation.advance_to(&document, 4, 4));
        assert!(installation.last_error().is_none());
        assert_eq!(
            installation.styled_tokens(2)[0].2.foreground,
            Color::parse("#ff0000")
        );

        document.replace(0, 2, "42").unwrap();
        assert!(!installation.advance_to(&document, 4, 4));
        document.replace(3, digits.len(), "1").unwrap();
        assert!(installation.advance_to(&document, 4, 4));
        assert!(installation.last_error().is_none());
        assert_matches_fresh(&installation, &document);
    }

    #[test]
    fn oversized_line_keeps_the_prefix_and_recovers_after_shortening() {
        let installation = TextMateInstallation::new(grammar(), theme("#ff0000"));
        let limit = installation.max_tokenized_line_length();
        let at_limit = format!("{}/*", "😀".repeat((limit - 2) / 2));
        let document = TextDocument::from_text(&format!("42\n{at_limit}a\ncomment\n*/"));

        assert!(!installation.advance_to(&document, 4, 4));
        assert_eq!(installation.last_work_count(), 1);
        assert!(installation.last_error().unwrap().contains("Line 2"));
        assert!(!installation.styled_tokens(1).is_empty());
        assert!(installation.styled_tokens(2).is_empty());
        assert!(installation.styled_tokens(3).is_empty());
        assert!(!installation.advance_to(&document, 4, 4));
        assert_eq!(installation.last_work_count(), 0);
        assert_eq!(
            installation.prospective_token_type(&document, 2, &at_limit, 0),
            Some(StandardTokenType::Other)
        );
        assert_eq!(
            installation.prospective_token_type(&document, 2, &format!("{at_limit}a"), 0),
            None
        );

        installation.set_max_tokenized_line_length(limit + 1);
        assert!(installation.advance_to(&document, 4, 4));
        assert_eq!(
            installation.styled_tokens(3)[0].2.foreground,
            Color::parse("#ff0000")
        );
        installation.set_max_tokenized_line_length(limit);
        assert!(!installation.advance_to(&document, 4, 4));
        assert!(installation.styled_tokens(3).is_empty());

        document.remove(3 + limit, 1).unwrap();
        assert!(installation.advance_to(&document, 4, 4));
        assert!(installation.last_error().is_none());
        assert_matches_fresh(&installation, &document);
    }

    #[test]
    fn changing_the_line_limit_reuses_verified_states_and_stops_at_the_next_oversized_line() {
        let prefix_lines = 128;
        let prefix = "42\n".repeat(prefix_lines);
        let first = format!("{}/*", "a".repeat(24));
        let second = "b".repeat(40);
        let document = TextDocument::from_text(&format!("{prefix}{first}\ncomment\n*/\n{second}\n42"));
        let installation = TextMateInstallation::new(grammar(), theme("#ff0000"));
        let end = document.line_count();
        let first_long = prefix_lines + 1;
        let second_long = first_long + 3;
        assert!(installation.advance_to(&document, end, end));

        installation.set_max_tokenized_line_length(16);
        assert!(installation.advance_to(&document, prefix_lines, 0));
        assert!(!installation.styled_tokens(prefix_lines).is_empty());
        assert!(!installation.advance_to(&document, end, 2));
        assert_eq!(installation.last_work_count(), 0);
        assert!(installation.last_error().unwrap().contains(&format!("Line {first_long} ")));

        installation.set_max_tokenized_line_length(first.len());
        assert!(!installation.advance_to(&document, end, 2));
        assert_eq!(installation.last_work_count(), 1);
        assert!(installation.last_error().unwrap().contains(&format!("Line {second_long} ")));
        assert_eq!(
            installation.styled_tokens(first_long + 1)[0].2.foreground,
            Color::parse("#ff0000"),
        );
        assert!(installation.styled_tokens(second_long).is_empty());
        assert!(installation.styled_tokens(end).is_empty());

        installation.set_max_tokenized_line_length(second.len());
        assert!(installation.advance_to(&document, end, 1));
        assert_eq!(installation.last_work_count(), 1);
        assert!(installation.last_error().is_none());
        assert_matches_fresh(&installation, &document);
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
