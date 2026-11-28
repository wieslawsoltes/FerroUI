//! The bitmap of `BitmapIcon.UriSource`.

use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::{Uri, UriRef};
use std::io;
use std::rc::Rc;

/// Loads the bitmap of `uri`: a `file:` URI from its path, any other through the asset loader. A
/// URI that cannot be loaded is logged and shows nothing, as a failed image source in WinUI.
pub(crate) fn load(uri: &Uri) -> Option<Rc<Bitmap>> {
    match open(uri) {
        Ok(bitmap) => Some(Rc::new(bitmap)),
        Err(error) => {
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
                logger.log(None, &format!("BitmapIcon: cannot load {}: {error}", uri.original_string()));
            }
            None
        }
    }
}

fn open(uri: &Uri) -> io::Result<Bitmap> {
    if uri.is_absolute_uri() && uri.scheme() == "file" {
        return Bitmap::from_file(&uri.local_path());
    }
    let reference = UriRef::try_from(uri).map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let mut stream = AssetLoader::open(&reference, None)?;
    Bitmap::from_stream(&mut *stream)
}
