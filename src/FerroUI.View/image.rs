use ferroui_base::{
    media::{imaging::Bitmap, IImage},
    platform::AssetLoader,
    utilities::UriRef,
};
use std::rc::Rc;

/// Decode a project raster embedded with `include_bytes!` as a control source.
/// Invalid or unsupported image data yields an empty image.
pub fn embedded_image(mut bytes: &[u8]) -> Option<Rc<dyn IImage>> {
    decode_image(&mut bytes)
}

/// Load a raster by its absolute address in the application's asset registry.
/// Missing or unsupported image data yields an empty image.
pub fn asset_image(address: &str) -> Option<Rc<dyn IImage>> {
    let uri = UriRef::from(UriRef::parse_absolute(address).ok()?);
    let mut stream = AssetLoader::open(&uri, None).ok()?;
    decode_image(&mut *stream)
}

fn decode_image(stream: &mut dyn std::io::Read) -> Option<Rc<dyn IImage>> {
    Bitmap::from_stream(stream)
        .ok()
        .map(|bitmap| Rc::new(bitmap) as Rc<dyn IImage>)
}
