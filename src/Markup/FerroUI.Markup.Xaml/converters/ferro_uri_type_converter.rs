//! Port of `Converters/FerroUriTypeConverter.cs`.

use super::type_converter::type_converter_markup;
use super::{ITypeDescriptorContext, TypeConverter};
use crate::XamlLoadException;
use ferroui_base::data::core::ValueType;
use ferroui_base::utilities::{CultureInfo, Uri, UriKind};
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts text to a URI. A path that starts with `/` is a relative URI,
/// never a file URI.
pub struct FerroUriTypeConverter;

impl TypeConverter for FerroUriTypeConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        _context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let Ok(s) = super::type_converter::text_of(value) else {
            return Ok(None);
        };
        // A path starting with "/" would otherwise be read as a file URI on Unix.
        let kind = if s.starts_with('/') { UriKind::Relative } else { UriKind::RelativeOrAbsolute };
        match Uri::try_create(s, kind) {
            Some(res) => Ok(Some(Rc::new(res))),
            None => Err(XamlLoadException::with_message(format!("Unable to parse URI: {s}"))),
        }
    }
}

type_converter_markup!(FerroUriTypeConverter);
