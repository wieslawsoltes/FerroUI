//! Native metadata over the original shared key objects; no mirrored key state.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::{
    keys::KeyType,
    layouts::{Key, KeyChanges, KeyDataError, PopupSet, TextKey, TextKeyData},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
fn factory_error(e: KeyDataError) -> MarkupInvokeError {
    crate::resource_error_metadata::failure(e)
}
#[doc(hidden)]
pub struct TextKeyDataMetadata;
ferroui_base::ferro_markup_type!(class TextKeyDataMetadata as "TextKeyData"{
 namespace:"FerroBoard.Layouts",this:TextKey,handles:[TextKey,Option<TextKey>],base:Key,
 constructors:[()=>(||Rc::new(TextKeyData::default()) as TextKey)],
 properties:[
 Type:KeyType{get:|k:&TextKey|k.data().key_type(),set:|k:&TextKey,v|k.data().set_type(v)},
 Code:i32{get:|k:&TextKey|k.data().code(),set:|k:&TextKey,v|k.data().set_code(v)},
 Label:Option<String>{try_get:|k:&TextKey|crate::utf16_metadata::to_native_optional(k.data().label()),set:|k:&TextKey,v|k.data().set_label(crate::utf16_metadata::from_native_optional(v))},
 GroupId:i32{get:|k:&TextKey|k.data().group_id(),set:|k:&TextKey,v|k.data().set_group_id(v)},
 Popup:Option<Rc<PopupSet>>{get:|k:&TextKey|k.data().popup(),set:|k:&TextKey,v|k.data().set_popup(v)},
 PopupLookupLabel:Option<String>{try_get:|k:&TextKey|crate::utf16_metadata::to_native_optional(k.data().popup_lookup_label())},
 IsSpace:bool{get:|k:&TextKey|k.data().is_space()},
 ProducesText:bool{try_get:|k:&TextKey|k.produces_text().map_err(crate::resource_error_metadata::failure)}[Virtual]
 ],methods:[
 try fn Compute(Option<crate::KeyEvaluatorRef>)->Option<TextKey> => (|k:&TextKey,evaluator:Option<crate::KeyEvaluatorRef>|k.clone().compute_nullable(evaluator.as_deref()).map_err(crate::derived_key_metadata::failure))[Override,Virtual],
 try fn AsString(bool)->Option<Utf16Text> => (|k:&TextKey,display|k.as_string(display).map_err(crate::derived_key_metadata::failure))[Override,Virtual],
 static try fn Char(Option<Utf16Text>)->TextKey => (|text|{let popup=None;TextKeyData::char_nullable(text,popup).map(|k|k as TextKey).map_err(factory_error)}),
 static try fn Char(Option<Utf16Text>,Option<Rc<PopupSet>>)->TextKey => (|text,popup|TextKeyData::char_nullable(text,popup).map(|k|k as TextKey).map_err(factory_error)),
 static fn Function(i32)->TextKey => (|code|{let kind=KeyType(2);let label=None;TextKeyData::function(code,kind,label) as TextKey}),
 static fn Function(i32,KeyType)->TextKey => (|code,kind|{let label=None;TextKeyData::function(code,kind,label) as TextKey}),
 static fn Function(i32,KeyType,Option<Utf16Text>)->TextKey => (|code,kind,label|TextKeyData::function(code,kind,label) as TextKey),
 try fn ToString()->Option<Utf16Text> => (|key:&TextKey|key.to_text().map_err(crate::resource_error_metadata::failure))[Virtual],
 fn With()->TextKey => (|key:&TextKey|{let code=None;let label=None;let kind=None;let popup=None;let group_id=None;key.with(KeyChanges{code,label,key_type:kind,popup,group_id})})[Virtual],
 fn With(Option<i32>)->TextKey => (|key:&TextKey,code|{let label=None;let kind=None;let popup=None;let group_id=None;key.with(KeyChanges{code,label,key_type:kind,popup,group_id})})[Virtual],
 fn With(Option<i32>,Option<Utf16Text>)->TextKey => (|key:&TextKey,code,label|{let kind=None;let popup=None;let group_id=None;key.with(KeyChanges{code,label,key_type:kind,popup,group_id})})[Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>)->TextKey => (|key:&TextKey,code,label,kind|{let popup=None;let group_id=None;key.with(KeyChanges{code,label,key_type:kind,popup,group_id})})[Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>,Option<Rc<PopupSet>>)->TextKey => (|key:&TextKey,code,label,kind,popup|{let group_id=None;key.with(KeyChanges{code,label,key_type:kind,popup,group_id})})[Virtual],
 fn With(Option<i32>,Option<Utf16Text>,Option<KeyType>,Option<Rc<PopupSet>>,Option<i32>)->TextKey => (|key:&TextKey,code,label,kind,popup,group_id|key.with(KeyChanges{code,label,key_type:kind,popup,group_id}))[Virtual]
 ]
});
#[doc(hidden)]
pub struct PopupSetMetadata;
ferroui_base::ferro_markup_type!(class PopupSetMetadata as "PopupSet"{
 namespace:"FerroBoard.Layouts",this:Rc<PopupSet>,handles:[Rc<PopupSet>,Option<Rc<PopupSet>> ],attributes:[Sealed],
 constructors:[()=>(||Rc::new(PopupSet::default()))],properties:[
 Main:Option<Key>{get:|p:&Rc<PopupSet>|p.main(),set:|p:&Rc<PopupSet>,v|p.set_main(v)},
 Relevant:Option<crate::KeyReadOnlyList>{get:|p:&Rc<PopupSet>|p.relevant().map(crate::KeyReadOnlyList::from_original),set:|p:&Rc<PopupSet>,v:Option<crate::KeyReadOnlyList>|p.set_relevant(v.map(|v|v.original()))},
 IsEmpty:bool{try_get:|p:&Rc<PopupSet>|p.is_empty()}]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::TextKeyDataMetadata,crate::PopupSetMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[TextKeyDataMetadata::MARKUP, PopupSetMetadata::MARKUP]);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<TextKey>();
        ValueTypes::register_nullable::<Rc<PopupSet>>();
        ValueTypes::register_cast::<TextKey, Key>(|key| key.clone());
        // A base reference to a text key boxes the same text-key identity as the derived view.
        ValueTypes::register_boxed_cast::<Key, Option<ferroui_base::BoxedValue>>(|value| {
            let key = value.downcast_ref::<Key>()?;
            Some(Some(match key.clone().into_text() {
                Some(text) => Rc::new(text) as ferroui_base::BoxedValue,
                None => value.clone(),
            }))
        });

        // A base-typed key keeps its original object; downcast succeeds only for text subclasses.
        ValueTypes::register_boxed_cast::<Key, TextKey>(|key| {
            key.downcast_ref::<Key>()?.clone().into_text()
        });
    });
}
