//! Structured block and inline content, document ranges and editing snapshots.

use ferroui_base::metadata::{MarkupType, MarkupTyped};

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
pub(crate) use rich_text_document::{BlockJournal, DocumentDelta};
pub use run::Run;
pub use span::Span;
pub use text_element::{TextElement, TextElementImpl, TextElementImplExt, TextElementVTable};
pub use text_highlight_merge::{TextHighlightMerge, TextHighlightRegion};
pub use text_highlight_range::{TextHighlightError, TextHighlightRange, TextRangeCollection};
pub use text_highlighter::TextHighlighter;
pub use text_highlighter_collection::{TextHighlighterCollection, WeakTextHighlighterCollection};
pub use text_position::{TextGravity, TextPointer, TextPosition, TextRange};
pub use typography::Typography;
pub use typography_format::TypographyFormat;
pub use underline::Underline;

/// The markup types of WinUI's rich text declared next to their types
/// (`docs/porting/winui-rich-text.md`), registered with the other types of the crate.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <Typography as MarkupTyped>::MARKUP,
    <BlockCollection as MarkupTyped>::MARKUP,
    <RichTextDocument as MarkupTyped>::MARKUP,
    <LineStackingStrategy as MarkupTyped>::MARKUP,
    <LineSpacingRule as MarkupTyped>::MARKUP,
    <TextGravity as MarkupTyped>::MARKUP,
    <UnderlineStyle as MarkupTyped>::MARKUP,
    <HyperlinkClickEventArgs as MarkupTyped>::MARKUP,
    <TextHighlightRange as MarkupTyped>::MARKUP,
    <TextRangeCollection as MarkupTyped>::MARKUP,
    <TextHighlighterCollection as MarkupTyped>::MARKUP,
];

/// Registers what the untyped value conversions need of the types of this file that
/// no first use shows: the nullable forms and the casts of the types that members of
/// markup types hold (compiled markup converts a value for such a member before it
/// sets it, and setting it registers nothing). Every other type registers when it is
/// first used ([`ValueTypeRegistration`](ferroui_base::data::core::ValueTypeRegistration)).
pub(crate) fn register_value_types() {
    use ferroui_base::data::core::ValueTypes;
    ValueTypes::register_reference::<TextHighlightRange>();
}

/// Registers every type of this file that registers when it is first used: what the
/// tables of types (`type-tables`) register at start-up.
pub(crate) fn register_value_type_table() {
    use ferroui_base::data::core::ValueTypes;
    ValueTypes::register_used::<BlockCollection>();
    ValueTypes::register_used::<RichTextDocument>();
    ValueTypes::register_used::<LineStackingStrategy>();
    ValueTypes::register_used::<LineSpacingRule>();
    ValueTypes::register_used::<TextGravity>();
    ValueTypes::register_used::<UnderlineStyle>();
    ValueTypes::register_used::<TextHighlightRange>();
    ValueTypes::register_used::<TextRangeCollection>();
    ValueTypes::register_used::<TextHighlighterCollection>();
}

impl ferroui_base::data::core::ValueTypeRegistration for BlockCollection {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<BlockCollection>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for RichTextDocument {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<RichTextDocument>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for LineStackingStrategy {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<LineStackingStrategy>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for LineSpacingRule {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<LineSpacingRule>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TextGravity {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<TextGravity>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for UnderlineStyle {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<UnderlineStyle>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TextHighlightRange {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<TextHighlightRange>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TextRangeCollection {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<TextRangeCollection>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TextHighlighterCollection {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<TextHighlighterCollection>();
    }
}

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
