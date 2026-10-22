mod document_highlighter;
mod highlighted_line;
mod highlighter;
mod highlighting_definition;
mod highlighting_engine;
mod highlighting_manager;
mod html_clipboard;
mod rich_text;
mod rich_text_writer;
pub mod xshd;

pub use document_highlighter::DocumentHighlighter;
pub use highlighted_line::{HighlightedLine, HighlightedSection};
pub use highlighter::IHighlighter as Highlighter;
pub use highlighter::{HighlightingStateChangedEventArgs, IHighlighter};
pub use highlighting_definition::{
    HighlightingColor, HighlightingDefinition, HighlightingError, HighlightingRule,
    HighlightingRuleSet, HighlightingSpan,
};
pub use highlighting_engine::{HighlightingEngine, HighlightingState};
pub use highlighting_manager::HighlightingManager;
pub use html_clipboard::{HtmlClipboard, HtmlOptions, HtmlRichTextWriter};
pub use rich_text::{RichText, RichTextError, RichTextModel};
pub use rich_text_writer::{PlainRichTextWriter, RichTextModelWriter, RichTextWriter};
