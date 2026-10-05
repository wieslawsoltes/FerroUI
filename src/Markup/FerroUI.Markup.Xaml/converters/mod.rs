//! The type converters of the markup runtime: conversions from the text of
//! a markup document that need its context (base URI, parent stack, type
//! resolver), and the color to brush conversion of resources.

mod bitmap_type_converter;
mod color_to_brush_converter;
mod ferro_property_type_converter;
mod ferro_uri_type_converter;
mod font_family_type_converter;
mod icon_type_converter;
mod points_list_type_converter;
mod time_span_type_converter;
mod type_converter;

pub use bitmap_type_converter::BitmapTypeConverter;
pub use color_to_brush_converter::ColorToBrushConverter;
pub use ferro_property_type_converter::FerroPropertyTypeConverter;
pub use ferro_uri_type_converter::FerroUriTypeConverter;
pub use font_family_type_converter::FontFamilyTypeConverter;
pub use icon_type_converter::IconTypeConverter;
pub use points_list_type_converter::PointsListTypeConverter;
pub use time_span_type_converter::TimeSpanTypeConverter;
pub use type_converter::{ITypeDescriptorContext, ServiceProviderTypeDescriptorContext, TypeConverter};

#[cfg(test)]
mod converters_tests;
