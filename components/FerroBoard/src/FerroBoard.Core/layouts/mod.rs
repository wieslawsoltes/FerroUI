mod key_data;
mod resource_dictionary;
mod resource_object_list;
mod resource_read_only_list;
mod resource_text_list;
mod subtype_records;
pub use key_data::*;
pub use resource_dictionary::*;
pub use resource_object_list::*;
pub use resource_read_only_list::*;
pub use resource_text_list::*;
mod layout_models;
pub use layout_models::*;
mod json_element;
mod layout_json;
pub use json_element::{JsonElement, JsonName, JsonProperties, normalize_json_options};
pub use layout_json::*;
mod keyboard_resources;
pub use keyboard_resources::*;
mod computed_keyboard;
pub use computed_keyboard::*;
mod key_labels;
pub use key_labels::*;
mod layout_computer;
pub use layout_computer::*;

mod code_point_array;
pub use code_point_array::CodePointArray;

mod key_evaluator;
pub use key_evaluator::{
    KeyEvaluator, KeyEvaluatorObject, KeyEvaluatorValues, SharedKeyEvaluator, WeakKeyEvaluator,
};

mod float_hash;
mod keyboard_geometry_options;
pub use keyboard_geometry_options::*;

mod key_compute_context;
pub use key_compute_context::*;

mod key_compute_context_record;

mod currency_set;
pub use currency_set::*;

#[cfg(test)]
mod computed_null_context_tests;

mod popup_mapping;
pub use popup_mapping::{PopupMapping, PopupMappingValues};

mod resource_read_only_text_list;
pub use resource_read_only_text_list::ResourceReadOnlyTextList;

mod computed_list;
pub use computed_list::{ComputedItems, ComputedList, ComputedListIterator};

/// Mutable alternatives retained by a computed key and any active popup.
pub type PopupKeys = ComputedList<TextKey>;
