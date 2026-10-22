use crate::TextArea;

/// Optional view lifetime hooks shared by generators and line transformers.
/// Implementations retaining a view should keep a WeakRef to avoid ownership cycles.
pub trait ITextViewConnect {
    fn add_to_text_view(&self, _text_view: &TextArea) {}
    fn remove_from_text_view(&self, _text_view: &TextArea) {}
}
