use crate::{
    Grammar, IModelTokensChangedListener, LineText, LineTokens, Result, TokenizationModel,
    TokenizationUpdate,
};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Borrowed line access for an editor document or a snapshot. Implementations
/// can include a final LF, CRLF or CR in line_text; line_length excludes it.
/// Mutable cache bookkeeping stays in TokenizationModel and is changed with
/// replace_lines/invalidate_lines rather than exposed ModelLine objects.
pub trait IModelLines {
    fn line_count(&self) -> usize;
    fn line_text(&self, index: usize) -> Option<LineText<'_>>;
    fn line_length(&self, index: usize) -> Option<usize> {
        Some(
            self.line_text(index)?
                .without_line_ending()
                .encode_utf16()
                .count(),
        )
    }
}
impl IModelLines for TokenizationModel {
    fn line_count(&self) -> usize {
        self.line_count()
    }
    fn line_text(&self, index: usize) -> Option<LineText<'_>> {
        self.line(index).map(LineText::new)
    }
}
impl<S: AsRef<str>> IModelLines for Vec<S> {
    fn line_count(&self) -> usize {
        self.len()
    }
    fn line_text(&self, index: usize) -> Option<LineText<'_>> {
        self.get(index).map(|line| LineText::new(line.as_ref()))
    }
}

/// Host-facing incremental model contract. Work is scheduled by the caller;
/// Rust ownership releases the model, and event registrations are removed
/// through the returned FerroUI IDisposable subscription.
pub trait ITMModel: IModelLines {
    fn grammar(&self) -> &Grammar;
    fn set_grammar(&mut self, grammar: Grammar);
    fn tokens(&self, index: usize) -> Option<&LineTokens>;
    fn force_tokenization(&mut self, index: usize) -> Result<TokenizationUpdate>;
    fn add_model_tokens_changed_listener(
        &self,
        listener: Rc<dyn IModelTokensChangedListener>,
    ) -> Rc<dyn IDisposable>;
}
impl ITMModel for TokenizationModel {
    fn grammar(&self) -> &Grammar {
        self.grammar()
    }
    fn set_grammar(&mut self, grammar: Grammar) {
        self.set_grammar(grammar);
    }
    fn tokens(&self, index: usize) -> Option<&LineTokens> {
        self.tokens(index)
    }
    fn force_tokenization(&mut self, index: usize) -> Result<TokenizationUpdate> {
        self.force_tokenization(index)
    }
    fn add_model_tokens_changed_listener(
        &self,
        listener: Rc<dyn IModelTokensChangedListener>,
    ) -> Rc<dyn IDisposable> {
        self.add_model_tokens_changed_listener(listener)
    }
}
