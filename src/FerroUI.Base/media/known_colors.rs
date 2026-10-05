//! The set of named colors and the lookups between names and ARGB values.

use crate::media::Color;

/// A named color, identified by its ARGB value.
///
/// Several names share a value (`AQUA`/`CYAN`, `FUCHSIA`/`MAGENTA`); those constants
/// compare equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct KnownColor(u32);

impl KnownColor {
    pub const NONE: KnownColor = KnownColor(0);
    pub const ALICE_BLUE: KnownColor = KnownColor(0xfff0f8ff);
    pub const ANTIQUE_WHITE: KnownColor = KnownColor(0xfffaebd7);
    pub const AQUA: KnownColor = KnownColor(0xff00ffff);
    pub const AQUAMARINE: KnownColor = KnownColor(0xff7fffd4);
    pub const AZURE: KnownColor = KnownColor(0xfff0ffff);
    pub const BEIGE: KnownColor = KnownColor(0xfff5f5dc);
    pub const BISQUE: KnownColor = KnownColor(0xffffe4c4);
    pub const BLACK: KnownColor = KnownColor(0xff000000);
    pub const BLANCHED_ALMOND: KnownColor = KnownColor(0xffffebcd);
    pub const BLUE: KnownColor = KnownColor(0xff0000ff);
    pub const BLUE_VIOLET: KnownColor = KnownColor(0xff8a2be2);
    pub const BROWN: KnownColor = KnownColor(0xffa52a2a);
    pub const BURLY_WOOD: KnownColor = KnownColor(0xffdeb887);
    pub const CADET_BLUE: KnownColor = KnownColor(0xff5f9ea0);
    pub const CHARTREUSE: KnownColor = KnownColor(0xff7fff00);
    pub const CHOCOLATE: KnownColor = KnownColor(0xffd2691e);
    pub const CORAL: KnownColor = KnownColor(0xffff7f50);
    pub const CORNFLOWER_BLUE: KnownColor = KnownColor(0xff6495ed);
    pub const CORNSILK: KnownColor = KnownColor(0xfffff8dc);
    pub const CRIMSON: KnownColor = KnownColor(0xffdc143c);
    pub const CYAN: KnownColor = KnownColor(0xff00ffff);
    pub const DARK_BLUE: KnownColor = KnownColor(0xff00008b);
    pub const DARK_CYAN: KnownColor = KnownColor(0xff008b8b);
    pub const DARK_GOLDENROD: KnownColor = KnownColor(0xffb8860b);
    pub const DARK_GRAY: KnownColor = KnownColor(0xffa9a9a9);
    pub const DARK_GREEN: KnownColor = KnownColor(0xff006400);
    pub const DARK_KHAKI: KnownColor = KnownColor(0xffbdb76b);
    pub const DARK_MAGENTA: KnownColor = KnownColor(0xff8b008b);
    pub const DARK_OLIVE_GREEN: KnownColor = KnownColor(0xff556b2f);
    pub const DARK_ORANGE: KnownColor = KnownColor(0xffff8c00);
    pub const DARK_ORCHID: KnownColor = KnownColor(0xff9932cc);
    pub const DARK_RED: KnownColor = KnownColor(0xff8b0000);
    pub const DARK_SALMON: KnownColor = KnownColor(0xffe9967a);
    pub const DARK_SEA_GREEN: KnownColor = KnownColor(0xff8fbc8f);
    pub const DARK_SLATE_BLUE: KnownColor = KnownColor(0xff483d8b);
    pub const DARK_SLATE_GRAY: KnownColor = KnownColor(0xff2f4f4f);
    pub const DARK_TURQUOISE: KnownColor = KnownColor(0xff00ced1);
    pub const DARK_VIOLET: KnownColor = KnownColor(0xff9400d3);
    pub const DEEP_PINK: KnownColor = KnownColor(0xffff1493);
    pub const DEEP_SKY_BLUE: KnownColor = KnownColor(0xff00bfff);
    pub const DIM_GRAY: KnownColor = KnownColor(0xff696969);
    pub const DODGER_BLUE: KnownColor = KnownColor(0xff1e90ff);
    pub const FIREBRICK: KnownColor = KnownColor(0xffb22222);
    pub const FLORAL_WHITE: KnownColor = KnownColor(0xfffffaf0);
    pub const FOREST_GREEN: KnownColor = KnownColor(0xff228b22);
    pub const FUCHSIA: KnownColor = KnownColor(0xffff00ff);
    pub const GAINSBORO: KnownColor = KnownColor(0xffdcdcdc);
    pub const GHOST_WHITE: KnownColor = KnownColor(0xfff8f8ff);
    pub const GOLD: KnownColor = KnownColor(0xffffd700);
    pub const GOLDENROD: KnownColor = KnownColor(0xffdaa520);
    pub const GRAY: KnownColor = KnownColor(0xff808080);
    pub const GREEN: KnownColor = KnownColor(0xff008000);
    pub const GREEN_YELLOW: KnownColor = KnownColor(0xffadff2f);
    pub const HONEYDEW: KnownColor = KnownColor(0xfff0fff0);
    pub const HOT_PINK: KnownColor = KnownColor(0xffff69b4);
    pub const INDIAN_RED: KnownColor = KnownColor(0xffcd5c5c);
    pub const INDIGO: KnownColor = KnownColor(0xff4b0082);
    pub const IVORY: KnownColor = KnownColor(0xfffffff0);
    pub const KHAKI: KnownColor = KnownColor(0xfff0e68c);
    pub const LAVENDER: KnownColor = KnownColor(0xffe6e6fa);
    pub const LAVENDER_BLUSH: KnownColor = KnownColor(0xfffff0f5);
    pub const LAWN_GREEN: KnownColor = KnownColor(0xff7cfc00);
    pub const LEMON_CHIFFON: KnownColor = KnownColor(0xfffffacd);
    pub const LIGHT_BLUE: KnownColor = KnownColor(0xffadd8e6);
    pub const LIGHT_CORAL: KnownColor = KnownColor(0xfff08080);
    pub const LIGHT_CYAN: KnownColor = KnownColor(0xffe0ffff);
    pub const LIGHT_GOLDENROD_YELLOW: KnownColor = KnownColor(0xfffafad2);
    pub const LIGHT_GREEN: KnownColor = KnownColor(0xff90ee90);
    pub const LIGHT_GRAY: KnownColor = KnownColor(0xffd3d3d3);
    pub const LIGHT_PINK: KnownColor = KnownColor(0xffffb6c1);
    pub const LIGHT_SALMON: KnownColor = KnownColor(0xffffa07a);
    pub const LIGHT_SEA_GREEN: KnownColor = KnownColor(0xff20b2aa);
    pub const LIGHT_SKY_BLUE: KnownColor = KnownColor(0xff87cefa);
    pub const LIGHT_SLATE_GRAY: KnownColor = KnownColor(0xff778899);
    pub const LIGHT_STEEL_BLUE: KnownColor = KnownColor(0xffb0c4de);
    pub const LIGHT_YELLOW: KnownColor = KnownColor(0xffffffe0);
    pub const LIME: KnownColor = KnownColor(0xff00ff00);
    pub const LIME_GREEN: KnownColor = KnownColor(0xff32cd32);
    pub const LINEN: KnownColor = KnownColor(0xfffaf0e6);
    pub const MAGENTA: KnownColor = KnownColor(0xffff00ff);
    pub const MAROON: KnownColor = KnownColor(0xff800000);
    pub const MEDIUM_AQUAMARINE: KnownColor = KnownColor(0xff66cdaa);
    pub const MEDIUM_BLUE: KnownColor = KnownColor(0xff0000cd);
    pub const MEDIUM_ORCHID: KnownColor = KnownColor(0xffba55d3);
    pub const MEDIUM_PURPLE: KnownColor = KnownColor(0xff9370db);
    pub const MEDIUM_SEA_GREEN: KnownColor = KnownColor(0xff3cb371);
    pub const MEDIUM_SLATE_BLUE: KnownColor = KnownColor(0xff7b68ee);
    pub const MEDIUM_SPRING_GREEN: KnownColor = KnownColor(0xff00fa9a);
    pub const MEDIUM_TURQUOISE: KnownColor = KnownColor(0xff48d1cc);
    pub const MEDIUM_VIOLET_RED: KnownColor = KnownColor(0xffc71585);
    pub const MIDNIGHT_BLUE: KnownColor = KnownColor(0xff191970);
    pub const MINT_CREAM: KnownColor = KnownColor(0xfff5fffa);
    pub const MISTY_ROSE: KnownColor = KnownColor(0xffffe4e1);
    pub const MOCCASIN: KnownColor = KnownColor(0xffffe4b5);
    pub const NAVAJO_WHITE: KnownColor = KnownColor(0xffffdead);
    pub const NAVY: KnownColor = KnownColor(0xff000080);
    pub const OLD_LACE: KnownColor = KnownColor(0xfffdf5e6);
    pub const OLIVE: KnownColor = KnownColor(0xff808000);
    pub const OLIVE_DRAB: KnownColor = KnownColor(0xff6b8e23);
    pub const ORANGE: KnownColor = KnownColor(0xffffa500);
    pub const ORANGE_RED: KnownColor = KnownColor(0xffff4500);
    pub const ORCHID: KnownColor = KnownColor(0xffda70d6);
    pub const PALE_GOLDENROD: KnownColor = KnownColor(0xffeee8aa);
    pub const PALE_GREEN: KnownColor = KnownColor(0xff98fb98);
    pub const PALE_TURQUOISE: KnownColor = KnownColor(0xffafeeee);
    pub const PALE_VIOLET_RED: KnownColor = KnownColor(0xffdb7093);
    pub const PAPAYA_WHIP: KnownColor = KnownColor(0xffffefd5);
    pub const PEACH_PUFF: KnownColor = KnownColor(0xffffdab9);
    pub const PERU: KnownColor = KnownColor(0xffcd853f);
    pub const PINK: KnownColor = KnownColor(0xffffc0cb);
    pub const PLUM: KnownColor = KnownColor(0xffdda0dd);
    pub const POWDER_BLUE: KnownColor = KnownColor(0xffb0e0e6);
    pub const PURPLE: KnownColor = KnownColor(0xff800080);
    pub const RED: KnownColor = KnownColor(0xffff0000);
    pub const ROSY_BROWN: KnownColor = KnownColor(0xffbc8f8f);
    pub const ROYAL_BLUE: KnownColor = KnownColor(0xff4169e1);
    pub const SADDLE_BROWN: KnownColor = KnownColor(0xff8b4513);
    pub const SALMON: KnownColor = KnownColor(0xfffa8072);
    pub const SANDY_BROWN: KnownColor = KnownColor(0xfff4a460);
    pub const SEA_GREEN: KnownColor = KnownColor(0xff2e8b57);
    pub const SEA_SHELL: KnownColor = KnownColor(0xfffff5ee);
    pub const SIENNA: KnownColor = KnownColor(0xffa0522d);
    pub const SILVER: KnownColor = KnownColor(0xffc0c0c0);
    pub const SKY_BLUE: KnownColor = KnownColor(0xff87ceeb);
    pub const SLATE_BLUE: KnownColor = KnownColor(0xff6a5acd);
    pub const SLATE_GRAY: KnownColor = KnownColor(0xff708090);
    pub const SNOW: KnownColor = KnownColor(0xfffffafa);
    pub const SPRING_GREEN: KnownColor = KnownColor(0xff00ff7f);
    pub const STEEL_BLUE: KnownColor = KnownColor(0xff4682b4);
    pub const TAN: KnownColor = KnownColor(0xffd2b48c);
    pub const TEAL: KnownColor = KnownColor(0xff008080);
    pub const THISTLE: KnownColor = KnownColor(0xffd8bfd8);
    pub const TOMATO: KnownColor = KnownColor(0xffff6347);
    pub const TRANSPARENT: KnownColor = KnownColor(0x00ffffff);
    pub const TURQUOISE: KnownColor = KnownColor(0xff40e0d0);
    pub const VIOLET: KnownColor = KnownColor(0xffee82ee);
    pub const WHEAT: KnownColor = KnownColor(0xfff5deb3);
    pub const WHITE: KnownColor = KnownColor(0xffffffff);
    pub const WHITE_SMOKE: KnownColor = KnownColor(0xfff5f5f5);
    pub const YELLOW: KnownColor = KnownColor(0xffffff00);
    pub const YELLOW_GREEN: KnownColor = KnownColor(0xff9acd32);

