//! Request construction and properties over shared Rust data. No CLR record/delegate runtime.
use ferroboard_core::text::{SuggestionRequest, Utf16Text};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
fn text(value: Option<String>) -> Option<Utf16Text> {
    value.map(|s| Utf16Text::from(s.as_str()))
}
fn native_text(value: Option<Utf16Text>) -> Result<Option<String>, std::string::FromUtf16Error> {
    value.map(|s| s.to_utf8()).transpose()
}
fn culture(s: &SuggestionRequest) -> Option<String> {
    crate::locale_metadata::name(s.culture())
}
fn set_culture(
    s: &SuggestionRequest,
    value: Option<String>,
) -> Result<(), ferroboard_core::text::UnsupportedCulture> {
    s.set_culture(crate::locale_metadata::parse(value)?);
    Ok(())
}
#[doc(hidden)]
pub struct SuggestionRequestMetadata;
ferroui_base::ferro_markup_type!(class SuggestionRequestMetadata as "SuggestionRequest" {
    namespace:"FerroBoard.Text",this:SuggestionRequest,
    handles:[SuggestionRequest,Option<SuggestionRequest>],
    constructors:[()=>SuggestionRequest::default],
    properties:[
        Word:Option<String>{try_get:|s:&SuggestionRequest|native_text(s.word()),set:|s:&SuggestionRequest,v|s.set_word(text(v))},
        PrecedingText:Option<String>{try_get:|s:&SuggestionRequest|native_text(s.preceding_text()),set:|s:&SuggestionRequest,v|s.set_preceding_text(text(v))},
        MaxCount:i32{get:SuggestionRequest::max_count,set:SuggestionRequest::set_max_count},
        AllowAutoCorrect:bool{get:SuggestionRequest::allow_auto_correct,set:SuggestionRequest::set_allow_auto_correct},
        Culture:Option<String>{get:culture,try_set:set_culture},
        KeyProximity:Option<crate::KeyProximityRef>{get:|s:&SuggestionRequest|s.key_proximity().map(crate::KeyProximityRef::new),set:|s:&SuggestionRequest,v:Option<crate::KeyProximityRef>|s.set_key_proximity(v.map(|v|v.callback()))}
    ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::SuggestionRequestMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(SuggestionRequestMetadata::MARKUP);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<SuggestionRequest>();
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[
            (||std::any::TypeId::of::<SuggestionRequest>(),"::ferroboard_core::text::SuggestionRequest")
        ]);
    }
}
