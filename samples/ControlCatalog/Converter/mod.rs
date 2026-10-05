//! The value converters of the catalog (namespace `ControlCatalog.Converter`): one module per upstream file.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use std::rc::Rc;

mod deg_to_rad_converter;
mod flex_demo_number_to_thickness_converter;
mod gdp_to_brush_converter;
mod hex_converter;
mod math_subtract_converter;
mod table_view_column_width_converter;

pub use deg_to_rad_converter::DegToRadConverter;
pub use flex_demo_number_to_thickness_converter::FlexDemoNumberToThicknessConverter;
pub use gdp_to_brush_converter::GdpToBrushConverter;
pub use hex_converter::HexConverter;
pub use math_subtract_converter::MathSubtractConverter;
pub use table_view_column_width_converter::TableViewColumnWidthConverter;

/// The types of this namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <DegToRadConverter as MarkupTyped>::MARKUP,
    <FlexDemoNumberToThicknessConverter as MarkupTyped>::MARKUP,
    <GdpToBrushConverter as MarkupTyped>::MARKUP,
    <HexConverter as MarkupTyped>::MARKUP,
    <MathSubtractConverter as MarkupTyped>::MARKUP,
    <TableViewColumnWidthConverter as MarkupTyped>::MARKUP,
];

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<DegToRadConverter>();
    ValueTypes::register_cast::<DegToRadConverter, Rc<dyn IValueConverter>>(DegToRadConverter::as_value_converter);
    ValueTypes::register_reference::<FlexDemoNumberToThicknessConverter>();
    ValueTypes::register_cast::<FlexDemoNumberToThicknessConverter, Rc<dyn IValueConverter>>(FlexDemoNumberToThicknessConverter::as_value_converter);
    ValueTypes::register_reference::<GdpToBrushConverter>();
    ValueTypes::register_cast::<GdpToBrushConverter, Rc<dyn IValueConverter>>(GdpToBrushConverter::as_value_converter);
    ValueTypes::register_reference::<HexConverter>();
    ValueTypes::register_cast::<HexConverter, Rc<dyn IValueConverter>>(HexConverter::as_value_converter);
    ValueTypes::register_reference::<MathSubtractConverter>();
    ValueTypes::register_cast::<MathSubtractConverter, Rc<dyn IValueConverter>>(MathSubtractConverter::as_value_converter);
    ValueTypes::register_reference::<TableViewColumnWidthConverter>();
    ValueTypes::register_cast::<TableViewColumnWidthConverter, Rc<dyn IValueConverter>>(TableViewColumnWidthConverter::as_value_converter);
}
