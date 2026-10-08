use crate::color_palettes::IColorPalette;
use crate::color_palettes::FlatColor;
use ferroui_base::media::Color;
use ferroui_base::utilities::MathUtilities;
use std::sync::OnceLock;

static COLOR_CHART: OnceLock<[[Color; 5]; 10]> = OnceLock::new();

/// Implements half of the [`FlatColorPalette`] for improved usability.
///
/// See [`FlatColorPalette`].
#[derive(Debug, Default)]
pub struct FlatHalfColorPalette;

impl FlatHalfColorPalette {
    /// Initializes a new instance of the [`FlatHalfColorPalette`] class.
    pub fn new() -> Self {
        Self
    }

    /// Initializes all color chart colors.
    pub fn init_color_chart(&self) {
        COLOR_CHART.get_or_init(|| {
            [
                // Pomegranate
                [
                    Color::from_uint32(FlatColor::Pomegranate1 as u32),
                    Color::from_uint32(FlatColor::Pomegranate3 as u32),
                    Color::from_uint32(FlatColor::Pomegranate5 as u32),
                    Color::from_uint32(FlatColor::Pomegranate7 as u32),
                    Color::from_uint32(FlatColor::Pomegranate9 as u32),
                ],

                // Amethyst
                [
                    Color::from_uint32(FlatColor::Amethyst1 as u32),
                    Color::from_uint32(FlatColor::Amethyst3 as u32),
                    Color::from_uint32(FlatColor::Amethyst5 as u32),
                    Color::from_uint32(FlatColor::Amethyst7 as u32),
                    Color::from_uint32(FlatColor::Amethyst9 as u32),
                ],

                // Belize Hole
                [
                    Color::from_uint32(FlatColor::BelizeHole1 as u32),
                    Color::from_uint32(FlatColor::BelizeHole3 as u32),
                    Color::from_uint32(FlatColor::BelizeHole5 as u32),
                    Color::from_uint32(FlatColor::BelizeHole7 as u32),
                    Color::from_uint32(FlatColor::BelizeHole9 as u32),
                ],

                // Turquoise
                [
                    Color::from_uint32(FlatColor::Turquoise1 as u32),
                    Color::from_uint32(FlatColor::Turquoise3 as u32),
                    Color::from_uint32(FlatColor::Turquoise5 as u32),
                    Color::from_uint32(FlatColor::Turquoise7 as u32),
                    Color::from_uint32(FlatColor::Turquoise9 as u32),
                ],

                // Nephritis
                [
                    Color::from_uint32(FlatColor::Nephritis1 as u32),
                    Color::from_uint32(FlatColor::Nephritis3 as u32),
                    Color::from_uint32(FlatColor::Nephritis5 as u32),
                    Color::from_uint32(FlatColor::Nephritis7 as u32),
                    Color::from_uint32(FlatColor::Nephritis9 as u32),
                ],

                // Sunflower
                [
                    Color::from_uint32(FlatColor::Sunflower1 as u32),
                    Color::from_uint32(FlatColor::Sunflower3 as u32),
                    Color::from_uint32(FlatColor::Sunflower5 as u32),
                    Color::from_uint32(FlatColor::Sunflower7 as u32),
                    Color::from_uint32(FlatColor::Sunflower9 as u32),
                ],

                // Carrot
                [
                    Color::from_uint32(FlatColor::Carrot1 as u32),
                    Color::from_uint32(FlatColor::Carrot3 as u32),
                    Color::from_uint32(FlatColor::Carrot5 as u32),
                    Color::from_uint32(FlatColor::Carrot7 as u32),
                    Color::from_uint32(FlatColor::Carrot9 as u32),
                ],

                // Clouds
                [
                    Color::from_uint32(FlatColor::Clouds1 as u32),
                    Color::from_uint32(FlatColor::Clouds3 as u32),
                    Color::from_uint32(FlatColor::Clouds5 as u32),
                    Color::from_uint32(FlatColor::Clouds7 as u32),
                    Color::from_uint32(FlatColor::Clouds9 as u32),
                ],

                // Concrete
                [
                    Color::from_uint32(FlatColor::Concrete1 as u32),
                    Color::from_uint32(FlatColor::Concrete3 as u32),
                    Color::from_uint32(FlatColor::Concrete5 as u32),
                    Color::from_uint32(FlatColor::Concrete7 as u32),
                    Color::from_uint32(FlatColor::Concrete9 as u32),
                ],

                // Wet Asphalt
                [
                    Color::from_uint32(FlatColor::WetAsphalt1 as u32),
                    Color::from_uint32(FlatColor::WetAsphalt3 as u32),
                    Color::from_uint32(FlatColor::WetAsphalt5 as u32),
                    Color::from_uint32(FlatColor::WetAsphalt7 as u32),
                    Color::from_uint32(FlatColor::WetAsphalt9 as u32),
                ],
            ]
        });
    }
}

impl IColorPalette for FlatHalfColorPalette {
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
