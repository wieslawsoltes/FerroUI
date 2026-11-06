//! XAML carrier of a native suggestion-provider factory, supplied in Rust.
use ferroboard_core::engine::SuggestionProviderFactory;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;

#[derive(Clone)]
pub struct SuggestionProviderFactoryRef(SuggestionProviderFactory);
impl SuggestionProviderFactoryRef {
    pub fn new(callback: SuggestionProviderFactory) -> Self {
        Self(callback)
    }
    pub fn callback(&self) -> SuggestionProviderFactory {
        self.0.clone()
    }
}
impl PartialEq for SuggestionProviderFactoryRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SuggestionProviderFactoryRef {}
#[doc(hidden)]
pub struct SuggestionProviderFactoryMetadata;
ferroui_base::ferro_markup_type!(class SuggestionProviderFactoryMetadata as "SuggestionProviderFactory" {
    namespace:"FerroBoard.Engine",this:SuggestionProviderFactoryRef,
    handles:[SuggestionProviderFactoryRef,Option<SuggestionProviderFactoryRef>]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::SuggestionProviderFactoryMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(SuggestionProviderFactoryMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_nullable::<SuggestionProviderFactoryRef>);
}
