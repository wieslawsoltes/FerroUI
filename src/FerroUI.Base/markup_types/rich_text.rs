//! Registration of the markup types of WinUI rich text: the typography
//! enumerations, the paragraph tab stops and the input scopes
//! (`docs/porting/winui-rich-text.md`).

use crate::data::core::{NullableRegistration, ValueTypes};
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

pub(super) fn register_value_types() {
    ValueTypes::register_nullable_types(&const { [
        NullableRegistration::of::<FontCapitals>(),
        NullableRegistration::of::<FontEastAsianLanguage>(),
        NullableRegistration::of::<FontEastAsianWidths>(),
        NullableRegistration::of::<FontFraction>(),
        NullableRegistration::of::<FontNumeralAlignment>(),
        NullableRegistration::of::<FontNumeralStyle>(),
        NullableRegistration::of::<FontVariants>(),
        NullableRegistration::of::<InputScopeNameValue>(),
        NullableRegistration::of::<TabAlignment>(),
        NullableRegistration::of::<TabLeader>(),
        NullableRegistration::of::<TextTabStops>(),
    ] });
}