    /// Gets the ARGB value of the known color.
    #[inline]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Converts the known color to a [`Color`].
    #[inline]
    pub const fn to_color(self) -> Color {
        Color::from_uint32(self.0)
    }
}

/// Lookups between color names and known colors.
pub struct KnownColors;

impl KnownColors {
    /// Gets the known color with the given name (ASCII case-insensitive), or
    /// [`KnownColor::NONE`] when there is no such color.
    pub fn get_known_color(s: &str) -> KnownColor {
        match BY_NAME.binary_search_by(|(name, _)| compare_ignore_ascii_case(name, s)) {
            Ok(index) => BY_NAME[index].1,
            Err(_) => KnownColor::NONE,
        }
    }

    /// Gets the name of the known color with the given ARGB value. When several
    /// names share the value, the first one in declaration order is returned.
    pub fn get_known_color_name(rgb: u32) -> Option<&'static str> {
        match BY_VALUE.binary_search_by_key(&rgb, |(value, _)| *value) {
            Ok(index) => Some(BY_VALUE[index].1),
            Err(_) => None,
        }
    }

    /// Same as [`get_known_color_name`](Self::get_known_color_name).
    #[inline]
    pub fn try_get_known_color_name(rgb: u32) -> Option<&'static str> {
        Self::get_known_color_name(rgb)
    }

    /// Converts the known color to a [`Color`].
    #[inline]
    pub const fn to_color(color: KnownColor) -> Color {
        color.to_color()
    }
}

