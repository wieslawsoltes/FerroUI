//! Native source CurrencySet class over the original Core allocation and collection handles.
use ferroboard_core::layouts::{CurrencySet, KeyDataError, TextKey};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
fn failure(e: KeyDataError) -> MarkupInvokeError {
    crate::resource_error_metadata::failure(e)
}
#[doc(hidden)]
pub struct CurrencySetMetadata;
ferroui_base::ferro_markup_type!(class CurrencySetMetadata as "CurrencySet"{
 namespace:"FerroBoard.Layouts",this:Rc<CurrencySet>,handles:[CurrencySet,Rc<CurrencySet>,Option<Rc<CurrencySet>>],attributes:[Sealed,RequiredMember],
 constructors:[()=>(||Rc::new(CurrencySet::default()))],
 properties:[
 ExtensionId:Option<String>{try_get:|v:&Rc<CurrencySet>|crate::utf16_metadata::to_native_optional(v.extension_id()),set:|v:&Rc<CurrencySet>,n|v.set_extension_id(crate::utf16_metadata::from_native_optional(n))},
 Id:Option<String>{try_get:|v:&Rc<CurrencySet>|crate::utf16_metadata::to_native_optional(v.id()),set:|v:&Rc<CurrencySet>,n|v.set_id(crate::utf16_metadata::from_native_optional(n))}[RequiredMember],
 Label:Option<String>{try_get:|v:&Rc<CurrencySet>|crate::utf16_metadata::to_native_optional(v.label()),set:|v:&Rc<CurrencySet>,n|v.set_label(crate::utf16_metadata::from_native_optional(n))},
 Slots:Option<crate::TextKeyReadOnlyList>{get:|v:&Rc<CurrencySet>|v.slots().map(crate::TextKeyReadOnlyList::from_original),set:|v:&Rc<CurrencySet>,n:Option<crate::TextKeyReadOnlyList>|v.set_slots(n.map(|v|v.original()))}[RequiredMember]
 ],methods:[try fn GetSlot(i32)->Option<TextKey> =>(|v:&Rc<CurrencySet>,code|v.get_slot(code).map_err(failure))]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::CurrencySetMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(CurrencySetMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_reference::<CurrencySet>();
    });
}
