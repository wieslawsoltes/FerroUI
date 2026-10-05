use crate::Size;

/// Provides the new size with a raw resize notification.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawSizeEventArgs {
    size: Size,
}

impl RawSizeEventArgs {
    /// Creates args for a size.
    pub fn new(size: Size) -> Self {
        Self { size }
    }

    /// Creates args for a width and a height.
    pub fn from_width_height(width: f64, height: f64) -> Self {
        Self { size: Size::new(width, height) }
    }

    /// The new size.
    #[inline]
    pub fn size(&self) -> Size {
        self.size
    }
}
