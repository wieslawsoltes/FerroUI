use crate::{
    model_events::subscribe, Error, Grammar, IModelLines, IModelTokensChangedListener,
    ModelSubscription, ModelTokensChangedEvent, Result, StateStack, Token, TokenizeOptions,
};
use ferroui_system::events::Event;
use std::{ops::Range, rc::Rc};

#[derive(Clone, Debug)]
pub struct LineTokens {
    pub tokens: Vec<Token>,
    pub end_state: StateStack,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenizationUpdate {
    /// Half-open zero-based range whose tokenization was recomputed.
    pub lines: Range<usize>,
    pub pending: bool,
    /// Number of regex transitions/searches is bounded separately per line.
    pub stopped_early: bool,
}

/// Incremental line cache. Edits preserve untouched suffix entries; tokenization
/// stops when the recomputed state reaches the old suffix state.
pub struct TokenizationModel {
    grammar: Option<Grammar>,
    lines: Vec<String>,
    cache: Vec<Option<LineTokens>>,
    invalid_from: Option<usize>,
    invalid_until: usize,
    revision: u64,
    tokens_changed: Rc<Event<ModelTokensChangedEvent>>,
}
impl Default for TokenizationModel {
    /// Creates an empty document with highlighting disabled.
    fn default() -> Self {
        Self {
            grammar: None,
            lines: vec![String::new()],
            cache: vec![None],
            invalid_from: None,
            invalid_until: 0,
            revision: 0,
            tokens_changed: Rc::new(Event::new()),
        }
    }
}

impl TokenizationModel {
    pub fn new(grammar: Grammar) -> Self {
        Self {
            grammar: Some(grammar),
            invalid_from: Some(0),
            invalid_until: 1,
            ..Self::default()
        }
    }
    pub fn from_lines(grammar: Grammar, source: &dyn IModelLines) -> Result<Self> {
        let mut model = Self::new(grammar);
        model.set_lines(source)?;
        Ok(model)
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Retain the returned guard to keep the listener registered.
    pub fn tokens_changed(
        &self,
        handler: impl Fn(&ModelTokensChangedEvent) + 'static,
    ) -> ModelSubscription {
        subscribe(&self.tokens_changed, Rc::new(handler))
    }
    pub fn add_model_tokens_changed_listener(
        &self,
        listener: Rc<dyn IModelTokensChangedListener>,
    ) -> ModelSubscription {
        self.tokens_changed(move |event| listener.model_tokens_changed(event))
    }
    pub fn grammar(&self) -> Option<&Grammar> {
        self.grammar.as_ref()
    }
    pub fn set_grammar(&mut self, grammar: Grammar) {
        if self
            .grammar()
            .is_some_and(|current| current.same_instance(&grammar))
        {
            return;
        }
        self.grammar = Some(grammar);
        self.reset_tokens();
    }
    /// Disables highlighting and publishes a whole-document change so hosts can
    /// clear existing styles immediately. The document lines remain editable.
    pub fn clear_grammar(&mut self) {
        if self.grammar.take().is_some() {
            self.reset_tokens();
            self.publish_tokens_changed(0..self.lines.len());
        }
    }
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
    pub fn line(&self, index: usize) -> Option<&str> {
        self.lines.get(index).map(String::as_str)
    }
    pub fn is_line_invalid(&self, index: usize) -> bool {
        index < self.lines.len() && self.invalid_from.is_some_and(|first| index >= first)
    }
    pub fn pending_line(&self) -> Option<usize> {
        self.invalid_from
    }
    pub fn tokens(&self, index: usize) -> Option<&LineTokens> {
        if self.invalid_from.is_some_and(|first| index >= first) {
            return None;
        }
        self.cache.get(index).and_then(Option::as_ref)
    }
    pub fn set_text(&mut self, text: &str) {
        let mut lines = Vec::new();
        let mut start = 0;
        let bytes = text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if matches!(bytes[index], b'\r' | b'\n') {
                lines.push(text[start..index].into());
                if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
                start = index + 1;
            }
            index += 1;
        }
        lines.push(text[start..].into());
        self.lines = lines;
        self.reset_tokens();
    }
    /// Copies a host snapshot into this model. Subsequent incremental edits use
    /// replace_lines so unchanged token and continuation caches can be reused.
    pub fn set_lines(&mut self, source: &dyn IModelLines) -> Result<()> {
        let mut lines = Vec::with_capacity(source.line_count());
        for index in 0..source.line_count() {
            let line = source.line_text(index).ok_or(Error::InvalidLine(index))?;
            lines.push(line.without_line_ending().to_owned());
        }
        if lines.is_empty() {
            lines.push(String::new());
        }
        self.lines = lines;
        self.reset_tokens();
        Ok(())
    }
    fn reset_tokens(&mut self) {
        self.cache.clear();
        self.cache.resize_with(self.lines.len(), || None);
        self.invalid_from = self.grammar.is_some().then_some(0);
        self.invalid_until = self.invalid_from.map_or(0, |_| self.lines.len());
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn replace_lines(
        &mut self,
        range: Range<usize>,
        replacement: impl IntoIterator<Item = String>,
    ) -> Result<()> {
        if range.start > range.end || range.end > self.lines.len() {
            return Err(Error::InvalidLine(range.end));
        }
        let replacement: Vec<_> = replacement.into_iter().collect();
        let inserted = replacement.len();
        let removed = range.len();
        self.lines.splice(range.clone(), replacement);
        self.cache
            .splice(range.clone(), std::iter::repeat_n(None, inserted));
        if self.lines.is_empty() {
            self.lines.push(String::new());
            self.cache.push(None);
        }
        if self.grammar.is_some() {
            let old_until = if self.invalid_until >= range.end {
                self.invalid_until - removed + inserted
            } else {
                self.invalid_until.min(range.start)
            };
            let first = range.start.min(self.lines.len() - 1);
            self.invalid_from = Some(self.invalid_from.map_or(first, |old| old.min(first)));
            self.invalid_until = old_until.max(range.start + inserted).min(self.lines.len());
        }
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    pub fn invalidate_lines(&mut self, range: Range<usize>) -> Result<()> {
        if range.start > range.end || range.end > self.lines.len() {
            return Err(Error::InvalidLine(range.end));
        }
        if range.is_empty() || self.grammar.is_none() {
            return Ok(());
        }
        self.invalid_from = Some(
            self.invalid_from
                .map_or(range.start, |old| old.min(range.start)),
        );
        self.invalid_until = self.invalid_until.max(range.end);
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
    /// Makes the requested line current, propagating its preceding multiline
    /// state as needed. The regular per-line work budget still applies.
    pub fn force_tokenization(&mut self, index: usize) -> Result<TokenizationUpdate> {
        self.force_tokenization_with_options(index, TokenizeOptions::default(), None)
    }
    pub fn force_tokenization_with_options(
        &mut self,
        index: usize,
        options: TokenizeOptions,
        cancelled: Option<&dyn Fn() -> bool>,
    ) -> Result<TokenizationUpdate> {
        if index >= self.lines.len() {
            return Err(Error::InvalidLine(index));
        }
        let max_lines = self
            .invalid_from
            .filter(|first| *first <= index)
            .map_or(0, |first| index - first + 1);
        self.retokenize_with_options(max_lines, options, cancelled)
    }
    pub fn retokenize(&mut self, max_lines: usize) -> Result<TokenizationUpdate> {
        self.retokenize_with_options(max_lines, TokenizeOptions::default(), None)
    }
    pub fn retokenize_with_options(
        &mut self,
        max_lines: usize,
        options: TokenizeOptions,
        cancelled: Option<&dyn Fn() -> bool>,
    ) -> Result<TokenizationUpdate> {
        let start = self.invalid_from.unwrap_or(0);
        let mut completed = start..start;
        let result = self.retokenize_inner(max_lines, options, cancelled, &mut completed);
        if !completed.is_empty() {
            self.publish_tokens_changed(completed);
        }
        result
    }
    fn publish_tokens_changed(&self, lines: Range<usize>) {
        self.tokens_changed.raise(&ModelTokensChangedEvent {
            ranges: vec![lines],
            revision: self.revision,
            line_count: self.lines.len(),
            pending: self.invalid_from.is_some(),
        });
    }
    fn retokenize_inner(
        &mut self,
        max_lines: usize,
        options: TokenizeOptions,
        cancelled: Option<&dyn Fn() -> bool>,
        completed: &mut Range<usize>,
    ) -> Result<TokenizationUpdate> {
        let (Some(start), Some(grammar)) = (self.invalid_from, self.grammar.as_ref()) else {
            return Ok(TokenizationUpdate {
                lines: 0..0,
                pending: false,
                stopped_early: false,
            });
        };
        let mut index = start;
        let mut previous = if start == 0 {
            grammar.initial_state()
        } else {
            self.cache[start - 1]
                .as_ref()
                .ok_or(Error::InvalidState)?
                .end_state
                .clone()
        };
        let mut stopped_early = false;
        while index < self.lines.len() && index - start < max_lines {
            let result = grammar.tokenize_line_with_options(
                &self.lines[index],
                Some(&previous),
                options,
                cancelled,
            )?;
            if result.stopped_early {
                stopped_early = true;
                break;
            }
            let converged = index >= self.invalid_until
                && self.cache[index].as_ref().is_some_and(|old| {
                    old.end_state == result.rule_stack && old.tokens == result.tokens
                });
            previous = result.rule_stack.clone();
            self.cache[index] = Some(LineTokens {
                tokens: result.tokens,
                end_state: result.rule_stack,
            });
            index += 1;
            completed.end = index;
            self.invalid_from = (index < self.lines.len()).then_some(index);
            if converged {
                // A previously cached suffix can still end in lines that have
                // never been tokenized. Convergence must not mark those valid.
                self.invalid_from = self.cache[index..]
                    .iter()
                    .position(Option::is_none)
                    .map(|offset| index + offset);
                self.invalid_until = self.invalid_from.map_or(0, |first| first + 1);
                return Ok(TokenizationUpdate {
                    lines: start..index,
                    pending: self.invalid_from.is_some(),
                    stopped_early: false,
                });
            }
        }
        self.invalid_from = (index < self.lines.len()).then_some(index);
        if self.invalid_from.is_none() {
            self.invalid_until = 0;
        }
        Ok(TokenizationUpdate {
            lines: start..index,
            pending: self.invalid_from.is_some(),
            stopped_early,
        })
    }
}
