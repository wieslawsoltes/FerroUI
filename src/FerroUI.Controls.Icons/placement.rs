//! Where an icon draws its image: scaled uniformly to its bounds and centered, as WinUI's
//! `BitmapIcon` and `ImageIcon` do.

use ferroui_base::media::{MediaExtensions, Stretch, StretchDirection};
use ferroui_base::{Rect, Size};

/// The size an image of `source` takes in `available`.
pub(crate) fn measure(available: Size, source: Size) -> Size {
    MediaExtensions::calculate_size(Stretch::Uniform, available, source, StretchDirection::Both)
}

/// The rectangle an image of `source` is drawn in, inside an icon of `bounds`.
pub(crate) fn image_rect(bounds: Size, source: Size) -> Option<Rect> {
    if bounds.width <= 0.0 || bounds.height <= 0.0 || source.width <= 0.0 || source.height <= 0.0 {
        return None;
    }
    let scale = MediaExtensions::calculate_scaling(Stretch::Uniform, bounds, source, StretchDirection::Both);
    let scaled = source * scale;
    Some(Rect::from_size(bounds).center_rect(Rect::from_size(scaled)))
}
