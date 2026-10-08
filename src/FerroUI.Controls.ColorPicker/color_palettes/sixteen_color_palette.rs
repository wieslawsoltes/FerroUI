use crate::color_palettes::IColorPalette;
use ferroui_base::media::{Color, Colors};
use ferroui_base::utilities::MathUtilities;

// The 16 standard colors from HTML and early Windows computers
// https://en.wikipedia.org/wiki/List_of_software_palettes
// https://en.wikipedia.org/wiki/Web_colors#HTML_color_names
static COLOR_CHART: [[Color; 2]; 8] = [
    [
        Colors::WHITE,
        Colors::SILVER,
    ],
    [
        Colors::GRAY,
        Colors::BLACK,
    ],
    [
        Colors::RED,
        Colors::MAROON,
    ],
    [
        Colors::YELLOW,
        Colors::OLIVE,
    ],
    [
        Colors::LIME,
        Colors::GREEN,
    ],
    [
        Colors::AQUA,
        Colors::TEAL,
    ],
    [
        Colors::BLUE,
        Colors::NAVY,
    ],
    [
        Colors::FUCHSIA,
        Colors::PURPLE,
    ],
];

/// Implements the standard sixteen color palette from the HTML 4.01 specification.
///
/// See https://en.wikipedia.org/wiki/Web_colors#HTML_color_names.
#[derive(Debug, Default)]
pub struct SixteenColorPalette;

impl SixteenColorPalette {
    /// Initializes a new instance of the [`SixteenColorPalette`] class.
    pub fn new() -> Self {
        Self
    }
}

impl IColorPalette for SixteenColorPalette {
    fn color_count(&self) -> i32 {
        COLOR_CHART.len() as i32
    }

    fn shade_count(&self) -> i32 {
        COLOR_CHART[0].len() as i32
    }

    fn get_color(&self, color_index: i32, shade_index: i32) -> Color {
        COLOR_CHART[MathUtilities::clamp_i32(color_index, 0, COLOR_CHART.len() as i32 - 1) as usize]
            [MathUtilities::clamp_i32(shade_index, 0, COLOR_CHART[0].len() as i32 - 1) as usize]
    }
}