/// Orders `name` (already lower-case) relative to `s`, ignoring the ASCII case of `s`.
fn compare_ignore_ascii_case(name: &str, s: &str) -> std::cmp::Ordering {
    name.bytes().cmp(s.bytes().map(|b| b.to_ascii_lowercase()))
}

/// Lower-cased names, sorted, for the case-insensitive name lookup.
static BY_NAME: [(&str, KnownColor); 141] = [
    ("aliceblue", KnownColor::ALICE_BLUE),
    ("antiquewhite", KnownColor::ANTIQUE_WHITE),
    ("aqua", KnownColor::AQUA),
    ("aquamarine", KnownColor::AQUAMARINE),
    ("azure", KnownColor::AZURE),
    ("beige", KnownColor::BEIGE),
    ("bisque", KnownColor::BISQUE),
    ("black", KnownColor::BLACK),
    ("blanchedalmond", KnownColor::BLANCHED_ALMOND),
    ("blue", KnownColor::BLUE),
    ("blueviolet", KnownColor::BLUE_VIOLET),
    ("brown", KnownColor::BROWN),
    ("burlywood", KnownColor::BURLY_WOOD),
    ("cadetblue", KnownColor::CADET_BLUE),
    ("chartreuse", KnownColor::CHARTREUSE),
    ("chocolate", KnownColor::CHOCOLATE),
    ("coral", KnownColor::CORAL),
    ("cornflowerblue", KnownColor::CORNFLOWER_BLUE),
    ("cornsilk", KnownColor::CORNSILK),
    ("crimson", KnownColor::CRIMSON),
    ("cyan", KnownColor::CYAN),
    ("darkblue", KnownColor::DARK_BLUE),
    ("darkcyan", KnownColor::DARK_CYAN),
    ("darkgoldenrod", KnownColor::DARK_GOLDENROD),
    ("darkgray", KnownColor::DARK_GRAY),
    ("darkgreen", KnownColor::DARK_GREEN),
    ("darkkhaki", KnownColor::DARK_KHAKI),
    ("darkmagenta", KnownColor::DARK_MAGENTA),
    ("darkolivegreen", KnownColor::DARK_OLIVE_GREEN),
    ("darkorange", KnownColor::DARK_ORANGE),
    ("darkorchid", KnownColor::DARK_ORCHID),
    ("darkred", KnownColor::DARK_RED),
    ("darksalmon", KnownColor::DARK_SALMON),
    ("darkseagreen", KnownColor::DARK_SEA_GREEN),
    ("darkslateblue", KnownColor::DARK_SLATE_BLUE),
    ("darkslategray", KnownColor::DARK_SLATE_GRAY),
    ("darkturquoise", KnownColor::DARK_TURQUOISE),
    ("darkviolet", KnownColor::DARK_VIOLET),
    ("deeppink", KnownColor::DEEP_PINK),
    ("deepskyblue", KnownColor::DEEP_SKY_BLUE),
    ("dimgray", KnownColor::DIM_GRAY),
    ("dodgerblue", KnownColor::DODGER_BLUE),
    ("firebrick", KnownColor::FIREBRICK),
    ("floralwhite", KnownColor::FLORAL_WHITE),
    ("forestgreen", KnownColor::FOREST_GREEN),
    ("fuchsia", KnownColor::FUCHSIA),
    ("gainsboro", KnownColor::GAINSBORO),
    ("ghostwhite", KnownColor::GHOST_WHITE),
    ("gold", KnownColor::GOLD),
    ("goldenrod", KnownColor::GOLDENROD),
    ("gray", KnownColor::GRAY),
    ("green", KnownColor::GREEN),
    ("greenyellow", KnownColor::GREEN_YELLOW),
    ("honeydew", KnownColor::HONEYDEW),
    ("hotpink", KnownColor::HOT_PINK),
    ("indianred", KnownColor::INDIAN_RED),
    ("indigo", KnownColor::INDIGO),
    ("ivory", KnownColor::IVORY),
    ("khaki", KnownColor::KHAKI),
    ("lavender", KnownColor::LAVENDER),
    ("lavenderblush", KnownColor::LAVENDER_BLUSH),
    ("lawngreen", KnownColor::LAWN_GREEN),
    ("lemonchiffon", KnownColor::LEMON_CHIFFON),
    ("lightblue", KnownColor::LIGHT_BLUE),
    ("lightcoral", KnownColor::LIGHT_CORAL),
    ("lightcyan", KnownColor::LIGHT_CYAN),
    ("lightgoldenrodyellow", KnownColor::LIGHT_GOLDENROD_YELLOW),
    ("lightgray", KnownColor::LIGHT_GRAY),
    ("lightgreen", KnownColor::LIGHT_GREEN),
    ("lightpink", KnownColor::LIGHT_PINK),
    ("lightsalmon", KnownColor::LIGHT_SALMON),
    ("lightseagreen", KnownColor::LIGHT_SEA_GREEN),
    ("lightskyblue", KnownColor::LIGHT_SKY_BLUE),
    ("lightslategray", KnownColor::LIGHT_SLATE_GRAY),
    ("lightsteelblue", KnownColor::LIGHT_STEEL_BLUE),
    ("lightyellow", KnownColor::LIGHT_YELLOW),
    ("lime", KnownColor::LIME),
    ("limegreen", KnownColor::LIME_GREEN),
    ("linen", KnownColor::LINEN),
    ("magenta", KnownColor::MAGENTA),
    ("maroon", KnownColor::MAROON),
    ("mediumaquamarine", KnownColor::MEDIUM_AQUAMARINE),
    ("mediumblue", KnownColor::MEDIUM_BLUE),
    ("mediumorchid", KnownColor::MEDIUM_ORCHID),
    ("mediumpurple", KnownColor::MEDIUM_PURPLE),
    ("mediumseagreen", KnownColor::MEDIUM_SEA_GREEN),
    ("mediumslateblue", KnownColor::MEDIUM_SLATE_BLUE),
    ("mediumspringgreen", KnownColor::MEDIUM_SPRING_GREEN),
    ("mediumturquoise", KnownColor::MEDIUM_TURQUOISE),
    ("mediumvioletred", KnownColor::MEDIUM_VIOLET_RED),
    ("midnightblue", KnownColor::MIDNIGHT_BLUE),
    ("mintcream", KnownColor::MINT_CREAM),
    ("mistyrose", KnownColor::MISTY_ROSE),
    ("moccasin", KnownColor::MOCCASIN),
    ("navajowhite", KnownColor::NAVAJO_WHITE),
    ("navy", KnownColor::NAVY),
    ("oldlace", KnownColor::OLD_LACE),
    ("olive", KnownColor::OLIVE),
    ("olivedrab", KnownColor::OLIVE_DRAB),
    ("orange", KnownColor::ORANGE),
    ("orangered", KnownColor::ORANGE_RED),
    ("orchid", KnownColor::ORCHID),
    ("palegoldenrod", KnownColor::PALE_GOLDENROD),
    ("palegreen", KnownColor::PALE_GREEN),
    ("paleturquoise", KnownColor::PALE_TURQUOISE),
    ("palevioletred", KnownColor::PALE_VIOLET_RED),
    ("papayawhip", KnownColor::PAPAYA_WHIP),
    ("peachpuff", KnownColor::PEACH_PUFF),
    ("peru", KnownColor::PERU),
    ("pink", KnownColor::PINK),
    ("plum", KnownColor::PLUM),
    ("powderblue", KnownColor::POWDER_BLUE),
    ("purple", KnownColor::PURPLE),
    ("red", KnownColor::RED),
    ("rosybrown", KnownColor::ROSY_BROWN),
    ("royalblue", KnownColor::ROYAL_BLUE),
    ("saddlebrown", KnownColor::SADDLE_BROWN),
    ("salmon", KnownColor::SALMON),
    ("sandybrown", KnownColor::SANDY_BROWN),
    ("seagreen", KnownColor::SEA_GREEN),
    ("seashell", KnownColor::SEA_SHELL),
    ("sienna", KnownColor::SIENNA),
    ("silver", KnownColor::SILVER),
    ("skyblue", KnownColor::SKY_BLUE),
    ("slateblue", KnownColor::SLATE_BLUE),
    ("slategray", KnownColor::SLATE_GRAY),
    ("snow", KnownColor::SNOW),
    ("springgreen", KnownColor::SPRING_GREEN),
    ("steelblue", KnownColor::STEEL_BLUE),
    ("tan", KnownColor::TAN),
    ("teal", KnownColor::TEAL),
    ("thistle", KnownColor::THISTLE),
    ("tomato", KnownColor::TOMATO),
    ("transparent", KnownColor::TRANSPARENT),
    ("turquoise", KnownColor::TURQUOISE),
    ("violet", KnownColor::VIOLET),
    ("wheat", KnownColor::WHEAT),
    ("white", KnownColor::WHITE),
    ("whitesmoke", KnownColor::WHITE_SMOKE),
    ("yellow", KnownColor::YELLOW),
    ("yellowgreen", KnownColor::YELLOW_GREEN),
];

