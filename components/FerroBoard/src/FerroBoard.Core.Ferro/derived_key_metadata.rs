//! Concrete derived key metadata over original objects and shared mutable native arrays.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::{
    keys::KeyType,
    layouts::{
        AbstractKeyData, AutoTextKeyData, ITextKeyData, Key, KeyChanges, KeyDataError,
        MultiTextKeyData, PopupSet, TextKey, TextKeyData,
    },
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::{any::Any, rc::Rc};
pub(crate) fn failure(error: KeyDataError) -> MarkupInvokeError {
    crate::resource_error_metadata::failure(error)
}
#[doc(hidden)]
pub struct AutoTextKeyDataMetadata;
ferroui_base::ferro_markup_type!(class AutoTextKeyDataMetadata as "AutoTextKeyData"{
 namespace:"FerroBoard.Layouts",this:Rc<AutoTextKeyData>,handles:[AutoTextKeyData,Rc<AutoTextKeyData>,Option<Rc<AutoTextKeyData>>],base:TextKey,attributes:[Sealed],
 constructors:[()=>(||Rc::new(AutoTextKeyData{base:TextKeyData::default()}))],
 methods:[
 try fn Compute(Option<crate::KeyEvaluatorRef>)->Option<TextKey> => (|k:&Rc<AutoTextKeyData>,evaluator:Option<crate::KeyEvaluatorRef>|k.clone().compute_nullable(evaluator.as_deref()).map_err(failure))[Override,Virtual],
 fn With()->TextKey=> (|k:&Rc<AutoTextKeyData>|{let code=None;let label=None;let key_type=None;let popup=None;let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>)->TextKey=> (|k:&Rc<AutoTextKeyData>,code|{let label=None;let key_type=None;let popup=None;let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>,Option<Utf16Text>)->TextKey=> (|k:&Rc<AutoTextKeyData>,code,label|{let key_type=None;let popup=None;let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>)->TextKey=> (|k:&Rc<AutoTextKeyData>,code,label,key_type|{let popup=None;let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>,Option<Rc<PopupSet>>)->TextKey=> (|k:&Rc<AutoTextKeyData>,code,label,key_type,popup|{let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>,Option<Rc<PopupSet>>,Option<i32>)->TextKey=>(|k:&Rc<AutoTextKeyData>,code,label,key_type,popup,group_id|k.with(KeyChanges{code,label,key_type,popup,group_id}))[Override,Virtual]
 ]
});
#[doc(hidden)]
pub struct MultiTextKeyDataMetadata;
ferroui_base::ferro_markup_type!(class MultiTextKeyDataMetadata as "MultiTextKeyData"{
 namespace:"FerroBoard.Layouts",this:Rc<MultiTextKeyData>,handles:[MultiTextKeyData,Rc<MultiTextKeyData>,Option<Rc<MultiTextKeyData>>],base:TextKey,attributes:[Sealed],
 constructors:[()=>(||Rc::new(MultiTextKeyData::default()))],
 properties:[
 CodePoints:Option<ferroboard_core::layouts::CodePointArray>{get:|k:&Rc<MultiTextKeyData>|k.code_points(),set:|k:&Rc<MultiTextKeyData>,value|k.set_code_points(value)},
 ProducesText:bool{try_get:|k:&Rc<MultiTextKeyData>|k.try_produces_text().map_err(failure)}[Override,Virtual]],
 methods:[
 try fn AsString(bool)->Option<Utf16Text> => (|k:&Rc<MultiTextKeyData>,display|k.as_string(display).map_err(failure))[Override,Virtual],
 fn With()->TextKey=> (|k:&Rc<MultiTextKeyData>|{let code=None;let label=None;let key_type=None;let popup=None;let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>)->TextKey=> (|k:&Rc<MultiTextKeyData>,code|{let label=None;let key_type=None;let popup=None;let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>,Option<Utf16Text>)->TextKey=> (|k:&Rc<MultiTextKeyData>,code,label|{let key_type=None;let popup=None;let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>)->TextKey=> (|k:&Rc<MultiTextKeyData>,code,label,key_type|{let popup=None;let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>,Option<Rc<PopupSet>>)->TextKey=> (|k:&Rc<MultiTextKeyData>,code,label,key_type,popup|{let group_id=None;k.with(KeyChanges{code,label,key_type,popup,group_id})})[Override,Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>,Option<Rc<PopupSet>>,Option<i32>)->TextKey=>(|k:&Rc<MultiTextKeyData>,code,label,key_type,popup,group_id|k.with(KeyChanges{code,label,key_type,popup,group_id}))[Override,Virtual]
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::AutoTextKeyDataMetadata,crate::MultiTextKeyDataMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        AutoTextKeyDataMetadata::MARKUP,
        MultiTextKeyDataMetadata::MARKUP,
    ]);
    ValueTypes::register_global(|| {
        ValueTypes::register_reference::<AutoTextKeyData>();
        ValueTypes::register_reference::<MultiTextKeyData>();
        ValueTypes::register_cast::<Rc<AutoTextKeyData>, TextKey>(|k| k.clone());
        ValueTypes::register_cast::<Rc<MultiTextKeyData>, TextKey>(|k| k.clone());
        ValueTypes::register_cast::<Rc<AutoTextKeyData>, Key>(|k| k.clone());
        ValueTypes::register_cast::<Rc<MultiTextKeyData>, Key>(|k| k.clone());
        ValueTypes::register_boxed_cast::<TextKey, Rc<AutoTextKeyData>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<TextKey>()?.clone();
            original.downcast::<AutoTextKeyData>().ok()
        });
        ValueTypes::register_boxed_cast::<TextKey, Rc<MultiTextKeyData>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<TextKey>()?.clone();
            original.downcast::<MultiTextKeyData>().ok()
        });
        ValueTypes::register_boxed_cast::<Key, Rc<AutoTextKeyData>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<Key>()?.clone();
            original.downcast::<AutoTextKeyData>().ok()
        });
        ValueTypes::register_boxed_cast::<Key, Rc<MultiTextKeyData>>(|k| {
            let original: Rc<dyn Any> = k.downcast_ref::<Key>()?.clone();
            original.downcast::<MultiTextKeyData>().ok()
        });
    });
}
