use crate::color_palettes::IColorPalette;
use crate::color_palettes::MaterialColor;
use ferroui_base::media::Color;
use ferroui_base::utilities::MathUtilities;
use std::sync::OnceLock;

static COLOR_CHART: OnceLock<[[Color; 5]; 10]> = OnceLock::new();

/// Implements half of the [`MaterialColorPalette`] for improved usability.
///
/// See [`MaterialColorPalette`].
#[derive(Debug, Default)]
pub struct MaterialHalfColorPalette;

impl MaterialHalfColorPalette {
    /// Initializes a new instance of the [`MaterialHalfColorPalette`] class.
    pub fn new() -> Self {
        Self
    }

    /// Initializes all color chart colors.
    pub fn init_color_chart(&self) {
        COLOR_CHART.get_or_init(|| {
            [
                // Red
                [
                    Color::from_uint32(MaterialColor::Red50 as u32),
                    Color::from_uint32(MaterialColor::Red200 as u32),
                    Color::from_uint32(MaterialColor::Red400 as u32),
                    Color::from_uint32(MaterialColor::Red600 as u32),
                    Color::from_uint32(MaterialColor::Red800 as u32),
                ],

                // Purple
                [
                    Color::from_uint32(MaterialColor::Purple50 as u32),
                    Color::from_uint32(MaterialColor::Purple200 as u32),
                    Color::from_uint32(MaterialColor::Purple400 as u32),
                    Color::from_uint32(MaterialColor::Purple600 as u32),
                    Color::from_uint32(MaterialColor::Purple800 as u32),
                ],

                // Indigo
                [
                    Color::from_uint32(MaterialColor::Indigo50 as u32),
                    Color::from_uint32(MaterialColor::Indigo200 as u32),
                    Color::from_uint32(MaterialColor::Indigo400 as u32),
                    Color::from_uint32(MaterialColor::Indigo600 as u32),
                    Color::from_uint32(MaterialColor::Indigo800 as u32),
                ],

                // Light Blue
                [
                    Color::from_uint32(MaterialColor::LightBlue50 as u32),
                    Color::from_uint32(MaterialColor::LightBlue200 as u32),
                    Color::from_uint32(MaterialColor::LightBlue400 as u32),
                    Color::from_uint32(MaterialColor::LightBlue600 as u32),
                    Color::from_uint32(MaterialColor::LightBlue800 as u32),
                ],

                // Teal
                [
                    Color::from_uint32(MaterialColor::Teal50 as u32),
                    Color::from_uint32(MaterialColor::Teal200 as u32),
                    Color::from_uint32(MaterialColor::Teal400 as u32),
                    Color::from_uint32(MaterialColor::Teal600 as u32),
                    Color::from_uint32(MaterialColor::Teal800 as u32),
                ],

                // Light Green
                [
                    Color::from_uint32(MaterialColor::LightGreen50 as u32),
                    Color::from_uint32(MaterialColor::LightGreen200 as u32),
                    Color::from_uint32(MaterialColor::LightGreen400 as u32),
                    Color::from_uint32(MaterialColor::LightGreen600 as u32),
                    Color::from_uint32(MaterialColor::LightGreen800 as u32),
                ],

                // Yellow
                [
                    Color::from_uint32(MaterialColor::Yellow50 as u32),
                    Color::from_uint32(MaterialColor::Yellow200 as u32),
                    Color::from_uint32(MaterialColor::Yellow400 as u32),
                    Color::from_uint32(MaterialColor::Yellow600 as u32),
                    Color::from_uint32(MaterialColor::Yellow800 as u32),
                ],

                // Orange
                [
                    Color::from_uint32(MaterialColor::Orange50 as u32),
                    Color::from_uint32(MaterialColor::Orange200 as u32),
                    Color::from_uint32(MaterialColor::Orange400 as u32),
                    Color::from_uint32(MaterialColor::Orange600 as u32),
                    Color::from_uint32(MaterialColor::Orange800 as u32),
                ],

                // Brown
                [
                    Color::from_uint32(MaterialColor::Brown50 as u32),
                    Color::from_uint32(MaterialColor::Brown200 as u32),
                    Color::from_uint32(MaterialColor::Brown400 as u32),
                    Color::from_uint32(MaterialColor::Brown600 as u32),
                    Color::from_uint32(MaterialColor::Brown800 as u32),
                ],

                // Blue Gray
                [
                    Color::from_uint32(MaterialColor::BlueGray50 as u32),
                    Color::from_uint32(MaterialColor::BlueGray200 as u32),
                    Color::from_uint32(MaterialColor::BlueGray400 as u32),
                    Color::from_uint32(MaterialColor::BlueGray600 as u32),
                    Color::from_uint32(MaterialColor::BlueGray800 as u32),
                ],
            ]
        });
    }
}

impl IColorPalette for MaterialHalfColorPalette {
    fn color_count(&self) -> i32 {
        10
    }

    fn shade_count(&self) -> i32 {
        5
    }

    fn get_color(&self, color_index: i32, shade_index: i32) -> Color {
        if COLOR_CHART.get().is_none() {
            self.init_color_chart();
        }

        let color_chart = COLOR_CHART.get().expect("the color chart is initialized");
        color_chart[MathUtilities::clamp_i32(color_index, 0, self.color_count() - 1) as usize]
            [MathUtilities::clamp_i32(shade_index, 0, self.shade_count() - 1) as usize]
    }
}
