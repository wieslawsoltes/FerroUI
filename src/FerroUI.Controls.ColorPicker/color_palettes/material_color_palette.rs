use crate::color_palettes::IColorPalette;
use ferroui_base::media::Color;
use ferroui_base::utilities::MathUtilities;
use std::sync::OnceLock;

static COLOR_CHART: OnceLock<[[Color; 10]; 19]> = OnceLock::new();

/// Implements a reduced version of the 2014 Material Design color palette.
///
/// This palette is based on the one outlined here:
///
///   https://material.io/design/color/the-color-system.html#tools-for-picking-colors
///
/// In order to make the palette uniform and rectangular the following
/// alterations were made:
///
///  1. The A100-A700 shades of each color are excluded.
///     These shades do not exist for all colors (brown/gray).
///  2. Black/White are stand-alone and are also excluded.
#[derive(Debug, Default)]
pub struct MaterialColorPalette;

/// Defines all colors in the [`MaterialColorPalette`].
///
/// This is done in an enum to ensure it is compiled into the assembly improving
/// startup performance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum MaterialColor {
    // Red
    Red50 = 0xFFFFEBEE,
    Red100 = 0xFFFFCDD2,
    Red200 = 0xFFEF9A9A,
    Red300 = 0xFFE57373,
    Red400 = 0xFFEF5350,
    Red500 = 0xFFF44336,
    Red600 = 0xFFE53935,
    Red700 = 0xFFD32F2F,
    Red800 = 0xFFC62828,
    Red900 = 0xFFB71C1C,

    RedA100 = 0xFFFF8A80,
    RedA200 = 0xFFFF5252,
    RedA400 = 0xFFFF1744,
    RedA700 = 0xFFD50000,

    // Pink
    Pink50 = 0xFFFCE4EC,
    Pink100 = 0xFFF8BBD0,
    Pink200 = 0xFFF48FB1,
    Pink300 = 0xFFF06292,
    Pink400 = 0xFFEC407A,
    Pink500 = 0xFFE91E63,
    Pink600 = 0xFFD81B60,
    Pink700 = 0xFFC2185B,
    Pink800 = 0xFFAD1457,
    Pink900 = 0xFF880E4F,

    PinkA100 = 0xFFFF80AB,
    PinkA200 = 0xFFFF4081,
    PinkA400 = 0xFFF50057,
    PinkA700 = 0xFFC51162,

    // Purple
    Purple50 = 0xFFF3E5F5,
    Purple100 = 0xFFE1BEE7,
    Purple200 = 0xFFCE93D8,
    Purple300 = 0xFFBA68C8,
    Purple400 = 0xFFAB47BC,
    Purple500 = 0xFF9C27B0,
    Purple600 = 0xFF8E24AA,
    Purple700 = 0xFF7B1FA2,
    Purple800 = 0xFF6A1B9A,
    Purple900 = 0xFF4A148C,

    PurpleA100 = 0xFFEA80FC,
    PurpleA200 = 0xFFE040FB,
    PurpleA400 = 0xFFD500F9,
    PurpleA700 = 0xFFAA00FF,

    // Deep Purple
    DeepPurple50 = 0xFFEDE7F6,
    DeepPurple100 = 0xFFD1C4E9,
    DeepPurple200 = 0xFFB39DDB,
    DeepPurple300 = 0xFF9575CD,
    DeepPurple400 = 0xFF7E57C2,
    DeepPurple500 = 0xFF673AB7,
    DeepPurple600 = 0xFF5E35B1,
    DeepPurple700 = 0xFF512DA8,
    DeepPurple800 = 0xFF4527A0,
    DeepPurple900 = 0xFF311B92,

    DeepPurpleA100 = 0xFFB388FF,
    DeepPurpleA200 = 0xFF7C4DFF,
    DeepPurpleA400 = 0xFF651FFF,
    DeepPurpleA700 = 0xFF6200EA,

    // Indigo
    Indigo50 = 0xFFE8EAF6,
    Indigo100 = 0xFFC5CAE9,
    Indigo200 = 0xFF9FA8DA,
    Indigo300 = 0xFF7986CB,
    Indigo400 = 0xFF5C6BC0,
    Indigo500 = 0xFF3F51B5,
    Indigo600 = 0xFF3949AB,
    Indigo700 = 0xFF303F9F,
    Indigo800 = 0xFF283593,
    Indigo900 = 0xFF1A237E,

    IndigoA100 = 0xFF8C9EFF,
    IndigoA200 = 0xFF536DFE,
    IndigoA400 = 0xFF3D5AFE,
    IndigoA700 = 0xFF304FFE,

    // Blue
    Blue50 = 0xFFE3F2FD,
    Blue100 = 0xFFBBDEFB,
    Blue200 = 0xFF90CAF9,
    Blue300 = 0xFF64B5F6,
    Blue400 = 0xFF42A5F5,
    Blue500 = 0xFF2196F3,
    Blue600 = 0xFF1E88E5,
    Blue700 = 0xFF1976D2,
    Blue800 = 0xFF1565C0,
    Blue900 = 0xFF0D47A1,

    BlueA100 = 0xFF82B1FF,
    BlueA200 = 0xFF448AFF,
    BlueA400 = 0xFF2979FF,
    BlueA700 = 0xFF2962FF,

    // Light Blue
    LightBlue50 = 0xFFE1F5FE,
    LightBlue100 = 0xFFB3E5FC,
    LightBlue200 = 0xFF81D4FA,
    LightBlue300 = 0xFF4FC3F7,
    LightBlue400 = 0xFF29B6F6,
    LightBlue500 = 0xFF03A9F4,
    LightBlue600 = 0xFF039BE5,
    LightBlue700 = 0xFF0288D1,
    LightBlue800 = 0xFF0277BD,
    LightBlue900 = 0xFF01579B,

    LightBlueA100 = 0xFF80D8FF,
    LightBlueA200 = 0xFF40C4FF,
    LightBlueA400 = 0xFF00B0FF,
    LightBlueA700 = 0xFF0091EA,

    // Cyan
    Cyan50 = 0xFFE0F7FA,
    Cyan100 = 0xFFB2EBF2,
    Cyan200 = 0xFF80DEEA,
    Cyan300 = 0xFF4DD0E1,
    Cyan400 = 0xFF26C6DA,
    Cyan500 = 0xFF00BCD4,
    Cyan600 = 0xFF00ACC1,
    Cyan700 = 0xFF0097A7,
    Cyan800 = 0xFF00838F,
    Cyan900 = 0xFF006064,

    CyanA100 = 0xFF84FFFF,
    CyanA200 = 0xFF18FFFF,
    CyanA400 = 0xFF00E5FF,
    CyanA700 = 0xFF00B8D4,

    // Teal
    Teal50 = 0xFFE0F2F1,
    Teal100 = 0xFFB2DFDB,
    Teal200 = 0xFF80CBC4,
    Teal300 = 0xFF4DB6AC,
    Teal400 = 0xFF26A69A,
    Teal500 = 0xFF009688,
    Teal600 = 0xFF00897B,
    Teal700 = 0xFF00796B,
    Teal800 = 0xFF00695C,
    Teal900 = 0xFF004D40,

    TealA100 = 0xFFA7FFEB,
    TealA200 = 0xFF64FFDA,
    TealA400 = 0xFF1DE9B6,
    TealA700 = 0xFF00BFA5,

    // Green
    Green50 = 0xFFE8F5E9,
    Green100 = 0xFFC8E6C9,
    Green200 = 0xFFA5D6A7,
    Green300 = 0xFF81C784,
    Green400 = 0xFF66BB6A,
    Green500 = 0xFF4CAF50,
    Green600 = 0xFF43A047,
    Green700 = 0xFF388E3C,
    Green800 = 0xFF2E7D32,
    Green900 = 0xFF1B5E20,

    GreenA100 = 0xFFB9F6CA,
    GreenA200 = 0xFF69F0AE,
    GreenA400 = 0xFF00E676,
    GreenA700 = 0xFF00C853,

    // Light Green
    LightGreen50 = 0xFFF1F8E9,
    LightGreen100 = 0xFFDCEDC8,
    LightGreen200 = 0xFFC5E1A5,
    LightGreen300 = 0xFFAED581,
    LightGreen400 = 0xFF9CCC65,
    LightGreen500 = 0xFF8BC34A,
    LightGreen600 = 0xFF7CB342,
    LightGreen700 = 0xFF689F38,
    LightGreen800 = 0xFF558B2F,
    LightGreen900 = 0xFF33691E,

    LightGreenA100 = 0xFFCCFF90,
    LightGreenA200 = 0xFFB2FF59,
    LightGreenA400 = 0xFF76FF03,
    LightGreenA700 = 0xFF64DD17,

    // Lime
    Lime50 = 0xFFF9FBE7,
    Lime100 = 0xFFF0F4C3,
    Lime200 = 0xFFE6EE9C,
    Lime300 = 0xFFDCE775,
    Lime400 = 0xFFD4E157,
    Lime500 = 0xFFCDDC39,
    Lime600 = 0xFFC0CA33,
    Lime700 = 0xFFAFB42B,
    Lime800 = 0xFF9E9D24,
    Lime900 = 0xFF827717,

    LimeA100 = 0xFFF4FF81,
    LimeA200 = 0xFFEEFF41,
    LimeA400 = 0xFFC6FF00,
    LimeA700 = 0xFFAEEA00,

    // Yellow
    Yellow50 = 0xFFFFFDE7,
    Yellow100 = 0xFFFFF9C4,
    Yellow200 = 0xFFFFF59D,
    Yellow300 = 0xFFFFF176,
    Yellow400 = 0xFFFFEE58,
    Yellow500 = 0xFFFFEB3B,
    Yellow600 = 0xFFFDD835,
    Yellow700 = 0xFFFBC02D,
    Yellow800 = 0xFFF9A825,
    Yellow900 = 0xFFF57F17,

    YellowA100 = 0xFFFFFF8D,
    YellowA200 = 0xFFFFFF00,
    YellowA400 = 0xFFFFEA00,
    YellowA700 = 0xFFFFD600,

    // Amber
    Amber50 = 0xFFFFF8E1,
    Amber100 = 0xFFFFECB3,
    Amber200 = 0xFFFFE082,
    Amber300 = 0xFFFFD54F,
    Amber400 = 0xFFFFCA28,
    Amber500 = 0xFFFFC107,
    Amber600 = 0xFFFFB300,
    Amber700 = 0xFFFFA000,
    Amber800 = 0xFFFF8F00,
    Amber900 = 0xFFFF6F00,

    AmberA100 = 0xFFFFE57F,
    AmberA200 = 0xFFFFD740,
    AmberA400 = 0xFFFFC400,
    AmberA700 = 0xFFFFAB00,

    // Orange
    Orange50 = 0xFFFFF3E0,
    Orange100 = 0xFFFFE0B2,
    Orange200 = 0xFFFFCC80,
    Orange300 = 0xFFFFB74D,
    Orange400 = 0xFFFFA726,
    Orange500 = 0xFFFF9800,
    Orange600 = 0xFFFB8C00,
    Orange700 = 0xFFF57C00,
    Orange800 = 0xFFEF6C00,
    Orange900 = 0xFFE65100,

    OrangeA100 = 0xFFFFD180,
    OrangeA200 = 0xFFFFAB40,
    OrangeA400 = 0xFFFF9100,
    OrangeA700 = 0xFFFF6D00,

    // Deep Orange
    DeepOrange50 = 0xFFFBE9E7,
    DeepOrange100 = 0xFFFFCCBC,
    DeepOrange200 = 0xFFFFAB91,
    DeepOrange300 = 0xFFFF8A65,
    DeepOrange400 = 0xFFFF7043,
    DeepOrange500 = 0xFFFF5722,
    DeepOrange600 = 0xFFF4511E,
    DeepOrange700 = 0xFFE64A19,
    DeepOrange800 = 0xFFD84315,
    DeepOrange900 = 0xFFBF360C,

    DeepOrangeA100 = 0xFFFF9E80,
    DeepOrangeA200 = 0xFFFF6E40,
    DeepOrangeA400 = 0xFFFF3D00,
    DeepOrangeA700 = 0xFFDD2C00,

    // Brown
    Brown50 = 0xFFEFEBE9,
    Brown100 = 0xFFD7CCC8,
    Brown200 = 0xFFBCAAA4,
    Brown300 = 0xFFA1887F,
    Brown400 = 0xFF8D6E63,
    Brown500 = 0xFF795548,
    Brown600 = 0xFF6D4C41,
    Brown700 = 0xFF5D4037,
    Brown800 = 0xFF4E342E,
    Brown900 = 0xFF3E2723,

    // Gray
    Gray50 = 0xFFFAFAFA,
    Gray100 = 0xFFF5F5F5,
    Gray200 = 0xFFEEEEEE,
    Gray300 = 0xFFE0E0E0,
    Gray400 = 0xFFBDBDBD,
    Gray500 = 0xFF9E9E9E,
    Gray600 = 0xFF757575,
    Gray700 = 0xFF616161,
    Gray800 = 0xFF424242,
    Gray900 = 0xFF212121,

    // Blue Gray
    BlueGray50 = 0xFFECEFF1,
    BlueGray100 = 0xFFCFD8DC,
    BlueGray200 = 0xFFB0BEC5,
    BlueGray300 = 0xFF90A4AE,
    BlueGray400 = 0xFF78909C,
    BlueGray500 = 0xFF607D8B,
    BlueGray600 = 0xFF546E7A,
    BlueGray700 = 0xFF455A64,
    BlueGray800 = 0xFF37474F,
    BlueGray900 = 0xFF263238,

    Black = 0xFF000000,
    White = 0xFFFFFFFF,
}

