//! Markup metadata of the enumeration of this crate.

use crate::ItemsViewSelectionMode;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_markup_enum, ferro_markup_type};
use ferroui_base::metadata::{MarkupType, MarkupTyped};

ferro_markup_enum!(ItemsViewSelectionMode { None, Single, Multiple, Extended }, { namespace: "FerroUI.Controls" });

pub(crate) fn high_contrast_key() -> ThemeVariant {
    ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()))
}

/// The keys the theme documents of this crate name. `HighContrast` compares equal to the high
/// contrast variant of the WinUI theme, because variants compare by key: the documents select
/// its dictionaries without this crate depending on that theme.
pub struct ItemsViewThemeKeys;

ferro_markup_type!(static ItemsViewThemeKeys {
    namespace: "FerroUI.Controls",
    fields: [
        HighContrast: ThemeVariant => high_contrast_key,
    ],
});

const TYPES: &[&MarkupType] = &[
    <ItemsViewSelectionMode as MarkupTyped>::MARKUP,
    <ItemsViewThemeKeys as MarkupTyped>::MARKUP,
];

impl ferroui_base::data::core::ValueTypeRegistration for ItemsViewSelectionMode {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<ItemsViewSelectionMode>();
    }
}

fn register_value_types() {
    ValueTypes::register_used::<ItemsViewSelectionMode>();
}

pub(crate) fn register() {
    MarkupType::register_table(TYPES);
    ValueTypes::register_global(register_value_types);
}
