//! Port of `Converters/BitmapTypeConverter.cs`.

use super::type_converter::{service_provider_of, text_of, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::{ServiceProviderExtensions, XamlLoadError};
use ferroui_base::data::core::ValueType;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::{CultureInfo, UriExtensions, UriRef};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// Converts text to a bitmap: the path of a file or the URI of an asset,
/// which may be relative to the base URI of the document.
pub struct BitmapTypeConverter;

/// `s.StartsWith("/") ? new Uri(s, Relative) : new Uri(s, RelativeOrAbsolute)`: the
/// rule of [`UriRef::parse`].
pub(crate) fn asset_uri(s: &str) -> Result<UriRef, XamlLoadError> {
    UriRef::parse(s).map_err(|e| XamlLoadError::with_inner(e.to_string(), e))
}

/// The path of an absolute file URL, `None` for every other reference.
pub(crate) fn local_file_path(uri: &UriRef) -> Option<String> {
    uri.as_url().filter(|url| url.scheme() == "file").map(UriExtensions::get_unescape_absolute_path)
}

fn io_error(e: std::io::Error) -> XamlLoadError {
    XamlLoadError::with_inner(e.to_string(), e)
}

impl BitmapTypeConverter {
    pub(crate) fn load(context: Option<&Rc<dyn ITypeDescriptorContext>>, s: &str) -> Result<Bitmap, XamlLoadError> {
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

    /// The bitmap as its own handle (`Rc<Bitmap>`), which casts to the image handle
    /// (`Rc<dyn IImage>`) image properties hold and to the source of an image brush
    /// (`Rc<dyn IImageBrushSource>`), as the bitmap class of the managed original
    /// implements both.
    fn convert_from(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadError> {
        let bitmap: Rc<Bitmap> = Rc::new(Self::load(context, text_of(value)?)?);
        Ok(Some(Rc::new(bitmap)))
    }
}

type_converter_markup!(BitmapTypeConverter);
