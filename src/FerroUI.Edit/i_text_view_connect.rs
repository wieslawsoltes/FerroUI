use crate::TextArea;

/// Optional view lifetime hooks shared by generators, transformers and renderers.
/// Implementations retaining a view should keep a WeakRef to avoid ownership cycles.
pub trait ITextViewConnect {
    fn add_to_text_view(&self, _text_view: &TextArea) {}
    /// Also called during view destruction. Use `text_view.to_weak()` when
    /// comparing connection identities: `to_ref()` cannot revive a dying view.
    fn remove_from_text_view(&self, _text_view: &TextArea) {}
}
