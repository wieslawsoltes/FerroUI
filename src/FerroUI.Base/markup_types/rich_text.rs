//! Registration of rich paragraph formatting and input-scope markup types.

use crate::data::core::ValueTypes;
use crate::input::InputScopeNameValue;
use crate::media::text_formatting::{TabAlignment, TabLeader, TextTabStops};
use crate::media::{FontCapitals, FontEastAsianLanguage, FontEastAsianWidths, FontFraction, FontNumeralAlignment, FontNumeralStyle, FontVariants};
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

pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<FontCapitals>();
    ValueTypes::register_nullable::<FontEastAsianLanguage>();
    ValueTypes::register_nullable::<FontEastAsianWidths>();
    ValueTypes::register_nullable::<FontFraction>();
    ValueTypes::register_nullable::<FontNumeralAlignment>();
    ValueTypes::register_nullable::<FontNumeralStyle>();
    ValueTypes::register_nullable::<FontVariants>();

    ValueTypes::register_nullable::<InputScopeNameValue>();
    ValueTypes::register_nullable::<TabAlignment>();
    ValueTypes::register_nullable::<TabLeader>();
    ValueTypes::register_nullable::<TextTabStops>();
}
