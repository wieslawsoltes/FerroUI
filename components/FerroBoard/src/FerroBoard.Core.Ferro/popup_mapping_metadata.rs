//! Original sealed PopupMapping constructor, init properties and lookup on its actual Core object.
use crate::popup_dictionary_metadata::PopupVariationsDictionary;
use ferroboard_core::text::Utf16Text;
use ferroboard_core::{
    keys::KeyVariation,
    layouts::{KeyDataError, PopupMapping, PopupSet},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
fn failure(value: KeyDataError) -> MarkupInvokeError {
    crate::resource_error_metadata::failure(value)
}
#[doc(hidden)]
pub struct PopupMappingMetadata;
ferroui_base::ferro_markup_type!(class PopupMappingMetadata as "PopupMapping"{
 namespace:"FerroBoard.Layouts",this:Rc<PopupMapping>,handles:[PopupMapping,Rc<PopupMapping>,Option<Rc<PopupMapping>>],attributes:[Sealed],
 constructors:[()=>(||Rc::new(PopupMapping::default()))],
 properties:[
 ExtensionId:Option<String>{try_get:|v:&Rc<PopupMapping>|crate::utf16_metadata::to_native_optional(v.extension_id()),set:|v:&Rc<PopupMapping>,n|v.set_extension_id(crate::utf16_metadata::from_native_optional(n))},
 Id:Option<String>{try_get:|v:&Rc<PopupMapping>|crate::utf16_metadata::to_native_optional(v.id()),set:|v:&Rc<PopupMapping>,n|v.set_id(crate::utf16_metadata::from_native_optional(n))},
 Authors:Option<crate::StringReadOnlyList>{get:|v:&Rc<PopupMapping>|v.authors().map(crate::resource_string_list::native),set:|v:&Rc<PopupMapping>,n:Option<crate::StringReadOnlyList>|v.set_authors_read_only(n.map(crate::resource_string_list::original))},
 Mappings:Option<PopupVariationsDictionary>{get:|v:&Rc<PopupMapping>|v.mappings().map(PopupVariationsDictionary::from_original),set:|v:&Rc<PopupMapping>,n:Option<PopupVariationsDictionary>|v.set_mappings(n.map(|v|v.original()))}
 ],methods:[try fn Get(KeyVariation,Option<Utf16Text>)->Option<Rc<PopupSet>> =>(|v:&Rc<PopupMapping>,variation,label:Option<Utf16Text>|v.get_nullable(variation,label.as_ref()).map_err(failure))]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::PopupMappingMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(PopupMappingMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_reference::<PopupMapping>);
}
