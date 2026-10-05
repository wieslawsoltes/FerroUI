use crate::skia_sharp_extensions::to_sk_color_type_or_panic;
use ferroui_base::platform::PixelFormat;
use skia_safe::ColorType;

/// Resolves the Skia color type for a pixel format, falling back to the
/// platform's native 32-bit color type when no format is given.
pub fn resolve_color_type(format: Option<PixelFormat>) -> ColorType {
    format.map(to_sk_color_type_or_panic).unwrap_or(ColorType::N32)
}
