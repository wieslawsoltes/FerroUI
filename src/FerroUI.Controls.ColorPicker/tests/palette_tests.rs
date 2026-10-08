//! Not from upstream: the colour palettes.

use crate::*;
use ferroui_base::media::{Color, Colors};

#[test]
fn palettes_have_the_dimensions_of_the_original() {
    let palettes: [(&dyn IColorPalette, i32, i32); 6] = [
        (&FlatColorPalette::new(), 20, 10),
        (&FlatHalfColorPalette::new(), 10, 5),
        (&FluentColorPalette::new(), 6, 8),
        (&MaterialColorPalette::new(), 19, 10),
        (&MaterialHalfColorPalette::new(), 10, 5),
        (&SixteenColorPalette::new(), 8, 2),
    ];

    for (palette, colors, shades) in palettes {
        assert_eq!(colors, palette.color_count());
        assert_eq!(shades, palette.shade_count());
    }
}

#[test]
fn palette_indices_are_clamped() {
    let palette = SixteenColorPalette::new();

    assert_eq!(Colors::WHITE, palette.get_color(0, 0));
    assert_eq!(Colors::PURPLE, palette.get_color(7, 1));
    assert_eq!(Colors::PURPLE, palette.get_color(100, 100));
    assert_eq!(Colors::WHITE, palette.get_color(-1, -1));
}

#[test]
fn flat_and_material_palettes_take_their_colors_from_the_enumerations() {
    assert_eq!(Color::from_uint32(FlatColor::Pomegranate1 as u32), FlatColorPalette::new().get_color(0, 0));
    assert_eq!(Color::from_uint32(FlatColor::MidnightBlue10 as u32), FlatColorPalette::new().get_color(19, 9));
    assert_eq!(Color::from_uint32(FlatColor::Amethyst3 as u32), FlatHalfColorPalette::new().get_color(1, 1));
    assert_eq!(FlatColor::Pomegranate6, FlatColor::Pomegranate);
    assert_eq!(Color::from_uint32(MaterialColor::Red50 as u32), MaterialColorPalette::new().get_color(0, 0));
    assert_eq!(Color::from_uint32(MaterialColor::Red800 as u32), MaterialHalfColorPalette::new().get_color(0, 4));
    assert_eq!(Color::from_argb(255, 255, 67, 67), FluentColorPalette::new().get_color(0, 0));
}
