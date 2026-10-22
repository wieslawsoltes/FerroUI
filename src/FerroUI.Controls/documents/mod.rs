//! Structured block and inline content, document ranges and editing snapshots.

mod block;
mod block_collection;
mod bold;
mod character_format;
mod document_link_field;
mod document_snapshot;
mod hyperlink;
mod i_inline_host;
mod inline;
mod inline_collection;
mod inline_run;
mod inline_ui_container;
mod italic;
mod line_break;
mod line_spacing_rule;
mod paragraph;
mod rich_text_document;
mod run;
mod span;
mod text_element;
mod text_highlight_merge;
mod text_highlight_range;
mod text_highlighter;
mod text_highlighter_collection;
pub(crate) mod text_language;
mod text_position;
mod typography;
mod typography_format;
mod underline;

pub use block::{Block, BlockImpl, BlockImplExt, BlockVTable, LineStackingStrategy};
pub use block_collection::BlockCollection;
pub use bold::Bold;
pub use character_format::{CharacterFormat, ParagraphFormat};
pub use document_link_field::DocumentLinkField;
pub use document_snapshot::{
    DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, ParagraphSnapshot,
};
pub use ferroui_base::media::text_formatting::{
    TabAlignment, TabLeader, TextTabStop, TextTabStops,
};
pub use hyperlink::{Hyperlink, HyperlinkBehavior, HyperlinkClickEventArgs, UnderlineStyle};
pub use i_inline_host::IInlineHost;
pub use inline::{Inline, InlineImpl, InlineImplExt, InlineVTable};
pub use inline_collection::InlineCollection;
pub use inline_run::EmbeddedControlRun;
pub use inline_ui_container::InlineUIContainer;
pub use italic::Italic;
pub use line_break::LineBreak;
pub use line_spacing_rule::LineSpacingRule;
pub use paragraph::Paragraph;
pub use rich_text_document::{
    DocumentChangeKind, DocumentChangedEventArgs, DocumentEditBatch, DocumentError,
    DocumentTextChange, RichTextDocument, WeakRichTextDocument,
};
pub use run::Run;
pub use span::Span;
pub use text_element::{TextElement, TextElementImpl, TextElementImplExt, TextElementVTable};
pub use text_highlight_merge::{TextHighlightMerge, TextHighlightRegion};
pub use text_highlight_range::{TextHighlightError, TextHighlightRange, TextRangeCollection};
pub use text_highlighter::TextHighlighter;
pub use text_highlighter_collection::{TextHighlighterCollection, WeakTextHighlighterCollection};
pub use text_position::{LogicalDirection, TextGravity, TextPointer, TextPosition, TextRange};
pub use typography::Typography;
pub use typography_format::TypographyFormat;
pub use underline::Underline;

#[cfg(test)]
mod inline_tests;

#[cfg(test)]
mod rich_text_document_tests;

#[cfg(test)]
mod rich_paragraph_tests;

#[cfg(test)]
mod rich_language_tests;

#[cfg(test)]
mod text_highlighter_tests;

#[cfg(test)]
mod rich_paragraph_tabs_tests;