/// Display names sorted by ARGB value; the first declared name wins for shared values.
static BY_VALUE: [(u32, &str); 139] = [
    (0x00ffffff, "Transparent"),
    (0xff000000, "Black"),
    (0xff000080, "Navy"),
    (0xff00008b, "DarkBlue"),
    (0xff0000cd, "MediumBlue"),
    (0xff0000ff, "Blue"),
    (0xff006400, "DarkGreen"),
    (0xff008000, "Green"),
    (0xff008080, "Teal"),
    (0xff008b8b, "DarkCyan"),
    (0xff00bfff, "DeepSkyBlue"),
    (0xff00ced1, "DarkTurquoise"),
    (0xff00fa9a, "MediumSpringGreen"),
    (0xff00ff00, "Lime"),
    (0xff00ff7f, "SpringGreen"),
    (0xff00ffff, "Aqua"),
    (0xff191970, "MidnightBlue"),
    (0xff1e90ff, "DodgerBlue"),
    (0xff20b2aa, "LightSeaGreen"),
    (0xff228b22, "ForestGreen"),
    (0xff2e8b57, "SeaGreen"),
    (0xff2f4f4f, "DarkSlateGray"),
    (0xff32cd32, "LimeGreen"),
    (0xff3cb371, "MediumSeaGreen"),
    (0xff40e0d0, "Turquoise"),
    (0xff4169e1, "RoyalBlue"),
    (0xff4682b4, "SteelBlue"),
    (0xff483d8b, "DarkSlateBlue"),
    (0xff48d1cc, "MediumTurquoise"),
    (0xff4b0082, "Indigo"),
    (0xff556b2f, "DarkOliveGreen"),
    (0xff5f9ea0, "CadetBlue"),
    (0xff6495ed, "CornflowerBlue"),
    (0xff66cdaa, "MediumAquamarine"),
    (0xff696969, "DimGray"),
    (0xff6a5acd, "SlateBlue"),
    (0xff6b8e23, "OliveDrab"),
    (0xff708090, "SlateGray"),
    (0xff778899, "LightSlateGray"),
    (0xff7b68ee, "MediumSlateBlue"),
    (0xff7cfc00, "LawnGreen"),
    (0xff7fff00, "Chartreuse"),
    (0xff7fffd4, "Aquamarine"),
    (0xff800000, "Maroon"),
    (0xff800080, "Purple"),
    (0xff808000, "Olive"),
    (0xff808080, "Gray"),
    (0xff87ceeb, "SkyBlue"),
    (0xff87cefa, "LightSkyBlue"),
    (0xff8a2be2, "BlueViolet"),
    (0xff8b0000, "DarkRed"),
    (0xff8b008b, "DarkMagenta"),
    (0xff8b4513, "SaddleBrown"),
    (0xff8fbc8f, "DarkSeaGreen"),
    (0xff90ee90, "LightGreen"),
    (0xff9370db, "MediumPurple"),
    (0xff9400d3, "DarkViolet"),
    (0xff98fb98, "PaleGreen"),
    (0xff9932cc, "DarkOrchid"),
    (0xff9acd32, "YellowGreen"),
    (0xffa0522d, "Sienna"),
    (0xffa52a2a, "Brown"),
    (0xffa9a9a9, "DarkGray"),
    (0xffadd8e6, "LightBlue"),
    (0xffadff2f, "GreenYellow"),
    (0xffafeeee, "PaleTurquoise"),
    (0xffb0c4de, "LightSteelBlue"),
    (0xffb0e0e6, "PowderBlue"),
    (0xffb22222, "Firebrick"),
    (0xffb8860b, "DarkGoldenrod"),
    (0xffba55d3, "MediumOrchid"),
    (0xffbc8f8f, "RosyBrown"),
    (0xffbdb76b, "DarkKhaki"),
    (0xffc0c0c0, "Silver"),
    (0xffc71585, "MediumVioletRed"),
    (0xffcd5c5c, "IndianRed"),
    (0xffcd853f, "Peru"),
    (0xffd2691e, "Chocolate"),
    (0xffd2b48c, "Tan"),
    (0xffd3d3d3, "LightGray"),
    (0xffd8bfd8, "Thistle"),
    (0xffda70d6, "Orchid"),
    (0xffdaa520, "Goldenrod"),
    (0xffdb7093, "PaleVioletRed"),
    (0xffdc143c, "Crimson"),
    (0xffdcdcdc, "Gainsboro"),
    (0xffdda0dd, "Plum"),
    (0xffdeb887, "BurlyWood"),
    (0xffe0ffff, "LightCyan"),
    (0xffe6e6fa, "Lavender"),
    (0xffe9967a, "DarkSalmon"),
    (0xffee82ee, "Violet"),
    (0xffeee8aa, "PaleGoldenrod"),
    (0xfff08080, "LightCoral"),
    (0xfff0e68c, "Khaki"),
    (0xfff0f8ff, "AliceBlue"),
    (0xfff0fff0, "Honeydew"),
    (0xfff0ffff, "Azure"),
    (0xfff4a460, "SandyBrown"),
    (0xfff5deb3, "Wheat"),
    (0xfff5f5dc, "Beige"),
    (0xfff5f5f5, "WhiteSmoke"),
    (0xfff5fffa, "MintCream"),
    (0xfff8f8ff, "GhostWhite"),
    (0xfffa8072, "Salmon"),
    (0xfffaebd7, "AntiqueWhite"),
    (0xfffaf0e6, "Linen"),
    (0xfffafad2, "LightGoldenrodYellow"),
    (0xfffdf5e6, "OldLace"),
    (0xffff0000, "Red"),
    (0xffff00ff, "Fuchsia"),
    (0xffff1493, "DeepPink"),
    (0xffff4500, "OrangeRed"),
    (0xffff6347, "Tomato"),
    (0xffff69b4, "HotPink"),
    (0xffff7f50, "Coral"),
    (0xffff8c00, "DarkOrange"),
    (0xffffa07a, "LightSalmon"),
    (0xffffa500, "Orange"),
    (0xffffb6c1, "LightPink"),
    (0xffffc0cb, "Pink"),
    (0xffffd700, "Gold"),
    (0xffffdab9, "PeachPuff"),
    (0xffffdead, "NavajoWhite"),
    (0xffffe4b5, "Moccasin"),
    (0xffffe4c4, "Bisque"),
    (0xffffe4e1, "MistyRose"),
    (0xffffebcd, "BlanchedAlmond"),
    (0xffffefd5, "PapayaWhip"),
    (0xfffff0f5, "LavenderBlush"),
    (0xfffff5ee, "SeaShell"),
    (0xfffff8dc, "Cornsilk"),
    (0xfffffacd, "LemonChiffon"),
    (0xfffffaf0, "FloralWhite"),
    (0xfffffafa, "Snow"),
    (0xffffff00, "Yellow"),
    (0xffffffe0, "LightYellow"),
    (0xfffffff0, "Ivory"),
    (0xffffffff, "White"),
];

