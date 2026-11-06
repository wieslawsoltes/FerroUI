//! Native metadata for the keyboard's exact code-unit buffer.
//! Text-facing properties use the fallible helpers below. The exact buffer is
//! not a System.String alias or an implicitly displayable object.
use ferroboard_core::text::Utf16Text;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};

#[doc(hidden)]
pub struct Utf16TextMetadata;

fn parse(text: &str) -> Result<Utf16Text, std::convert::Infallible> {
    Ok(Utf16Text::from(text))
}

pub(crate) fn from_native_optional(text: Option<String>) -> Option<Utf16Text> {
    text.map(|text| Utf16Text::from(text.as_str()))
}

pub(crate) fn to_native(text: Utf16Text) -> Result<String, String> {
    text.to_utf8().map_err(|error| error.to_string())
}

pub(crate) fn to_native_optional(text: Option<Utf16Text>) -> Result<Option<String>, String> {
    text.map(to_native).transpose()
}

ferroui_base::ferro_markup_type!(struct Utf16TextMetadata as "Utf16Text" {
    namespace:"FerroBoard.Text",this:Utf16Text,handles:[Utf16Text],
    parse:parse
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::Utf16TextMetadata],contracts:[],generics:[],generic_contracts:[]}

pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(Utf16TextMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<Utf16Text>();
        ValueTypes::register_cast::<String, Utf16Text>(|text| Utf16Text::from(text.as_str()));
    });
}
