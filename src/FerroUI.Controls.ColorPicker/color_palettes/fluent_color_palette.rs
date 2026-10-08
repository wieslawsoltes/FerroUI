use crate::color_palettes::IColorPalette;
use ferroui_base::media::Color;
use ferroui_base::utilities::MathUtilities;

// Values were taken from the Settings App, Personalization > Colors which match with
// https://docs.microsoft.com/en-us/windows/uwp/whats-new/windows-docs-december-2017
//
// The default ordering and grouping of colors was undesirable so was modified.
// Colors were transposed: the colors in rows within the Settings app became columns here.
// This is because columns in an IColorPalette generally should contain different shades of
// the same color. In the settings app this concept is somewhat loosely reversed.
// The first 'column' ordering, after being transposed, was then reversed so 'red' colors
// were near to each other.
//
// This new ordering most closely follows the Windows standard while:
//
//  1. Keeping colors in a 'spectrum' order
//  2. Keeping like colors next to each both in rows and columns
//     (which is unique for the windows palette).
//     For example, similar red colors are next to each other in both
//     rows within the same column and rows within the column next to it.
//     This follows a 'snake-like' pattern as illustrated below.
//  3. A downside of this ordering is colors don't follow strict 'shades'
//     as in other palettes.
//
// The colors will be displayed in the below pattern.
// This pattern follows a spectrum while keeping like-colors near to one
// another across both rows and columns.
//
//      ┌Red───┐      ┌Blue──┐      ┌Gray──┐
//      │      │      │      │      │      |
//      │      │      │      │      │      |
// Yellow      └Violet┘      └Green─┘      Brown
static COLOR_CHART: [[Color; 8]; 6] = [
    [
        // Ordering reversed for this section only
        Color::from_argb(255, 255, 67, 67), // #FF4343
        Color::from_argb(255, 209, 52, 56), // #D13438
        Color::from_argb(255, 239, 105, 80), // #EF6950
        Color::from_argb(255, 218, 59, 1), // #DA3B01
        Color::from_argb(255, 202, 80, 16), // #CA5010
        Color::from_argb(255, 247, 99, 12), // #F7630C
        Color::from_argb(255, 255, 140, 0), // #FF8C00
        Color::from_argb(255, 255, 185, 0), // #FFB900
    ],
    [
        Color::from_argb(255, 231, 72, 86), // #E74856
        Color::from_argb(255, 232, 17, 35), // #E81123
        Color::from_argb(255, 234, 0, 94), // #EA005E
        Color::from_argb(255, 195, 0, 82), // #C30052
        Color::from_argb(255, 227, 0, 140), // #E3008C
        Color::from_argb(255, 191, 0, 119), // #BF0077
        Color::from_argb(255, 194, 57, 179), // #C239B3
        Color::from_argb(255, 154, 0, 137), // #9A0089
    ],
    [
        Color::from_argb(255, 0, 120, 215), // #0078D7
        Color::from_argb(255, 0, 99, 177), // #0063B1
        Color::from_argb(255, 142, 140, 216), // #8E8CD8
        Color::from_argb(255, 107, 105, 214), // #6B69D6
        Color::from_argb(255, 135, 100, 184), // #8764B8
        Color::from_argb(255, 116, 77, 169), // #744DA9
        Color::from_argb(255, 177, 70, 194), // #B146C2
        Color::from_argb(255, 136, 23, 152), // #881798
    ],
    [
        Color::from_argb(255, 0, 153, 188), // #0099BC
        Color::from_argb(255, 45, 125, 154), // #2D7D9A
        Color::from_argb(255, 0, 183, 195), // #00B7C3
        Color::from_argb(255, 3, 131, 135), // #038387
        Color::from_argb(255, 0, 178, 148), // #00B294
        Color::from_argb(255, 1, 133, 116), // #018574
        Color::from_argb(255, 0, 204, 106), // #00CC6A
        Color::from_argb(255, 16, 137, 62), // #10893E
    ],
    [
        Color::from_argb(255, 122, 117, 116), // #7A7574
        Color::from_argb(255, 93, 90, 80), // #5D5A58
        Color::from_argb(255, 104, 118, 138), // #68768A
        Color::from_argb(255, 81, 92, 107), // #515C6B
        Color::from_argb(255, 86, 124, 115), // #567C73
        Color::from_argb(255, 72, 104, 96), // #486860
        Color::from_argb(255, 73, 130, 5), // #498205
        Color::from_argb(255, 16, 124, 16), // #107C10
    ],
    [
        Color::from_argb(255, 118, 118, 118), // #767676
        Color::from_argb(255, 76, 74, 72), // #4C4A48
        Color::from_argb(255, 105, 121, 126), // #69797E
        Color::from_argb(255, 74, 84, 89), // #4A5459
        Color::from_argb(255, 100, 124, 100), // #647C64
        Color::from_argb(255, 82, 94, 84), // #525E54
        Color::from_argb(255, 132, 117, 69), // #847545
        Color::from_argb(255, 126, 115, 95), // #7E735F
    ],
];

/// Implements the standard Windows 10 color palette.
#[derive(Debug, Default)]
pub struct FluentColorPalette;

impl FluentColorPalette {
    /// Initializes a new instance of the [`FluentColorPalette`] class.
    pub fn new() -> Self {
        Self
    }
}

impl IColorPalette for FluentColorPalette {
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
