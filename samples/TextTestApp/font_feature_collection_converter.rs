//! Port of `FontFeatureCollectionConverter.cs`: the type converter that converts the text of
//! the features box to a font feature collection.
//!
//! The managed original attaches the converter to the collection type at run time, in its
//! entry point, so that the binding of the features box to the `FontFeatures` property of the
//! line control converts its text. The framework has no run-time registration of a type
//! converter for a type of another crate (GAPS.md, T002): the collection type states its
//! conversion from text itself (`parse:` of its markup metadata), which is the conversion
//! this converter performs, and the bindings use it.

use ferroui_base::data::core::ValueType;
use ferroui_base::media::FontFeatureCollection;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{AnyValue, BoxedValue};
use ferroui_markup_xaml::converters::{ITypeDescriptorContext, TypeConverter};
use ferroui_markup_xaml::XamlLoadException;
use std::rc::Rc;

pub struct FontFeatureCollectionConverter;

impl TypeConverter for FontFeatureCollectionConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        _context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        // `(string)value`: a value that is not text is an invalid cast.
        let text = value.and_then(|value| {
            let value: &dyn AnyValue = &**value;
            value.downcast_ref::<String>().map(String::as_str).or_else(|| value.downcast_ref::<&'static str>().copied())
        });
        let Some(text) = text else {
            return Err(XamlLoadException::with_message(format!(
                "Unable to cast object of type '{}' to type 'String'.",
                value.map_or("null", |value| (**value).type_name())
            )));
        };

        let font_features =
            FontFeatureCollection::parse(text).map_err(|error| XamlLoadException::with_message(error.to_string()))?;
        Ok(Some(Rc::new(font_features)))
    }
}
