use ferroui_base::media::{imaging::Bitmap, IImage};
use std::rc::Rc;

/// Decode a project raster embedded with `include_bytes!` as a control source.
/// Invalid or unsupported image data yields an empty image.
pub fn embedded_image(mut bytes: &[u8]) -> Option<Rc<dyn IImage>> {
    Bitmap::from_stream(&mut bytes)
        .ok()
        .map(|bitmap| Rc::new(bitmap) as Rc<dyn IImage>)
}
