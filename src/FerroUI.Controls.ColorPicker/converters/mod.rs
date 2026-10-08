//! The value converters of the colour picker (`Converters/` of the upstream
//! project): namespace `FerroUI.Controls.Converters`, and
//! `FerroUI.Controls.Primitives.Converters` for the two that the primitives
//! use ([`AccentColorConverter`](crate::primitives::converters::AccentColorConverter),
//! [`ContrastBrushConverter`](crate::primitives::converters::ContrastBrushConverter)).

pub(crate) mod accent_color_converter;
mod color_to_display_name_converter;
mod color_to_hex_converter;
pub(crate) mod contrast_brush_converter;
mod do_nothing_for_null_converter;
mod to_brush_converter;
mod to_color_converter;

pub use color_to_display_name_converter::ColorToDisplayNameConverter;
pub use color_to_hex_converter::ColorToHexConverter;
pub use do_nothing_for_null_converter::DoNothingForNullConverter;
pub use to_brush_converter::ToBrushConverter;
pub use to_color_converter::ToColorConverter;

use ferroui_base::data::converters::cast_value;
use ferroui_base::media::{IBrush, SolidColorBrush};
use ferroui_base::{BoxedValue, Ref};
use std::rc::Rc;

/// `value is SolidColorBrush`: the mutable solid color brush a value holds.
pub(crate) fn as_solid_color_brush(value: Option<&BoxedValue>) -> Option<Ref<SolidColorBrush>> {
    as_brush(value)?.as_object()?.to_ref().cast::<SolidColorBrush>()
}

/// `value is IBrush`: the brush a value holds.
pub(crate) fn as_brush(value: Option<&BoxedValue>) -> Option<Rc<dyn IBrush>> {
    cast_value::<Rc<dyn IBrush>>(value)
}

/// `new SolidColorBrush(color)` as the untyped result of a converter.
pub(crate) fn solid_color_brush(color: ferroui_base::media::Color) -> BoxedValue {
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(color).into();
    Rc::new(brush)
}

#[cfg(test)]
mod converters_tests;