impl MaterialColorPalette {
    /// Initializes a new instance of the [`MaterialColorPalette`] class.
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
                    Color::from_uint32(MaterialColor::Red100 as u32),
                    Color::from_uint32(MaterialColor::Red200 as u32),
                    Color::from_uint32(MaterialColor::Red300 as u32),
                    Color::from_uint32(MaterialColor::Red400 as u32),
                    Color::from_uint32(MaterialColor::Red500 as u32),
                    Color::from_uint32(MaterialColor::Red600 as u32),
                    Color::from_uint32(MaterialColor::Red700 as u32),
                    Color::from_uint32(MaterialColor::Red800 as u32),
                    Color::from_uint32(MaterialColor::Red900 as u32),
                ],

                // Pink
                [
                    Color::from_uint32(MaterialColor::Pink50 as u32),
                    Color::from_uint32(MaterialColor::Pink100 as u32),
                    Color::from_uint32(MaterialColor::Pink200 as u32),
                    Color::from_uint32(MaterialColor::Pink300 as u32),
                    Color::from_uint32(MaterialColor::Pink400 as u32),
                    Color::from_uint32(MaterialColor::Pink500 as u32),
                    Color::from_uint32(MaterialColor::Pink600 as u32),
                    Color::from_uint32(MaterialColor::Pink700 as u32),
                    Color::from_uint32(MaterialColor::Pink800 as u32),
                    Color::from_uint32(MaterialColor::Pink900 as u32),
                ],

                // Purple
                [
                    Color::from_uint32(MaterialColor::Purple50 as u32),
                    Color::from_uint32(MaterialColor::Purple100 as u32),
                    Color::from_uint32(MaterialColor::Purple200 as u32),
                    Color::from_uint32(MaterialColor::Purple300 as u32),
                    Color::from_uint32(MaterialColor::Purple400 as u32),
                    Color::from_uint32(MaterialColor::Purple500 as u32),
                    Color::from_uint32(MaterialColor::Purple600 as u32),
                    Color::from_uint32(MaterialColor::Purple700 as u32),
                    Color::from_uint32(MaterialColor::Purple800 as u32),
                    Color::from_uint32(MaterialColor::Purple900 as u32),
                ],

                // Deep Purple
                [
                    Color::from_uint32(MaterialColor::DeepPurple50 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple100 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple200 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple300 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple400 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple500 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple600 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple700 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple800 as u32),
                    Color::from_uint32(MaterialColor::DeepPurple900 as u32),
                ],

                // Indigo
                [
                    Color::from_uint32(MaterialColor::Indigo50 as u32),
                    Color::from_uint32(MaterialColor::Indigo100 as u32),
                    Color::from_uint32(MaterialColor::Indigo200 as u32),
                    Color::from_uint32(MaterialColor::Indigo300 as u32),
                    Color::from_uint32(MaterialColor::Indigo400 as u32),
                    Color::from_uint32(MaterialColor::Indigo500 as u32),
                    Color::from_uint32(MaterialColor::Indigo600 as u32),
                    Color::from_uint32(MaterialColor::Indigo700 as u32),
                    Color::from_uint32(MaterialColor::Indigo800 as u32),
                    Color::from_uint32(MaterialColor::Indigo900 as u32),
                ],

                // Blue
                [
                    Color::from_uint32(MaterialColor::Blue50 as u32),
                    Color::from_uint32(MaterialColor::Blue100 as u32),
                    Color::from_uint32(MaterialColor::Blue200 as u32),
                    Color::from_uint32(MaterialColor::Blue300 as u32),
                    Color::from_uint32(MaterialColor::Blue400 as u32),
                    Color::from_uint32(MaterialColor::Blue500 as u32),
                    Color::from_uint32(MaterialColor::Blue600 as u32),
                    Color::from_uint32(MaterialColor::Blue700 as u32),
                    Color::from_uint32(MaterialColor::Blue800 as u32),
                    Color::from_uint32(MaterialColor::Blue900 as u32),
                ],

                // Light Blue
                [
                    Color::from_uint32(MaterialColor::LightBlue50 as u32),
                    Color::from_uint32(MaterialColor::LightBlue100 as u32),
                    Color::from_uint32(MaterialColor::LightBlue200 as u32),
                    Color::from_uint32(MaterialColor::LightBlue300 as u32),
                    Color::from_uint32(MaterialColor::LightBlue400 as u32),
                    Color::from_uint32(MaterialColor::LightBlue500 as u32),
                    Color::from_uint32(MaterialColor::LightBlue600 as u32),
                    Color::from_uint32(MaterialColor::LightBlue700 as u32),
                    Color::from_uint32(MaterialColor::LightBlue800 as u32),
                    Color::from_uint32(MaterialColor::LightBlue900 as u32),
                ],

                // Cyan
                [
                    Color::from_uint32(MaterialColor::Cyan50 as u32),
                    Color::from_uint32(MaterialColor::Cyan100 as u32),
                    Color::from_uint32(MaterialColor::Cyan200 as u32),
                    Color::from_uint32(MaterialColor::Cyan300 as u32),
                    Color::from_uint32(MaterialColor::Cyan400 as u32),
                    Color::from_uint32(MaterialColor::Cyan500 as u32),
                    Color::from_uint32(MaterialColor::Cyan600 as u32),
                    Color::from_uint32(MaterialColor::Cyan700 as u32),
                    Color::from_uint32(MaterialColor::Cyan800 as u32),
                    Color::from_uint32(MaterialColor::Cyan900 as u32),
                ],

                // Teal
                [
                    Color::from_uint32(MaterialColor::Teal50 as u32),
                    Color::from_uint32(MaterialColor::Teal100 as u32),
                    Color::from_uint32(MaterialColor::Teal200 as u32),
                    Color::from_uint32(MaterialColor::Teal300 as u32),
                    Color::from_uint32(MaterialColor::Teal400 as u32),
                    Color::from_uint32(MaterialColor::Teal500 as u32),
                    Color::from_uint32(MaterialColor::Teal600 as u32),
                    Color::from_uint32(MaterialColor::Teal700 as u32),
                    Color::from_uint32(MaterialColor::Teal800 as u32),
                    Color::from_uint32(MaterialColor::Teal900 as u32),
                ],

                // Green
                [
                    Color::from_uint32(MaterialColor::Green50 as u32),
                    Color::from_uint32(MaterialColor::Green100 as u32),
                    Color::from_uint32(MaterialColor::Green200 as u32),
                    Color::from_uint32(MaterialColor::Green300 as u32),
                    Color::from_uint32(MaterialColor::Green400 as u32),
                    Color::from_uint32(MaterialColor::Green500 as u32),
                    Color::from_uint32(MaterialColor::Green600 as u32),
                    Color::from_uint32(MaterialColor::Green700 as u32),
                    Color::from_uint32(MaterialColor::Green800 as u32),
                    Color::from_uint32(MaterialColor::Green900 as u32),
                ],

                // Light Green
                [
                    Color::from_uint32(MaterialColor::LightGreen50 as u32),
                    Color::from_uint32(MaterialColor::LightGreen100 as u32),
                    Color::from_uint32(MaterialColor::LightGreen200 as u32),
                    Color::from_uint32(MaterialColor::LightGreen300 as u32),
                    Color::from_uint32(MaterialColor::LightGreen400 as u32),
                    Color::from_uint32(MaterialColor::LightGreen500 as u32),
                    Color::from_uint32(MaterialColor::LightGreen600 as u32),
                    Color::from_uint32(MaterialColor::LightGreen700 as u32),
                    Color::from_uint32(MaterialColor::LightGreen800 as u32),
                    Color::from_uint32(MaterialColor::LightGreen900 as u32),
                ],

                // Lime
                [
                    Color::from_uint32(MaterialColor::Lime50 as u32),
                    Color::from_uint32(MaterialColor::Lime100 as u32),
                    Color::from_uint32(MaterialColor::Lime200 as u32),
                    Color::from_uint32(MaterialColor::Lime300 as u32),
                    Color::from_uint32(MaterialColor::Lime400 as u32),
                    Color::from_uint32(MaterialColor::Lime500 as u32),
                    Color::from_uint32(MaterialColor::Lime600 as u32),
                    Color::from_uint32(MaterialColor::Lime700 as u32),
                    Color::from_uint32(MaterialColor::Lime800 as u32),
                    Color::from_uint32(MaterialColor::Lime900 as u32),
                ],

                // Yellow
                [
                    Color::from_uint32(MaterialColor::Yellow50 as u32),
                    Color::from_uint32(MaterialColor::Yellow100 as u32),
                    Color::from_uint32(MaterialColor::Yellow200 as u32),
                    Color::from_uint32(MaterialColor::Yellow300 as u32),
                    Color::from_uint32(MaterialColor::Yellow400 as u32),
                    Color::from_uint32(MaterialColor::Yellow500 as u32),
                    Color::from_uint32(MaterialColor::Yellow600 as u32),
                    Color::from_uint32(MaterialColor::Yellow700 as u32),
                    Color::from_uint32(MaterialColor::Yellow800 as u32),
                    Color::from_uint32(MaterialColor::Yellow900 as u32),
                ],

                // Amber
                [
                    Color::from_uint32(MaterialColor::Amber50 as u32),
                    Color::from_uint32(MaterialColor::Amber100 as u32),
                    Color::from_uint32(MaterialColor::Amber200 as u32),
                    Color::from_uint32(MaterialColor::Amber300 as u32),
                    Color::from_uint32(MaterialColor::Amber400 as u32),
                    Color::from_uint32(MaterialColor::Amber500 as u32),
                    Color::from_uint32(MaterialColor::Amber600 as u32),
                    Color::from_uint32(MaterialColor::Amber700 as u32),
                    Color::from_uint32(MaterialColor::Amber800 as u32),
                    Color::from_uint32(MaterialColor::Amber900 as u32),
                ],

                // Orange
                [
                    Color::from_uint32(MaterialColor::Orange50 as u32),
                    Color::from_uint32(MaterialColor::Orange100 as u32),
                    Color::from_uint32(MaterialColor::Orange200 as u32),
                    Color::from_uint32(MaterialColor::Orange300 as u32),
                    Color::from_uint32(MaterialColor::Orange400 as u32),
                    Color::from_uint32(MaterialColor::Orange500 as u32),
                    Color::from_uint32(MaterialColor::Orange600 as u32),
                    Color::from_uint32(MaterialColor::Orange700 as u32),
                    Color::from_uint32(MaterialColor::Orange800 as u32),
                    Color::from_uint32(MaterialColor::Orange900 as u32),
                ],

                // Deep Orange
                [
                    Color::from_uint32(MaterialColor::DeepOrange50 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange100 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange200 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange300 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange400 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange500 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange600 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange700 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange800 as u32),
                    Color::from_uint32(MaterialColor::DeepOrange900 as u32),
                ],

                // Brown
                [
                    Color::from_uint32(MaterialColor::Brown50 as u32),
                    Color::from_uint32(MaterialColor::Brown100 as u32),
                    Color::from_uint32(MaterialColor::Brown200 as u32),
                    Color::from_uint32(MaterialColor::Brown300 as u32),
                    Color::from_uint32(MaterialColor::Brown400 as u32),
                    Color::from_uint32(MaterialColor::Brown500 as u32),
                    Color::from_uint32(MaterialColor::Brown600 as u32),
                    Color::from_uint32(MaterialColor::Brown700 as u32),
                    Color::from_uint32(MaterialColor::Brown800 as u32),
                    Color::from_uint32(MaterialColor::Brown900 as u32),
                ],

                // Gray
                [
                    Color::from_uint32(MaterialColor::Gray50 as u32),
                    Color::from_uint32(MaterialColor::Gray100 as u32),
                    Color::from_uint32(MaterialColor::Gray200 as u32),
                    Color::from_uint32(MaterialColor::Gray300 as u32),
                    Color::from_uint32(MaterialColor::Gray400 as u32),
                    Color::from_uint32(MaterialColor::Gray500 as u32),
                    Color::from_uint32(MaterialColor::Gray600 as u32),
                    Color::from_uint32(MaterialColor::Gray700 as u32),
                    Color::from_uint32(MaterialColor::Gray800 as u32),
                    Color::from_uint32(MaterialColor::Gray900 as u32),
                ],

                // Blue Gray
                [
                    Color::from_uint32(MaterialColor::BlueGray50 as u32),
                    Color::from_uint32(MaterialColor::BlueGray100 as u32),
                    Color::from_uint32(MaterialColor::BlueGray200 as u32),
                    Color::from_uint32(MaterialColor::BlueGray300 as u32),
                    Color::from_uint32(MaterialColor::BlueGray400 as u32),
                    Color::from_uint32(MaterialColor::BlueGray500 as u32),
                    Color::from_uint32(MaterialColor::BlueGray600 as u32),
                    Color::from_uint32(MaterialColor::BlueGray700 as u32),
                    Color::from_uint32(MaterialColor::BlueGray800 as u32),
                    Color::from_uint32(MaterialColor::BlueGray900 as u32),
                ],
            ]
        });
    }
}

impl IColorPalette for MaterialColorPalette {
    fn color_count(&self) -> i32 {
        19
    }

    fn shade_count(&self) -> i32 {
        10
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
