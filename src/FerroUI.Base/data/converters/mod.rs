//! Value converters.

pub mod composite_format;

mod bool_converters;
mod default_value_converter;
mod func_multi_value_converter;
mod func_value_converter;
mod i_multi_value_converter;
mod i_value_converter;
mod method_to_command_converter;
mod object_converters;
mod string_converters;
mod string_format_multi_value_converter;
mod string_format_value_converter;

#[cfg(test)]
mod composite_format_tests;

pub use bool_converters::BoolConverters;
pub use default_value_converter::DefaultValueConverter;
pub use func_multi_value_converter::FuncMultiValueConverter;
pub use func_value_converter::{cast_value, FuncValueConverter, FuncValueConverterWithParameter};
pub use i_multi_value_converter::IMultiValueConverter;
pub use i_value_converter::IValueConverter;
pub(crate) use method_to_command_converter::method_to_command;
pub use method_to_command_converter::MethodToCommandConverter;
pub use object_converters::ObjectConverters;
pub use string_converters::StringConverters;
pub use string_format_multi_value_converter::StringFormatMultiValueConverter;
pub use string_format_value_converter::StringFormatValueConverter;
