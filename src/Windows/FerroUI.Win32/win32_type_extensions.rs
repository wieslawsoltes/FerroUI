//! Conversions between the structures of the system and the types of the
//! toolkit.

use crate::interop::unmanaged_methods::RECT;
use ferroui_base::PixelRect;

/// The conversions of the structures of the system.
pub trait Win32TypeExtensions {
    /// The rectangle as a pixel rectangle: position and size.
    fn to_pixel_rect(&self) -> PixelRect;
}

impl Win32TypeExtensions for RECT {
    fn to_pixel_rect(&self) -> PixelRect {
        PixelRect::new(self.left, self.top, self.right - self.left, self.bottom - self.top)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rectangle_of_edges_becomes_a_position_and_a_size() {
        let rect = RECT { left: -10, top: 20, right: 90, bottom: 70 };
        assert_eq!(rect.to_pixel_rect(), PixelRect::new(-10, 20, 100, 50));
        assert_eq!(RECT::from_pixel_rect(rect.to_pixel_rect()), rect);
        assert_eq!((rect.width(), rect.height()), (100, 50));
    }
}
