//! Value converters used by the control themes.
//!
//! The casts of a converter to the converter interfaces (`IValueConverter`,
//! `IMultiValueConverter`), which compiled markup needs before it sets a converter, are registered
//! by the construction of the first converter of its type ([`register_casts!`]): every
//! converter of this module is made through a constructor that registers them. A converter an
//! application never makes keeps neither its cast nor the code of its conversion (#612).

/// Registers the value conversions of converter `$type` (its `ValueTypeRegistration`, which
/// holds its casts to the converter interfaces) on every thread, the first time a converter of
/// the type is made; later calls read one flag.
macro_rules! register_casts {
    ($type:ty) => {{
        static CASTS: ::std::sync::Once = ::std::sync::Once::new();
        CASTS.call_once(::ferroui_base::data::core::ValueTypes::register_used::<$type>);
    }};
}

mod border_gap_mask_converter;
mod corner_radius_filter_converter;
mod corner_radius_to_double_converter;
mod corners;
mod enum_to_bool_converter;
mod margin_multiplier_converter;
mod menu_scrolling_visibility_converter;
mod platform_key_gesture_converter;
mod string_format_converter;
mod tree_view_item_indent_converter;

pub use border_gap_mask_converter::BorderGapMaskConverter;
pub use corner_radius_filter_converter::CornerRadiusFilterConverter;
pub use corner_radius_to_double_converter::CornerRadiusToDoubleConverter;
pub use corners::Corners;
pub use enum_to_bool_converter::EnumToBoolConverter;
pub use margin_multiplier_converter::MarginMultiplierConverter;
pub use menu_scrolling_visibility_converter::MenuScrollingVisibilityConverter;
pub use platform_key_gesture_converter::PlatformKeyGestureConverter;
pub use string_format_converter::StringFormatConverter;
pub use tree_view_item_indent_converter::TreeViewItemIndentConverter;

#[cfg(test)]
mod converters_tests;
