//! Native metadata over original Core objects; no control dependency or duplicate typing state.
pub use ferroboard_core::text::locale_text;
mod settings_metadata;
pub use settings_metadata::KeyboardSettingsMetadata;
mod subtype_list;
pub use subtype_list::SettingsSubtypeList;
mod enums;
pub use enums::*;
use ferroboard_core::settings::KeyboardSettings;
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupAssembly, MarkupType, MarkupTyped},
};
/// Native assembly and namespace migration to FerroBoard.*; original C# stays unchanged.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroBoard.Core",
    crate_name: "ferroboard_core_ferro",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::KeyboardSettingsMetadata,crate::SettingsSubtypeList,crate::LayoutMetadataSnapshot,crate::LayoutAuthorsList,crate::LayoutMetadataMetadata,crate::ComponentNameMetadata,crate::KeyRectMetadata,crate::KeyboardLayoutMetadata,crate::LayoutArrangementMetadata,crate::ArrangementRowsMetadata,crate::ArrangementRowMetadata,crate::AbstractKeyDataMetadata],contracts:[],generics:[],generic_contracts:[]}
pub fn register_types() {
    ferroui_base::register_types();
    utf16_metadata::register();
    locale_text::install_locale_provider(native_current_culture)
        .expect("Core culture provider already configured by a different host");
    MarkupAssembly::register(&ASSEMBLY);
    register_rust_paths();
    MarkupType::register_all(&[
        KeyboardSettingsMetadata::MARKUP,
        SettingsSubtypeList::MARKUP,
    ]);
    ValueTypes::register_global(register_values);
    enums::register();
    input_metadata::register();
    engine_metadata::register();
    layout_metadata::register();
    key_data_metadata::register();
    derived_key_metadata::register();
    code_point_array::register();
    selector_metadata::register();
    evaluator_metadata::register();
    concrete_evaluator_metadata::register();
    key_list_metadata::register();
    text_key_list_metadata::register();
    popup_dictionary_metadata::register();
    popup_mapping_metadata::register();
    currency_set_metadata::register();
    key_rect_metadata::register();
    computed_key_metadata::register();
    computed_collection_metadata::register();
    popup_keys_metadata::register();
    computed_keyboard_metadata::register();
    layout_computer_metadata::register();
    geometry_options_metadata::register();
    compute_context_metadata::register();
    clock_metadata::register();
    key_proximity_metadata::register();
    editor_metadata::register();
    touch_processor_metadata::register();
    candidate_metadata::register();
    request_metadata::register();
    suggestion_list_metadata::register();
    string_list_metadata::register();
    suggestion_provider_metadata::register();
    suggestion_factory_metadata::register();
    subtype_metadata::register();
    dictionary_provider_metadata::register();
}
fn register_values() {
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<std::time::Duration>(),"::std::time::Duration")]);}
    ValueTypes::register_nullable::<std::time::Duration>();
    ferroui_base::__ferro_compiler_metadata! {ferroui_base::metadata::register_type_rust_paths(&[(||std::any::TypeId::of::<KeyboardSettings>(),"::ferroboard_core::settings::KeyboardSettings"),(||std::any::TypeId::of::<std::rc::Rc<KeyboardSettings>>(),"::std::rc::Rc<::ferroboard_core::settings::KeyboardSettings>")]);}

    ValueTypes::register_reference::<KeyboardSettings>();
    ValueTypes::register_nullable::<SettingsSubtypeList>();
}

#[cfg(test)]
mod tests;

mod input_metadata;
mod touch_processor_metadata;
pub use input_metadata::{
    ITextInputTargetMetadata, InputAttributesMetadata, PerformedActionList,
    TextBufferTargetMetadata,
};
pub use touch_processor_metadata::{KeyboardTouchProcessorMetadata, KeyboardTouchProcessorRef};
mod input_delegates;
mod subscriptions;
pub use subscriptions::{
    release_engine_markup_subscriptions, release_resources_markup_subscriptions,
    release_target_markup_subscriptions,
};

mod engine_metadata;
pub use engine_metadata::{
    KeyboardEngineMetadata, KeyboardResourcesMetadata, LanguageModelsMetadata,
};

mod editor_metadata;
pub use editor_metadata::{EditorControllerMetadata, PunctuationRuleMetadata};
#[cfg(test)]
mod input_tests;

fn native_current_culture() -> Result<String, locale_text::CollationError> {
    Ok(ferroui_base::utilities::CultureInfo::current_culture()
        .name()
        .to_string())
}

