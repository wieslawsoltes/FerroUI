mod ascii_casing_data;
mod utf16_text;
pub use utf16_text::*;
mod casing_data;
mod text_culture;
mod text_utils;
pub use text_culture::*;
pub use text_utils::*;
mod composer_data;
mod composers;
pub use composers::*;
mod grapheme;
mod lowercase_data;
mod normalization;
mod normalization_data;
mod text_properties;
pub use normalization::NormalizationError;
mod ordinal;
mod ordinal_data;
pub use ordinal::*;
mod word_dictionary;
pub use word_dictionary::*;
mod user_dictionary;
pub use user_dictionary::*;
mod suggestion_request;
pub use suggestion_request::*;
mod suggestion;
pub use suggestion::*;
mod suggestion_list;
pub use suggestion_list::*;
mod suggestions;
pub use suggestions::*;

pub(crate) fn is_letter_unit(c: u16) -> bool {
    text_properties::properties(c as u32).0 & 8 != 0
}

mod lower_categories;
pub(crate) use lower_categories::is_lower_code_point;

pub mod locale_text;

mod prediction_list;
pub use prediction_list::*;
