//! Port of `Converters/FerroUriTypeConverter.cs`.

use super::type_converter::{convert_to_text, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::XamlLoadError;
use ferroui_base::data::core::ValueType;
use ferroui_base::utilities::{CultureInfo, Uri, UriKind, UriRef};
use ferroui_base::BoxedValue;
use std::rc::Rc;
use url::Url;

/// Converts text to a URI. A path that starts with `/` is a relative URI,
/// never a file URI.
///
/// A URI has three Rust forms, `Uri`, `UriRef` and `url::Url` (C5 of #423): the
/// converter converts text to a `Uri`, and a URI or text to a `UriRef` or a
/// `Url` ([`convert_to`](TypeConverter::convert_to)), where a relative reference
/// is not a `Url` and a text the URL parser rejects is neither (section 3.5 of
/// docs/idiomatic-rust-signatures.md).
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
    ) -> Result<Option<BoxedValue>, XamlLoadError> {
        let Ok(s) = super::type_converter::text_of(value) else {
            return Ok(None);
        };
        // A path starting with "/" would otherwise be read as a file URI on Unix.
        let kind = if s.starts_with('/') { UriKind::Relative } else { UriKind::RelativeOrAbsolute };
        match Uri::try_create(s, kind) {
            Some(res) => Ok(Some(Rc::new(res))),
            None => Err(XamlLoadError::with_message(format!("Unable to parse URI: {s}"))),
        }
    }

    fn can_convert_to(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, destination_type: ValueType) -> bool {
        destination_type.is_string()
            || destination_type.is::<UriRef>()
            || destination_type.is::<Option<UriRef>>()
            || destination_type.is::<Url>()
            || destination_type.is::<Option<Url>>()
    }

    /// Converts a URI (a `Uri`, a `UriRef`, a `Url`, or text that converts to a
    /// `Uri`) to a `UriRef`, a `Url` or one of their nullable forms: the
    /// reference of the same text, and its URL. A relative reference is not a
    /// `Url` and fails the conversion, and so does a text the URL parser
    /// rejects. Every value converts to text.
    fn convert_to(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
        destination_type: ValueType,
    ) -> Result<Option<BoxedValue>, XamlLoadError> {
        let to_url = destination_type.is::<Url>() || destination_type.is::<Option<Url>>();
        let nullable = destination_type.is::<Option<UriRef>>() || destination_type.is::<Option<Url>>();
        if !to_url && !nullable && !destination_type.is::<UriRef>() {
            return convert_to_text(value, destination_type);
        }
        let unable = || {
            XamlLoadError::with_message(format!(
                "'FerroUriTypeConverter' is unable to convert '{}' to '{}'.",
                value.map_or("(null)", |v| (**v).type_name()),
                destination_type
            ))
        };
        let reference = match value {
            None if nullable => {
                return Ok(Some(match to_url {
                    true => Rc::new(Option::<Url>::None) as BoxedValue,
                    false => Rc::new(Option::<UriRef>::None) as BoxedValue,
                }));
            }
            None => return Err(unable()),
            Some(value) => match (value.downcast_ref::<UriRef>(), value.downcast_ref::<Url>()) {
                (Some(reference), _) => reference.clone(),
                (None, Some(url)) => UriRef::Absolute(url.clone()),
                (None, None) => {
                    let uri = match value.downcast_ref::<Uri>() {
                        Some(uri) => uri.clone(),
                        None => self
                            .convert_from(context, culture, Some(value))?
                            .and_then(|converted| converted.downcast_ref::<Uri>().cloned())
                            .ok_or_else(unable)?,
                    };
                    UriRef::try_from(&uri).map_err(|error| {
                        XamlLoadError::with_message(format!("The URI {uri} is not a valid Url: {error}."))
                    })?
                }
            },
        };
        if !to_url {
            return Ok(Some(match nullable {
                true => Rc::new(Some(reference)) as BoxedValue,
                false => Rc::new(reference) as BoxedValue,
            }));
        }
        let url = reference.as_url().cloned().ok_or_else(|| {
            XamlLoadError::with_message(format!("The URI {reference} is relative and cannot be converted to a Url."))
        })?;
        Ok(Some(match nullable {
            true => Rc::new(Some(url)) as BoxedValue,
            false => Rc::new(url) as BoxedValue,
        }))
    }
}

type_converter_markup!(FerroUriTypeConverter);
