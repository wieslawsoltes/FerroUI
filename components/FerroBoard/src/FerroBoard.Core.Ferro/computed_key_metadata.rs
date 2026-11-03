//! Public source getters and ToString over original ComputedKey references. Private/internal
//! constructor and setter access is preserved; numeric diagnostics use Rust formatting.
use ferroboard_core::text::Utf16Text;
use ferroboard_core::{
    keys::{KeyIcon, KeyType},
    layouts::{ComputedKey, Key, KeyRect, TextKey},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
#[doc(hidden)]
pub struct ComputedKeyMetadata;
ferroui_base::ferro_markup_type!(class ComputedKeyMetadata as "ComputedKey" {
 namespace:"FerroBoard.Layouts",this:Rc<ComputedKey>,handles:[ComputedKey,Rc<ComputedKey>,Option<Rc<ComputedKey>>],attributes:[Sealed],
 properties:[
 Source:Option<Key>{get:|k:&Rc<ComputedKey>|k.source()},
 Row:i32{get:|k:&Rc<ComputedKey>|k.row()},Column:i32{get:|k:&Rc<ComputedKey>|k.column()},
 Data:TextKey{get:|k:&Rc<ComputedKey>|k.data()},Code:i32{get:|k:&Rc<ComputedKey>|k.code()},Type:KeyType{get:|k:&Rc<ComputedKey>|k.key_type()},
 Label:Option<String>{try_get:|k:&Rc<ComputedKey>|crate::utf16_metadata::to_native_optional(k.label())},Icon:KeyIcon{get:|k:&Rc<ComputedKey>|k.icon()},
 HintLabel:Option<String>{try_get:|k:&Rc<ComputedKey>|crate::utf16_metadata::to_native_optional(k.hint_label())},SubLabel:Option<String>{try_get:|k:&Rc<ComputedKey>|crate::utf16_metadata::to_native_optional(k.sub_label())},HintKey:Option<TextKey>{get:|k:&Rc<ComputedKey>|k.hint_key()},
 PopupKeys:crate::PopupKeysView{get:|k:&Rc<ComputedKey>|crate::PopupKeysView::new(k.popup_keys())},
 PopupDefaultIndex:i32{get:|k:&Rc<ComputedKey>|k.popup_default_index()},IsVisible:bool{get:|k:&Rc<ComputedKey>|k.is_visible()},IsEnabled:bool{get:|k:&Rc<ComputedKey>|k.is_enabled()},
 WidthFactor:f64{get:|k:&Rc<ComputedKey>|k.width_factor()},Grow:f64{get:|k:&Rc<ComputedKey>|k.grow()},Shrink:f64{get:|k:&Rc<ComputedKey>|k.shrink()},
 TouchBounds:KeyRect{get:|k:&Rc<ComputedKey>|k.touch_bounds()},VisibleBounds:KeyRect{get:|k:&Rc<ComputedKey>|k.visible_bounds()},
 HasPopup:bool{get:|k:&Rc<ComputedKey>|k.has_popup()},IsCharacterKey:bool{try_get:character_key},Output:Option<String>{try_get:|k:&Rc<ComputedKey>|(k.output().map_err(crate::derived_key_metadata::failure)).map_err(|e|e.to_string()).and_then(crate::utf16_metadata::to_native_optional)}
 ],
 methods:[try fn ToString()->Utf16Text=>(|key:&Rc<ComputedKey>|key.to_text().map_err(crate::derived_key_metadata::failure))[Virtual,Override]]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::ComputedKeyMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[ComputedKeyMetadata::MARKUP]);
    ValueTypes::register_global(|| {
        ValueTypes::register_reference::<ComputedKey>();
        ValueTypes::register_nullable::<Rc<ComputedKey>>();
    });
}

fn character_key(key: &Rc<ComputedKey>) -> Result<bool, MarkupInvokeError> {
    key.is_character_key()
        .map_err(crate::derived_key_metadata::failure)
}
