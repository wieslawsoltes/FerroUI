//! Native provider interface carrier retaining its actual Rc and synchronous trait dispatch.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::text::{ISuggestionProvider, SuggestionRequest};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
#[derive(Clone)]
pub struct SuggestionProviderRef(Rc<dyn ISuggestionProvider>);
impl SuggestionProviderRef {
    pub fn interface(&self) -> Rc<dyn ISuggestionProvider> {
        self.0.clone()
    }
    pub fn downcast<T: ISuggestionProvider>(&self) -> Option<Rc<T>> {
        let value: Rc<dyn std::any::Any> = self.0.clone();
        value.downcast().ok()
    }
}
impl std::ops::Deref for SuggestionProviderRef {
    type Target = dyn ISuggestionProvider;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl PartialEq for SuggestionProviderRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for SuggestionProviderRef {}
pub fn native_suggestion_provider(provider: Rc<dyn ISuggestionProvider>) -> SuggestionProviderRef {
    SuggestionProviderRef(provider)
}
#[doc(hidden)]
pub struct ISuggestionProviderMetadata;
ferroui_base::ferro_markup_type!(interface ISuggestionProviderMetadata as "ISuggestionProvider"{
 namespace:"FerroBoard.Text",this:SuggestionProviderRef,handles:[SuggestionProviderRef,Option<SuggestionProviderRef>],methods:[
 try fn Suggest(Option<SuggestionRequest>)->Option<crate::SuggestionReadOnlyList> => (|p:&SuggestionProviderRef,r:Option<SuggestionRequest>|p.suggest(r.as_ref()).map(|v|v.map(crate::SuggestionReadOnlyList::from_original)).map_err(crate::suggestion_list_metadata::failure))[Abstract,Virtual],
 try fn Learn(Option<Utf16Text>,Option<Utf16Text>)=>(|p:&SuggestionProviderRef,w:Option<Utf16Text>,previous:Option<Utf16Text>|p.learn(w.as_ref(),previous.as_ref()).map_err(crate::suggestion_list_metadata::failure))[Abstract,Virtual],
 try fn Forget(Option<Utf16Text>)=>(|p:&SuggestionProviderRef,w:Option<Utf16Text>|p.forget(w.as_ref()).map_err(crate::suggestion_list_metadata::failure))[Abstract,Virtual]
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::ISuggestionProviderMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(ISuggestionProviderMetadata::MARKUP);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<SuggestionProviderRef>();
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<SuggestionProviderRef>(),"::ferroboard_core_ferro::SuggestionProviderRef")]);}
}
