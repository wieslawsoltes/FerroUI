//! Native source ComputedKeyboard members over the original Core allocation. Its constructor is internal.
use crate::computed_collection_metadata::{
    ComputedKeyQuery, ComputedKeyReadOnlyList, ComputedRowsReadOnlyList,
};
use ferroboard_core::{
    keys::{KeyboardMode, LayoutDirection},
    layouts::{
        ComputedKey, ComputedKeyboard, ComputedRow, KeyComputeContext, KeyDataError,
        KeyboardGeometryOptions, PopupMapping, Subtype,
    },
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::{rc::Rc, sync::Arc};
fn failure(value: KeyDataError) -> MarkupInvokeError {
    crate::resource_error_metadata::failure(value)
}
fn call<T>(body: impl FnOnce() -> Result<T, KeyDataError>) -> Result<T, MarkupInvokeError> {
    body().map_err(failure)
}
#[doc(hidden)]
pub struct ComputedKeyboardMetadata;
ferroui_base::ferro_markup_type!(class ComputedKeyboardMetadata as "ComputedKeyboard"{
 namespace:"FerroBoard.Layouts",this:Rc<ComputedKeyboard>,handles:[ComputedKeyboard,Rc<ComputedKeyboard>,Option<Rc<ComputedKeyboard>>],attributes:[Sealed],
 properties:[
 Mode:KeyboardMode{get:|v:&Rc<ComputedKeyboard>|v.mode()},Subtype:Arc<Subtype>{get:|v:&Rc<ComputedKeyboard>|v.subtype()},Direction:LayoutDirection{get:|v:&Rc<ComputedKeyboard>|v.direction()},
 PopupMapping:Option<Rc<PopupMapping>>{get:|v:&Rc<ComputedKeyboard>|v.popup_mapping()},DefaultPopupMapping:Option<Rc<PopupMapping>>{get:|v:&Rc<ComputedKeyboard>|v.default_popup_mapping()},
 Rows:ComputedRowsReadOnlyList{get:|v:&Rc<ComputedKeyboard>|ComputedRowsReadOnlyList::from_rows(v.rows())},
 Keys:ComputedKeyQuery{get:|v:&Rc<ComputedKeyboard>|ComputedKeyQuery::from_query(v.keys())},VisibleKeys:ComputedKeyQuery{get:|v:&Rc<ComputedKeyboard>|ComputedKeyQuery::from_query(v.visible_keys())},
 RowCount:i32{get:|v:&Rc<ComputedKeyboard>|v.row_count()as i32},Version:i32{get:|v:&Rc<ComputedKeyboard>|v.version()},Context:Option<Rc<KeyComputeContext>>{get:|v:&Rc<ComputedKeyboard>|v.context()},Width:f64{get:|v:&Rc<ComputedKeyboard>|v.width()},Height:f64{get:|v:&Rc<ComputedKeyboard>|v.height()},
 CharacterKeys:ComputedKeyReadOnlyList{try_get:|v:&Rc<ComputedKeyboard>|call(||Ok(ComputedKeyReadOnlyList::from_row(ComputedRow::new(v.character_keys()?))))}
 ],methods:[
 try fn Compute(Option<Rc<KeyComputeContext>>) =>(|v:&Rc<ComputedKeyboard>,context|call(||v.compute_nullable(context))),
 try fn Layout(f64,f64) => (|v:&Rc<ComputedKeyboard>,width,height|{let options:Option<Rc<KeyboardGeometryOptions>>=None;call(||v.layout(width,height,options.as_deref()))}),
 try fn Layout(f64,f64,Option<Rc<KeyboardGeometryOptions>>) =>(|v:&Rc<ComputedKeyboard>,width,height,options:Option<Rc<KeyboardGeometryOptions>>|call(||v.layout(width,height,options.as_deref()))),
 try fn HitTest(f64,f64)->Option<Rc<ComputedKey>> =>(|v:&Rc<ComputedKeyboard>,x,y|call(||v.hit_test(x,y))),
 try fn FindKey(i32)->Option<Rc<ComputedKey>> =>(|v:&Rc<ComputedKeyboard>,code|call(||v.find_key(code)))
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::ComputedKeyboardMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(ComputedKeyboardMetadata::MARKUP);
    ValueTypes::register_global(ValueTypes::register_reference::<ComputedKeyboard>);
}
