//! Port of `Converters/FontFamilyTypeConverter.cs`.

use super::type_converter::{service_provider_of, text_of, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::{ServiceProviderExtensions, XamlLoadException};
use ferroui_base::data::core::ValueType;
use ferroui_base::media::FontFamily;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts text to a font family; relative font URIs are resolved against
/// the base URI of the document.
pub struct FontFamilyTypeConverter;

impl TypeConverter for FontFamilyTypeConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let s = text_of(value)?;
        let base_uri = service_provider_of(context).and_then(|sp| sp.get_context_base_uri());

        let font_family = FontFamily::parse_with_base_uri(s, base_uri.as_ref())
            .map_err(|e| XamlLoadException::with_message(e.to_string()))?;
        Ok(Some(Rc::new(font_family)))
    }
}

type_converter_markup!(FontFamilyTypeConverter);
