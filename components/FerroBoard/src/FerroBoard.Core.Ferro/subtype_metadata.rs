//! Native configuration metadata over the actual shared subtype allocations.
use ferroboard_core::layouts::{ComponentName, LayoutType, Subtype, SubtypeLayoutMap};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::sync::Arc;

#[doc(hidden)]
pub struct SubtypeLayoutMapMetadata;
#[doc(hidden)]
pub struct SubtypeMetadata;

ferroui_base::ferro_markup_type!(class SubtypeLayoutMapMetadata as "SubtypeLayoutMap" {
 namespace:"FerroBoard.Layouts",this:Arc<SubtypeLayoutMap>,handles:[Arc<SubtypeLayoutMap>,Option<Arc<SubtypeLayoutMap>>],
 constructors:[()=>(||Arc::new(SubtypeLayoutMap::default()))],
 fields:[CoreLayouts:String=>(||SubtypeLayoutMap::CORE_LAYOUTS.to_owned())],
 properties:[
 Characters:ComponentName{get:|v:&Arc<SubtypeLayoutMap>|v.characters(),set:|v:&Arc<SubtypeLayoutMap>,n|v.set_characters(n)},
 Symbols:ComponentName{get:|v:&Arc<SubtypeLayoutMap>|v.symbols(),set:|v:&Arc<SubtypeLayoutMap>,n|v.set_symbols(n)},
 Symbols2:ComponentName{get:|v:&Arc<SubtypeLayoutMap>|v.symbols2(),set:|v:&Arc<SubtypeLayoutMap>,n|v.set_symbols2(n)},
 Numeric:ComponentName{get:|v:&Arc<SubtypeLayoutMap>|v.numeric(),set:|v:&Arc<SubtypeLayoutMap>,n|v.set_numeric(n)},
 NumericAdvanced:ComponentName{get:|v:&Arc<SubtypeLayoutMap>|v.numeric_advanced(),set:|v:&Arc<SubtypeLayoutMap>,n|v.set_numeric_advanced(n)},
 NumericRow:ComponentName{get:|v:&Arc<SubtypeLayoutMap>|v.numeric_row(),set:|v:&Arc<SubtypeLayoutMap>,n|v.set_numeric_row(n)},
 Phone:ComponentName{get:|v:&Arc<SubtypeLayoutMap>|v.phone(),set:|v:&Arc<SubtypeLayoutMap>,n|v.set_phone(n)},
 Phone2:ComponentName{get:|v:&Arc<SubtypeLayoutMap>|v.phone2(),set:|v:&Arc<SubtypeLayoutMap>,n|v.set_phone2(n)}
 ],methods:[try fn Get(LayoutType)->ComponentName=>(|v:&Arc<SubtypeLayoutMap>,kind|v.get(kind))]
});
ferroui_base::ferro_markup_type!(class SubtypeMetadata as "Subtype" {
 namespace:"FerroBoard.Layouts",this:Arc<Subtype>,handles:[Arc<Subtype>,Option<Arc<Subtype>>],
 constructors:[()=>(||Arc::new(Subtype::default()))],
 fields:[CoreComposers:String=>(||Subtype::CORE_COMPOSERS.to_owned()),CoreCurrencySets:String=>(||Subtype::CORE_CURRENCY_SETS.to_owned()),CoreLocalization:String=>(||Subtype::CORE_LOCALIZATION.to_owned())],
 static_properties:[Default:Arc<Subtype>{get:Subtype::shared_default}],
 properties:[
 LanguageTag:Option<String>{try_get:|v:&Arc<Subtype>|crate::utf16_metadata::to_native_optional(v.language_tag()),set:|v:&Arc<Subtype>,n|v.set_language_tag(crate::utf16_metadata::from_native_optional(n))},
 DisplayName:Option<String>{try_get:|v:&Arc<Subtype>|crate::utf16_metadata::to_native_optional(v.display_name()),set:|v:&Arc<Subtype>,n|v.set_display_name(crate::utf16_metadata::from_native_optional(n))},
 Composer:ComponentName{get:|v:&Arc<Subtype>|v.composer(),set:|v:&Arc<Subtype>,n|v.set_composer(n)},
 CurrencySet:ComponentName{get:|v:&Arc<Subtype>|v.currency_set(),set:|v:&Arc<Subtype>,n|v.set_currency_set(n)},
 PopupMapping:ComponentName{get:|v:&Arc<Subtype>|v.popup_mapping(),set:|v:&Arc<Subtype>,n|v.set_popup_mapping(n)},
 PunctuationRule:ComponentName{get:|v:&Arc<Subtype>|v.punctuation_rule(),set:|v:&Arc<Subtype>,n|v.set_punctuation_rule(n)},
 Layouts:Option<Arc<SubtypeLayoutMap>>{get:|v:&Arc<Subtype>|v.layouts(),set:|v:&Arc<Subtype>,n|v.set_layouts(n)},
 Id:String{try_get:|v:&Arc<Subtype>|v.try_id().map_err(|e|e.to_string()).and_then(crate::utf16_metadata::to_native)},
 ShortLabel:String{try_get:|v:&Arc<Subtype>|v.try_short_label().map_err(|e|e.to_string()).and_then(crate::utf16_metadata::to_native)}
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::SubtypeLayoutMapMetadata,crate::SubtypeMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[SubtypeLayoutMapMetadata::MARKUP, SubtypeMetadata::MARKUP]);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<Arc<SubtypeLayoutMap>>();
        ValueTypes::register_nullable::<Arc<Subtype>>();
    });
}
