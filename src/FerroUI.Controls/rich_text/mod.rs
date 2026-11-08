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

/// Registers every type of this file that registers when it is first used: what the
/// tables of types (`type-tables`) register at start-up.
pub(crate) fn register_value_type_table() {
    use ferroui_base::data::core::ValueTypes;
    ValueTypes::register_used::<RichEditClipboardFormat>();
    ValueTypes::register_used::<FindOptions>();
    ValueTypes::register_used::<RangeGravity>();
    ValueTypes::register_used::<SelectionOptions>();
    ValueTypes::register_used::<SelectionType>();
    ValueTypes::register_used::<TextGetOptions>();
    ValueTypes::register_used::<TextSetOptions>();
    ValueTypes::register_used::<TextStreamEncoding>();
    ValueTypes::register_used::<TextRangeUnit>();
    ValueTypes::register_used::<LetterCase>();
}

impl ferroui_base::data::core::ValueTypeRegistration for RichEditClipboardFormat {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<RichEditClipboardFormat>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for FindOptions {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<FindOptions>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for RangeGravity {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<RangeGravity>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for SelectionOptions {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<SelectionOptions>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for SelectionType {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<SelectionType>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TextGetOptions {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<TextGetOptions>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TextSetOptions {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<TextSetOptions>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TextStreamEncoding {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<TextStreamEncoding>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TextRangeUnit {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<TextRangeUnit>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for LetterCase {
    fn register_value_conversions() {
        use ferroui_base::data::core::ValueTypes;
        ValueTypes::register_nullable::<LetterCase>();
    }
}

