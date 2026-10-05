//! Port of `Converters/IconTypeConverter.cs`.

use super::bitmap_type_converter::{asset_uri, local_file_path};
use super::type_converter::{service_provider_of, type_converter_markup};
use super::{ITypeDescriptorContext, TypeConverter};
use crate::{ServiceProviderExtensions, XamlLoadException};
use ferroui_base::data::core::ValueType;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::IImage;
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions};
use ferroui_controls::WindowIcon;
use std::rc::Rc;

/// Converts text (the path of a file or the URI of an asset) or a bitmap to
/// a window icon.
pub struct IconTypeConverter;

fn io_error(e: std::io::Error) -> XamlLoadException {
    XamlLoadException::with_inner(e.to_string(), e)
}

impl IconTypeConverter {
    fn create_icon_from_path(
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        s: &str,
    ) -> Result<Rc<WindowIcon>, XamlLoadException> {
        let uri = asset_uri(s)?;

        if let Some(path) = local_file_path(&uri) {
            return WindowIcon::from_file(&path).map_err(io_error);
        }
        let assets = FerroLocator::current().get_required_service::<dyn IAssetLoader>();
        let base_uri = service_provider_of(context).and_then(|sp| sp.get_context_base_uri());
        let mut stream = assets.open(&uri, base_uri.as_ref()).map_err(io_error)?;
        WindowIcon::from_stream(&mut *stream).map_err(io_error)
    }
}

impl TypeConverter for IconTypeConverter {
    fn can_convert_from(&self, _context: Option<&Rc<dyn ITypeDescriptorContext>>, source_type: ValueType) -> bool {
        source_type.is_string()
    }

    fn convert_from(
        &self,
        context: Option<&Rc<dyn ITypeDescriptorContext>>,
        _culture: Option<&CultureInfo>,
        value: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, XamlLoadException> {
        if let Ok(path) = super::type_converter::text_of(value) {
            return Ok(Some(Rc::new(Self::create_icon_from_path(context, path)?)));
        }

        let image = value.and_then(|v| v.downcast_ref::<Rc<dyn IImage>>());
        if let Some(bitmap) = image.and_then(|image| image.as_any().downcast_ref::<Bitmap>()) {
            return Ok(Some(Rc::new(WindowIcon::from_bitmap(bitmap))));
        }

        Err(XamlLoadException::with_message("Specified method is not supported."))
    }
}

type_converter_markup!(IconTypeConverter);
