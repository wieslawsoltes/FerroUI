//! Native evaluator construction, values and interface projection.
use ferroboard_core::{
    keys::{KeyVariation, KeyboardMode, LayoutDirection, ShiftState},
    layouts::{IKeyEvaluator, KeyEvaluator, KeyEvaluatorObject},
    text::UnsupportedCulture,
};
use ferroui_base::{
    BoxedValue,
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
#[allow(clippy::too_many_arguments)]
fn create(
    mode: KeyboardMode,
    shift: ShiftState,
    variation: KeyVariation,
    direction: LayoutDirection,
    half: bool,
    kana: bool,
    culture: Option<String>,
) -> Result<KeyEvaluator, UnsupportedCulture> {
    Ok(KeyEvaluator::new(
        mode,
        shift,
        variation,
        direction,
        half,
        kana,
        crate::locale_metadata::parse(culture)?,
    ))
}
fn set_culture(value: &KeyEvaluator, name: Option<String>) -> Result<(), UnsupportedCulture> {
    value.set_culture_override(crate::locale_metadata::parse(name)?);
    Ok(())
}
#[doc(hidden)]
pub struct KeyEvaluatorMetadata;
ferroui_base::ferro_markup_type!(class KeyEvaluatorMetadata as "KeyEvaluator" {
 namespace:"FerroBoard.Layouts",this:KeyEvaluator,handles:[KeyEvaluator,Option<KeyEvaluator>,KeyEvaluatorObject,Rc<KeyEvaluatorObject>],interfaces:[crate::KeyEvaluatorRef],
 constructors:[()=>KeyEvaluator::default,try (Mode:KeyboardMode,ShiftState:ShiftState,Variation:KeyVariation,Direction:LayoutDirection,IsCharHalfWidth:bool,IsKanaKata:bool,CultureOverride:Option<String>)=>create],
 properties:[
 Mode:KeyboardMode{get:KeyEvaluator::mode,set:KeyEvaluator::set_mode},
 ShiftState:ShiftState{get:KeyEvaluator::shift_state,set:KeyEvaluator::set_shift_state},
 Variation:KeyVariation{get:KeyEvaluator::variation,set:KeyEvaluator::set_variation},
 Direction:LayoutDirection{get:KeyEvaluator::direction,set:KeyEvaluator::set_direction},
 IsCharHalfWidth:bool{get:KeyEvaluator::is_char_half_width,set:KeyEvaluator::set_is_char_half_width},
 IsKanaKata:bool{get:KeyEvaluator::is_kana_kata,set:KeyEvaluator::set_is_kana_kata},
 CultureOverride:Option<String>{get:|e:&KeyEvaluator|crate::locale_metadata::name(e.culture_override()),try_set:set_culture},
 Culture:Option<String>{get:|e:&KeyEvaluator|crate::locale_metadata::name(e.culture())}
 ],static_properties:[Default:KeyEvaluator{get:KeyEvaluator::default_reference}]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::KeyEvaluatorMetadata],contracts:[],generics:[],generic_contracts:[]}
fn original(value: &BoxedValue) -> Option<KeyEvaluator> {
    let any: Rc<dyn std::any::Any> = value.clone();
    Some(KeyEvaluator::from_original(
        any.downcast::<KeyEvaluatorObject>().ok()?,
    ))
}
fn interface(value: &KeyEvaluator) -> crate::KeyEvaluatorRef {
    crate::native_key_evaluator(value.original_object())
}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register(KeyEvaluatorMetadata::MARKUP);
    ValueTypes::register_global(|| {
        ValueTypes::register_reference::<KeyEvaluatorObject>();
        ValueTypes::register_nullable::<KeyEvaluator>();
        ValueTypes::register_cast::<Rc<KeyEvaluatorObject>, KeyEvaluator>(|v| {
            KeyEvaluator::from_original(v.clone())
        });
        ValueTypes::register_boxed_cast::<KeyEvaluatorObject, KeyEvaluator>(original);
        ValueTypes::register_boxed_cast::<KeyEvaluatorObject, Option<KeyEvaluator>>(|v| {
            Some(Some(original(v)?))
        });
        ValueTypes::register_cast::<KeyEvaluator, crate::KeyEvaluatorRef>(interface);
        ValueTypes::register_boxed_cast::<KeyEvaluatorObject, crate::KeyEvaluatorRef>(|v| {
            Some(interface(&original(v)?))
        });
        ValueTypes::register_boxed_cast::<crate::KeyEvaluatorRef, KeyEvaluator>(|v| {
            Some(KeyEvaluator::from_original(
                v.downcast_ref::<crate::KeyEvaluatorRef>()?.downcast()?,
            ))
        });
        ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<KeyEvaluator>(),"::ferroboard_core::layouts::KeyEvaluator")]);}
    });
}
