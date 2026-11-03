//! Native context configuration and interface dispatch.
use ferroboard_core::{
    editor::EnterAction,
    keys::{KeyVariation, KeyboardMode, LayoutDirection, ShiftState},
    layouts::{CurrencySet, KeyComputeContext},
    settings::{KeyHintMode, SpaceBarMode, UtilityKeyAction},
    text::{UnsupportedCulture, Utf16Text},
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
fn set_culture(
    value: &Rc<KeyComputeContext>,
    name: Option<String>,
) -> Result<(), UnsupportedCulture> {
    value.set_culture(crate::locale_metadata::parse(name)?);
    Ok(())
}
fn text(value: Option<String>) -> Option<Utf16Text> {
    value.map(|v| v.as_str().into())
}
#[doc(hidden)]
pub struct KeyComputeContextMetadata;
ferroui_base::ferro_markup_type!(class KeyComputeContextMetadata as "KeyComputeContext" {
 namespace:"FerroBoard.Layouts",this:Rc<KeyComputeContext>,
 handles:[KeyComputeContext,Rc<KeyComputeContext>,Option<Rc<KeyComputeContext>>],interfaces:[crate::KeyEvaluatorRef],
 constructors:[()=>(||Rc::new(KeyComputeContext::default()))],
 properties:[
 Mode:KeyboardMode{get:|v:&Rc<KeyComputeContext>|v.mode(),set:|v:&Rc<KeyComputeContext>,n|v.set_mode(n)},
 ShiftState:ShiftState{get:|v:&Rc<KeyComputeContext>|v.shift_state(),set:|v:&Rc<KeyComputeContext>,n|v.set_shift_state(n)},
 Variation:KeyVariation{get:|v:&Rc<KeyComputeContext>|v.variation(),set:|v:&Rc<KeyComputeContext>,n|v.set_variation(n)},
 Direction:LayoutDirection{get:|v:&Rc<KeyComputeContext>|v.direction(),set:|v:&Rc<KeyComputeContext>,n|v.set_direction(n)},
 IsCharHalfWidth:bool{get:|v:&Rc<KeyComputeContext>|v.is_char_half_width(),set:|v:&Rc<KeyComputeContext>,n|v.set_is_char_half_width(n)},
 IsKanaKata:bool{get:|v:&Rc<KeyComputeContext>|v.is_kana_kata(),set:|v:&Rc<KeyComputeContext>,n|v.set_is_kana_kata(n)},
 Culture:Option<String>{get:|v:&Rc<KeyComputeContext>|crate::locale_metadata::name(v.culture()),try_set:set_culture},
 EnterAction:EnterAction{get:|v:&Rc<KeyComputeContext>|v.enter_action(),set:|v:&Rc<KeyComputeContext>,n|v.set_enter_action(n)},
 CurrencySet:Option<Rc<CurrencySet>>{get:|v:&Rc<KeyComputeContext>|v.currency_set(),set:|v:&Rc<KeyComputeContext>,n|v.set_currency_set(n)},
 HasMultipleSubtypes:bool{get:|v:&Rc<KeyComputeContext>|v.has_multiple_subtypes(),set:|v:&Rc<KeyComputeContext>,n|v.set_has_multiple_subtypes(n)},
 UtilityKeyAction:UtilityKeyAction{get:|v:&Rc<KeyComputeContext>|v.utility_key_action(),set:|v:&Rc<KeyComputeContext>,n|v.set_utility_key_action(n)},
 SpaceBarMode:SpaceBarMode{get:|v:&Rc<KeyComputeContext>|v.space_bar_mode(),set:|v:&Rc<KeyComputeContext>,n|v.set_space_bar_mode(n)},
 SpaceBarLabel:Option<String>{try_get:|v:&Rc<KeyComputeContext>|v.space_bar_label().map(|t|t.to_utf8()).transpose(),set:|v:&Rc<KeyComputeContext>,n|v.set_space_bar_label(text(n))},
 HintMode:KeyHintMode{get:|v:&Rc<KeyComputeContext>|v.hint_mode(),set:|v:&Rc<KeyComputeContext>,n|v.set_hint_mode(n)},
 ShowSymbolHints:bool{get:|v:&Rc<KeyComputeContext>|v.show_symbol_hints(),set:|v:&Rc<KeyComputeContext>,n|v.set_show_symbol_hints(n)},
 ShowNumberHints:bool{get:|v:&Rc<KeyComputeContext>|v.show_number_hints(),set:|v:&Rc<KeyComputeContext>,n|v.set_show_number_hints(n)}
 ],methods:[fn Unshifted()->Rc<KeyComputeContext> =>(|v:&Rc<KeyComputeContext>|Rc::new(v.unshifted()))]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::KeyComputeContextMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(KeyComputeContextMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_reference::<KeyComputeContext>();
        ValueTypes::register_cast::<Rc<KeyComputeContext>, crate::KeyEvaluatorRef>(|v| {
            crate::native_key_evaluator(v.clone())
        });
        ValueTypes::register_boxed_cast::<KeyComputeContext, crate::KeyEvaluatorRef>(|v| {
            let any: Rc<dyn std::any::Any> = v.clone();
            Some(crate::native_key_evaluator(
                any.downcast::<KeyComputeContext>().ok()?,
            ))
        });
        ValueTypes::register_boxed_cast::<crate::KeyEvaluatorRef, Rc<KeyComputeContext>>(|v| {
            v.downcast_ref::<crate::KeyEvaluatorRef>()?.downcast()
        });
    });
}
