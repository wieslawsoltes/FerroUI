//! Markup metadata of the enumerations of this crate.

use crate::{TeachingTipCloseReason, TeachingTipPlacementMode, TeachingTipTailVisibility};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_markup_enum, ferro_markup_type};

ferro_markup_enum!(TeachingTipCloseReason { CloseButton, LightDismiss, Programmatic }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(
    TeachingTipPlacementMode {
        Auto, Top, Bottom, Left, Right, TopRight, TopLeft, BottomRight, BottomLeft, LeftTop, LeftBottom, RightTop,
        RightBottom,
    },
    { namespace: "FerroUI.Controls" }
);
ferro_markup_enum!(TeachingTipTailVisibility { Auto, Visible, Collapsed }, { namespace: "FerroUI.Controls" });

fn high_contrast_key() -> ThemeVariant {
    ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()))
}

/// The keys the theme documents of this crate name. `HighContrast` compares equal to the high
/// contrast variant of the WinUI theme, because variants compare by key: the documents select
/// its dictionaries without this crate depending on that theme.
pub struct TeachingTipThemeKeys;

ferro_markup_type!(static TeachingTipThemeKeys {
    namespace: "FerroUI.Controls",
    fields: [
        HighContrast: ThemeVariant => high_contrast_key,
    ],
});

const TYPES: &[&MarkupType] = &[
    <TeachingTipCloseReason as MarkupTyped>::MARKUP,
    <TeachingTipPlacementMode as MarkupTyped>::MARKUP,
    <TeachingTipTailVisibility as MarkupTyped>::MARKUP,
    <TeachingTipThemeKeys as MarkupTyped>::MARKUP,
];

pub(crate) fn register() {
    MarkupType::register_all(TYPES);
    ValueTypes::register_used::<TeachingTipPlacementMode>();
    ValueTypes::register_used::<TeachingTipTailVisibility>();
}

impl ferroui_base::data::core::ValueTypeRegistration for TeachingTipPlacementMode {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<TeachingTipPlacementMode>();
    }
}

impl ferroui_base::data::core::ValueTypeRegistration for TeachingTipTailVisibility {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<TeachingTipTailVisibility>();
    }
}
