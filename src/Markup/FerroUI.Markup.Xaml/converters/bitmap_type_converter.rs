//! Port of `Converters/BitmapTypeConverter.cs`.

use super::type_converter::{service_provider_of, text_of, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::{ServiceProviderExtensions, XamlLoadException};
use ferroui_base::data::core::ValueType;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::IImage;
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::{CultureInfo, Uri, UriExtensions, UriKind};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// Converts text to a bitmap: the path of a file or the URI of an asset,
/// which may be relative to the base URI of the document.
pub struct BitmapTypeConverter;

/// `s.StartsWith("/") ? new Uri(s, Relative) : new Uri(s, RelativeOrAbsolute)`.
pub(crate) fn asset_uri(s: &str) -> Result<Uri, XamlLoadException> {
    let kind = if s.starts_with('/') { UriKind::Relative } else { UriKind::RelativeOrAbsolute };
    Uri::new(s, kind).map_err(|e| XamlLoadException::with_inner(e.to_string(), e))
}

/// The path of an absolute file URI, `None` for every other URI.
pub(crate) fn local_file_path(uri: &Uri) -> Option<String> {
    (uri.is_absolute_uri() && uri.scheme() == "file").then(|| UriExtensions::get_unescape_absolute_path(uri))
}

fn io_error(e: std::io::Error) -> XamlLoadException {
    XamlLoadException::with_inner(e.to_string(), e)
}

impl BitmapTypeConverter {
    pub(crate) fn load(context: Option<&Rc<dyn ITypeDescriptorContext>>, s: &str) -> Result<Bitmap, XamlLoadException> {
        let uri = asset_uri(s)?;

        if let Some(path) = local_file_path(&uri) {
            return Bitmap::from_file(&path).map_err(io_error);
        }

        let assets = FerroLocator::current().get_required_service::<dyn IAssetLoader>();
        let base_uri = service_provider_of(context).and_then(|sp| sp.get_context_base_uri());
        let mut stream = assets.open(&uri, base_uri.as_ref()).map_err(io_error)?;
        Bitmap::from_stream(&mut *stream).map_err(io_error)
    }
}

impl TypeConverter for BitmapTypeConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    /// The bitmap as an image handle (`Rc<dyn IImage>`), the form image
    /// properties hold.
    fn convert_from(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        let bitmap: Rc<dyn IImage> = Rc::new(Self::load(context, text_of(value)?)?);
        Ok(Some(Rc::new(bitmap)))
    }
}

type_converter_markup!(BitmapTypeConverter);
