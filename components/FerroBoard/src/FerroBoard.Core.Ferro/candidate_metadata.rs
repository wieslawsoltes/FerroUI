//! XAML construction and properties over native candidate objects.
use ferroboard_core::text::{Suggestion, SuggestionKind, Utf16Text};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

#[doc(hidden)]
pub struct SuggestionMetadata;
fn text(value: Option<String>) -> Option<Utf16Text> {
    value.map(|s| Utf16Text::from(s.as_str()))
}
fn native_text(value: Option<Utf16Text>) -> Result<Option<String>, std::string::FromUtf16Error> {
    value.map(|s| s.to_utf8()).transpose()
}
ferroui_base::ferro_markup_type!(class SuggestionMetadata as "Suggestion" {
    namespace:"FerroBoard.Text",
    this:Rc<Suggestion>,handles:[Rc<Suggestion>,Option<Rc<Suggestion>>,Suggestion],
    constructors:[
        ()=>(||Rc::new(Suggestion::new_nullable(None,SuggestionKind::Typed,0.0,false))),
        (Text:Option<String>,Kind:SuggestionKind)=>(|value,kind|Rc::new(Suggestion::new_nullable(text(value),kind,0.0,false))),
        (Text:Option<String>,Kind:SuggestionKind,Confidence:f64,IsAutoCommit:bool)=>(|value,kind,confidence,auto|Rc::new(Suggestion::new_nullable(text(value),kind,confidence,auto)))
    ],
    properties:[
        Text:Option<String>{try_get:|s:&Rc<Suggestion>|native_text(s.text()),set:|s:&Rc<Suggestion>,v|s.set_text(text(v))},
        Kind:SuggestionKind{get:|s:&Rc<Suggestion>|s.kind(),set:|s:&Rc<Suggestion>,v|s.set_kind(v)},
        Confidence:f64{get:|s:&Rc<Suggestion>|s.confidence(),set:|s:&Rc<Suggestion>,v|s.set_confidence(v)},
        IsAutoCommit:bool{get:|s:&Rc<Suggestion>|s.is_auto_commit(),set:|s:&Rc<Suggestion>,v|s.set_is_auto_commit(v)},
        Description:Option<String>{try_get:|s:&Rc<Suggestion>|native_text(s.description()),set:|s:&Rc<Suggestion>,v|s.set_description(text(v))},
        IsRemovable:bool{get:|s:&Rc<Suggestion>|s.is_removable(),set:|s:&Rc<Suggestion>,v|s.set_is_removable(v)}
    ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::SuggestionMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(SuggestionMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_reference::<Suggestion>);
}
