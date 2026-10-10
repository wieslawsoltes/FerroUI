use ferroui_base::media::Color;

/// A colour of the Windows Runtime (`Windows.UI.Color`): four bytes, alpha
/// first.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct WinRTColor {
    #[allow(missing_docs)]
    pub a: u8,
    #[allow(missing_docs)]
    pub r: u8,
    #[allow(missing_docs)]
    pub g: u8,
    #[allow(missing_docs)]
    pub b: u8,
}

impl WinRTColor {
    /// A colour from its alpha, red, green and blue.
    pub fn from_argb(a: u8, r: u8, g: u8, b: u8) -> WinRTColor {
        WinRTColor { a, r, g, b }
    }

    /// The colour as a colour of the toolkit.
    pub fn to_ferro(self) -> Color {
        Color::new(self.a, self.r, self.g, self.b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_is_four_bytes_alpha_first() {
        assert_eq!(std::mem::size_of::<WinRTColor>(), 4);
        assert_eq!(std::mem::align_of::<WinRTColor>(), 1);
        let color = WinRTColor::from_argb(1, 2, 3, 4);
        // SAFETY: four bytes without padding.
        let bytes: [u8; 4] = unsafe { std::mem::transmute(color) };
        assert_eq!(bytes, [1, 2, 3, 4]);
    }

    #[test]
    fn the_colour_of_the_toolkit_has_the_same_channels() {
        assert_eq!(WinRTColor::from_argb(255, 0, 120, 215).to_ferro(), Color::new(255, 0, 120, 215));
    }
}
