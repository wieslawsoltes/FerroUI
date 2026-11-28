use super::{HighlightedLine, HighlightingColor, HighlightingError};
use crate::document::{DocumentSubscription, TextDocument};
use std::rc::Rc;

/// Inclusive one-based line range whose input highlighting state changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighlightingStateChangedEventArgs {
    pub from_line_number: usize,
    pub to_line_number: usize,
}

/// A highlighted document that can be consumed without knowing its parser.
///
/// Event handlers may inspect already computed states. Starting another
/// highlighting operation from a handler is rejected. A handler must not edit
/// the document; implementations return an interruption error if it does.
pub trait IHighlighter {
    fn document(&self) -> &TextDocument;
    fn get_color_stack(
        &self,
        line_number: usize,
    ) -> Result<Vec<HighlightingColor>, HighlightingError>;
    fn highlight_line(&self, line_number: usize) -> Result<HighlightedLine, HighlightingError>;
    fn update_highlighting_state(&self, line_number: usize) -> Result<(), HighlightingError>;
    fn highlighting_state_changed(
        &self,
        handler: Rc<dyn Fn(&HighlightingStateChangedEventArgs)>,
    ) -> DocumentSubscription;
    fn begin_highlighting(&self) -> Result<(), HighlightingError>;
    fn end_highlighting(&self) -> Result<(), HighlightingError>;
    fn get_named_color(&self, name: &str) -> Option<HighlightingColor>;
    fn default_text_color(&self) -> Option<HighlightingColor>;
    fn dispose(&self);
}