mod candidate_metadata;
pub use candidate_metadata::SuggestionMetadata;
mod utf16_metadata;
pub use utf16_metadata::Utf16TextMetadata;
#[cfg(test)]
mod candidate_metadata_tests;

mod layout_metadata;
pub use layout_metadata::{
    AbstractKeyDataMetadata, ArrangementRowMetadata, ArrangementRowsMetadata,
    ComponentNameMetadata, KeyboardLayoutMetadata, LayoutArrangementMetadata, LayoutAuthorsList,
    LayoutMetadataMetadata, LayoutMetadataSnapshot,
};

mod key_rect_metadata;
pub use key_rect_metadata::KeyRectMetadata;

mod clock_metadata;
pub use clock_metadata::{ClockRef, EditorClockMetadata};

#[cfg(test)]
mod native_callback_tests;

mod key_proximity_metadata;
pub use key_proximity_metadata::{KeyProximityMetadata, KeyProximityRef};

mod locale_metadata;
mod request_metadata;
pub use request_metadata::SuggestionRequestMetadata;

mod suggestion_list_metadata;
pub use suggestion_list_metadata::{SuggestionReadOnlyList, SuggestionReadOnlyListMetadata};
mod suggestion_provider_metadata;
pub use suggestion_provider_metadata::{
    ISuggestionProviderMetadata, SuggestionProviderRef, native_suggestion_provider,
};
mod suggestion_factory_metadata;
pub use suggestion_factory_metadata::{
    SuggestionProviderFactoryMetadata, SuggestionProviderFactoryRef,
};
mod subtype_metadata;
pub use subtype_metadata::{SubtypeLayoutMapMetadata, SubtypeMetadata};

mod dictionary_provider_metadata;
pub use dictionary_provider_metadata::{
    DictionarySuggestionProviderMetadata, UserDictionaryMetadata, WordDictionaryMetadata,
};

mod string_list_metadata;
pub use string_list_metadata::{
    StringReadOnlyList, StringReadOnlyListMetadata, native_string_list,
};

mod key_data_metadata;
pub use key_data_metadata::{PopupSetMetadata, TextKeyDataMetadata};

mod key_list_metadata;
pub use key_list_metadata::{KeyReadOnlyList, KeyReadOnlyListMetadata};

mod selector_metadata;
pub use selector_metadata::{
    CaseSelectorMetadata, CharWidthSelectorMetadata, KanaSelectorMetadata,
    LayoutDirectionSelectorMetadata, ShiftStateSelectorMetadata, VariationSelectorMetadata,
};

mod evaluator_metadata;
pub use evaluator_metadata::{IKeyEvaluatorMetadata, KeyEvaluatorRef, native_key_evaluator};

mod derived_key_metadata;
pub use derived_key_metadata::{AutoTextKeyDataMetadata, MultiTextKeyDataMetadata};

mod code_point_array;
pub use code_point_array::CodePointArrayMetadata;

mod concrete_evaluator_metadata;
pub use concrete_evaluator_metadata::KeyEvaluatorMetadata;

mod computed_key_metadata;
pub use computed_key_metadata::ComputedKeyMetadata;

mod text_key_list_metadata;
pub use text_key_list_metadata::{TextKeyReadOnlyList, TextKeyReadOnlyListMetadata};

mod text_key_covariance;

mod geometry_options_metadata;
pub use geometry_options_metadata::KeyboardGeometryOptionsMetadata;

mod currency_set_metadata;
pub use currency_set_metadata::CurrencySetMetadata;

mod compute_context_metadata;
pub use compute_context_metadata::KeyComputeContextMetadata;

mod resource_error_metadata;

#[cfg(test)]
mod native_collection_tests;
#[cfg(test)]
mod native_method_tests;
#[cfg(test)]
mod native_popup_lookup_tests;
#[cfg(test)]
mod native_snapshot_tests;
pub mod popup_dictionary_metadata;

mod popup_mapping_metadata;
mod resource_string_list;
pub use popup_mapping_metadata::PopupMappingMetadata;

pub mod computed_collection_metadata;

mod computed_keyboard_metadata;
pub use computed_keyboard_metadata::ComputedKeyboardMetadata;

mod layout_computer_metadata;
pub use layout_computer_metadata::LayoutComputerMetadata;

#[cfg(test)]
#[path = "../../tests/support/fixture_clock.rs"]
mod fixture_clock;

pub mod popup_keys_metadata;
pub use popup_keys_metadata::{PopupKeysMetadata, PopupKeysView};
