//! Registration of the markup types of WinUI rich text: the typography
//! enumerations, the paragraph tab stops and the input scopes
//! (`docs/porting/winui-rich-text.md`).

use crate::data::core::ValueTypes;
use crate::input::InputScopeNameValue;
use crate::media::text_formatting::{TabAlignment, TabLeader, TextTabStops};
use crate::media::{
    FontCapitals, FontEastAsianLanguage, FontEastAsianWidths, FontFraction, FontNumeralAlignment, FontNumeralStyle,
    FontVariants,
};
use crate::metadata::{MarkupType, MarkupTyped};

pub(super) const TYPES: &[&MarkupType] = &[
    <FontCapitals as MarkupTyped>::MARKUP,
    <FontEastAsianLanguage as MarkupTyped>::MARKUP,
    <FontEastAsianWidths as MarkupTyped>::MARKUP,
    <FontFraction as MarkupTyped>::MARKUP,
    <FontNumeralAlignment as MarkupTyped>::MARKUP,
    <FontNumeralStyle as MarkupTyped>::MARKUP,
    <FontVariants as MarkupTyped>::MARKUP,
    <InputScopeNameValue as MarkupTyped>::MARKUP,
    <TabAlignment as MarkupTyped>::MARKUP,
    <TabLeader as MarkupTyped>::MARKUP,
    <TextTabStops as MarkupTyped>::MARKUP,
];

/// Registers every type of this file that registers when it is first used: what the
/// tables of types (`type-tables`) register at start-up.
pub(super) fn register_value_type_table() {
    ValueTypes::register_used::<FontCapitals>();
    ValueTypes::register_used::<FontEastAsianLanguage>();
    ValueTypes::register_used::<FontEastAsianWidths>();
    ValueTypes::register_used::<FontFraction>();
    ValueTypes::register_used::<FontNumeralAlignment>();
    ValueTypes::register_used::<FontNumeralStyle>();
    ValueTypes::register_used::<FontVariants>();
    ValueTypes::register_used::<InputScopeNameValue>();
    ValueTypes::register_used::<TabAlignment>();
    ValueTypes::register_used::<TabLeader>();
    ValueTypes::register_used::<TextTabStops>();
}

impl crate::data::core::ValueTypeRegistration for FontCapitals {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<FontCapitals>();
    }
}

impl crate::data::core::ValueTypeRegistration for FontEastAsianLanguage {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<FontEastAsianLanguage>();
    }
}

impl crate::data::core::ValueTypeRegistration for FontEastAsianWidths {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<FontEastAsianWidths>();
    }
}

impl crate::data::core::ValueTypeRegistration for FontFraction {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<FontFraction>();
    }
}

impl crate::data::core::ValueTypeRegistration for FontNumeralAlignment {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<FontNumeralAlignment>();
    }
}

impl crate::data::core::ValueTypeRegistration for FontNumeralStyle {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<FontNumeralStyle>();
    }
}

impl crate::data::core::ValueTypeRegistration for FontVariants {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<FontVariants>();
    }
}

impl crate::data::core::ValueTypeRegistration for InputScopeNameValue {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<InputScopeNameValue>();
    }
}

impl crate::data::core::ValueTypeRegistration for TabAlignment {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<TabAlignment>();
    }
}

impl crate::data::core::ValueTypeRegistration for TabLeader {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<TabLeader>();
    }
}

impl crate::data::core::ValueTypeRegistration for TextTabStops {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<TextTabStops>();
    }
}