#[cfg(test)]
mod tests {
    // The reference test-suite has no dedicated tests for this type.
    use super::*;

    #[test]
    fn tables_are_sorted_for_binary_search() {
        assert!(BY_NAME.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(BY_VALUE.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(BY_NAME
            .iter()
            .all(|(name, _)| name.bytes().all(|b| b.is_ascii_lowercase())));
    }

    #[test]
    fn name_lookup_is_case_insensitive() {
        assert_eq!(KnownColors::get_known_color("red"), KnownColor::RED);
        assert_eq!(KnownColors::get_known_color("RED"), KnownColor::RED);
        assert_eq!(
            KnownColors::get_known_color("AliceBlue"),
            KnownColor::ALICE_BLUE
        );
        assert_eq!(
            KnownColors::get_known_color("yellowgreen"),
            KnownColor::YELLOW_GREEN
        );
        assert_eq!(
            KnownColors::get_known_color("Transparent").value(),
            0x00ffffff
        );
        assert_eq!(KnownColors::get_known_color("None"), KnownColor::NONE);
        assert_eq!(KnownColors::get_known_color(""), KnownColor::NONE);
        assert_eq!(KnownColors::get_known_color("reds"), KnownColor::NONE);
        assert_eq!(KnownColors::get_known_color("re"), KnownColor::NONE);
    }

    #[test]
    fn every_name_round_trips() {
        for (name, color) in BY_NAME.iter() {
            assert_eq!(KnownColors::get_known_color(name), *color);
            assert_eq!(
                KnownColors::get_known_color(&name.to_ascii_uppercase()),
                *color
            );
            let display = KnownColors::get_known_color_name(color.value()).unwrap();
            assert_eq!(KnownColors::get_known_color(display), *color);
        }
    }

    #[test]
    fn shared_values_use_first_declared_name() {
        assert_eq!(KnownColor::AQUA, KnownColor::CYAN);
        assert_eq!(KnownColor::FUCHSIA, KnownColor::MAGENTA);
        assert_eq!(KnownColors::get_known_color_name(0xff00ffff), Some("Aqua"));
        assert_eq!(
            KnownColors::get_known_color_name(0xffff00ff),
            Some("Fuchsia")
        );
        assert_eq!(KnownColors::get_known_color_name(0xffff0000), Some("Red"));
        assert_eq!(
            KnownColors::get_known_color_name(0x00ffffff),
            Some("Transparent")
        );
        assert_eq!(KnownColors::get_known_color_name(0), None);
        assert_eq!(KnownColors::get_known_color_name(0xff123456), None);
    }
}
