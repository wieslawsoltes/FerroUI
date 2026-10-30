use ferroui_base::metadata::{MarkupType, MarkupTyped};

mod rich_edit_text_document;
mod rich_edit_text_field_story;
mod rich_text_editor;
mod rich_text_field_story;
mod rich_text_link;
mod rich_text_transfer;
mod rtf;

pub(crate) use rich_edit_text_document::{unit_ranges as rich_text_unit_ranges, UnicodeTextUnit};
pub use rich_edit_text_document::{
    FindOptions, LetterCase, RangeGravity, RichEditTextDocument, RichEditTextRange,
    RichEditTextSelection, SelectionOptions, SelectionType, TextFindOptions, TextFormatValue,
    TextGetOptions, TextOperationError, TextRangeUnit, TextSetOptions, TextStreamEncoding,
    TextStreamOptions,
};
pub(crate) use rich_text_editor::utf16_prefix;
// The editor, the RTF reader and writer and the field stories are the machinery under
// the text object model, not WinUI API: hidden from the documentation and free to
// change (#243 review, S12).
#[doc(hidden)]
pub use rich_text_editor::{
    EditError, EditKind, EditorSelection, RichTextCompositionChange, RichTextEditor,
};
pub use rich_text_transfer::{RichEditClipboardFormat, RichTextPasteFormat, RichTextTransfer};
pub use rtf::RtfError;
#[doc(hidden)]
pub use rtf::{RtfLimits, RtfReader, RtfWriter};

#[doc(hidden)]
pub use rich_edit_text_field_story::{
    FieldInstructionBoundary, RichEditTextFieldRange, RichEditTextFieldStory, TextFieldError,
};
#[doc(hidden)]
pub use rich_text_field_story::FieldStoryOffset;

/// The markup types of the text object model declared next to their types, registered
/// with the other types of the crate.
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <RichEditClipboardFormat as MarkupTyped>::MARKUP,
    <FindOptions as MarkupTyped>::MARKUP,
    <RangeGravity as MarkupTyped>::MARKUP,
    <SelectionOptions as MarkupTyped>::MARKUP,
    <SelectionType as MarkupTyped>::MARKUP,
    <TextGetOptions as MarkupTyped>::MARKUP,
    <TextSetOptions as MarkupTyped>::MARKUP,
    <TextStreamEncoding as MarkupTyped>::MARKUP,
    <TextRangeUnit as MarkupTyped>::MARKUP,
    <LetterCase as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the value types of [`MARKUP_TYPES`].
pub(crate) fn register_value_types() {
    use ferroui_base::data::core::ValueTypes;
    ValueTypes::register_nullable::<RichEditClipboardFormat>();
    ValueTypes::register_nullable::<FindOptions>();
    ValueTypes::register_nullable::<RangeGravity>();
    ValueTypes::register_nullable::<SelectionOptions>();
    ValueTypes::register_nullable::<SelectionType>();
    ValueTypes::register_nullable::<TextGetOptions>();
    ValueTypes::register_nullable::<TextSetOptions>();
    ValueTypes::register_nullable::<TextStreamEncoding>();
    ValueTypes::register_nullable::<TextRangeUnit>();
    ValueTypes::register_nullable::<LetterCase>();
}
