use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::KnownColor;
use std::rc::Rc;

/// Predefined brushes.
pub struct Brushes;

impl Brushes {
    /// Gets a brush with the `AliceBlue` color.
    pub fn alice_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::ALICE_BLUE.to_brush()
    }

    /// Gets a brush with the `AntiqueWhite` color.
    pub fn antique_white() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::ANTIQUE_WHITE.to_brush()
    }

    /// Gets a brush with the `Aqua` color.
    pub fn aqua() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::AQUA.to_brush()
    }

    /// Gets a brush with the `Aquamarine` color.
    pub fn aquamarine() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::AQUAMARINE.to_brush()
    }

    /// Gets a brush with the `Azure` color.
    pub fn azure() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::AZURE.to_brush()
    }

    /// Gets a brush with the `Beige` color.
    pub fn beige() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::BEIGE.to_brush()
    }

    /// Gets a brush with the `Bisque` color.
    pub fn bisque() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::BISQUE.to_brush()
    }

    /// Gets a brush with the `Black` color.
    pub fn black() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::BLACK.to_brush()
    }

    /// Gets a brush with the `BlanchedAlmond` color.
    pub fn blanched_almond() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::BLANCHED_ALMOND.to_brush()
    }

    /// Gets a brush with the `Blue` color.
    pub fn blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::BLUE.to_brush()
    }

    /// Gets a brush with the `BlueViolet` color.
    pub fn blue_violet() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::BLUE_VIOLET.to_brush()
    }

    /// Gets a brush with the `Brown` color.
    pub fn brown() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::BROWN.to_brush()
    }

    /// Gets a brush with the `BurlyWood` color.
    pub fn burly_wood() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::BURLY_WOOD.to_brush()
    }

    /// Gets a brush with the `CadetBlue` color.
    pub fn cadet_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::CADET_BLUE.to_brush()
    }

    /// Gets a brush with the `Chartreuse` color.
    pub fn chartreuse() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::CHARTREUSE.to_brush()
    }

    /// Gets a brush with the `Chocolate` color.
    pub fn chocolate() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::CHOCOLATE.to_brush()
    }

    /// Gets a brush with the `Coral` color.
    pub fn coral() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::CORAL.to_brush()
    }

    /// Gets a brush with the `CornflowerBlue` color.
    pub fn cornflower_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::CORNFLOWER_BLUE.to_brush()
    }

    /// Gets a brush with the `Cornsilk` color.
    pub fn cornsilk() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::CORNSILK.to_brush()
    }

    /// Gets a brush with the `Crimson` color.
    pub fn crimson() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::CRIMSON.to_brush()
    }

    /// Gets a brush with the `Cyan` color.
    pub fn cyan() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::CYAN.to_brush()
    }

    /// Gets a brush with the `DarkBlue` color.
    pub fn dark_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_BLUE.to_brush()
    }

    /// Gets a brush with the `DarkCyan` color.
    pub fn dark_cyan() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_CYAN.to_brush()
    }

    /// Gets a brush with the `DarkGoldenrod` color.
    pub fn dark_goldenrod() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_GOLDENROD.to_brush()
    }

    /// Gets a brush with the `DarkGray` color.
    pub fn dark_gray() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_GRAY.to_brush()
    }

    /// Gets a brush with the `DarkGreen` color.
    pub fn dark_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_GREEN.to_brush()
    }

    /// Gets a brush with the `DarkKhaki` color.
    pub fn dark_khaki() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_KHAKI.to_brush()
    }

    /// Gets a brush with the `DarkMagenta` color.
    pub fn dark_magenta() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_MAGENTA.to_brush()
    }

    /// Gets a brush with the `DarkOliveGreen` color.
    pub fn dark_olive_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_OLIVE_GREEN.to_brush()
    }

    /// Gets a brush with the `DarkOrange` color.
    pub fn dark_orange() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_ORANGE.to_brush()
    }

    /// Gets a brush with the `DarkOrchid` color.
    pub fn dark_orchid() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_ORCHID.to_brush()
    }

    /// Gets a brush with the `DarkRed` color.
    pub fn dark_red() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_RED.to_brush()
    }

    /// Gets a brush with the `DarkSalmon` color.
    pub fn dark_salmon() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_SALMON.to_brush()
    }

    /// Gets a brush with the `DarkSeaGreen` color.
    pub fn dark_sea_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_SEA_GREEN.to_brush()
    }

    /// Gets a brush with the `DarkSlateBlue` color.
    pub fn dark_slate_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_SLATE_BLUE.to_brush()
    }

    /// Gets a brush with the `DarkSlateGray` color.
    pub fn dark_slate_gray() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_SLATE_GRAY.to_brush()
    }

    /// Gets a brush with the `DarkTurquoise` color.
    pub fn dark_turquoise() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_TURQUOISE.to_brush()
    }

    /// Gets a brush with the `DarkViolet` color.
    pub fn dark_violet() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DARK_VIOLET.to_brush()
    }

    /// Gets a brush with the `DeepPink` color.
    pub fn deep_pink() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DEEP_PINK.to_brush()
    }

    /// Gets a brush with the `DeepSkyBlue` color.
    pub fn deep_sky_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DEEP_SKY_BLUE.to_brush()
    }

    /// Gets a brush with the `DimGray` color.
    pub fn dim_gray() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DIM_GRAY.to_brush()
    }

    /// Gets a brush with the `DodgerBlue` color.
    pub fn dodger_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::DODGER_BLUE.to_brush()
    }

    /// Gets a brush with the `Firebrick` color.
    pub fn firebrick() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::FIREBRICK.to_brush()
    }

    /// Gets a brush with the `FloralWhite` color.
    pub fn floral_white() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::FLORAL_WHITE.to_brush()
    }

    /// Gets a brush with the `ForestGreen` color.
    pub fn forest_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::FOREST_GREEN.to_brush()
    }

    /// Gets a brush with the `Fuchsia` color.
    pub fn fuchsia() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::FUCHSIA.to_brush()
    }

    /// Gets a brush with the `Gainsboro` color.
    pub fn gainsboro() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::GAINSBORO.to_brush()
    }

    /// Gets a brush with the `GhostWhite` color.
    pub fn ghost_white() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::GHOST_WHITE.to_brush()
    }

    /// Gets a brush with the `Gold` color.
    pub fn gold() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::GOLD.to_brush()
    }

    /// Gets a brush with the `Goldenrod` color.
    pub fn goldenrod() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::GOLDENROD.to_brush()
    }

    /// Gets a brush with the `Gray` color.
    pub fn gray() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::GRAY.to_brush()
    }

    /// Gets a brush with the `Green` color.
    pub fn green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::GREEN.to_brush()
    }

    /// Gets a brush with the `GreenYellow` color.
    pub fn green_yellow() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::GREEN_YELLOW.to_brush()
    }

    /// Gets a brush with the `Honeydew` color.
    pub fn honeydew() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::HONEYDEW.to_brush()
    }

    /// Gets a brush with the `HotPink` color.
    pub fn hot_pink() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::HOT_PINK.to_brush()
    }

    /// Gets a brush with the `IndianRed` color.
    pub fn indian_red() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::INDIAN_RED.to_brush()
    }

    /// Gets a brush with the `Indigo` color.
    pub fn indigo() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::INDIGO.to_brush()
    }

    /// Gets a brush with the `Ivory` color.
    pub fn ivory() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::IVORY.to_brush()
    }

    /// Gets a brush with the `Khaki` color.
    pub fn khaki() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::KHAKI.to_brush()
    }

    /// Gets a brush with the `Lavender` color.
    pub fn lavender() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LAVENDER.to_brush()
    }

    /// Gets a brush with the `LavenderBlush` color.
    pub fn lavender_blush() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LAVENDER_BLUSH.to_brush()
    }

    /// Gets a brush with the `LawnGreen` color.
    pub fn lawn_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LAWN_GREEN.to_brush()
    }

    /// Gets a brush with the `LemonChiffon` color.
    pub fn lemon_chiffon() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LEMON_CHIFFON.to_brush()
    }

    /// Gets a brush with the `LightBlue` color.
    pub fn light_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_BLUE.to_brush()
    }

    /// Gets a brush with the `LightCoral` color.
    pub fn light_coral() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_CORAL.to_brush()
    }

    /// Gets a brush with the `LightCyan` color.
    pub fn light_cyan() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_CYAN.to_brush()
    }

    /// Gets a brush with the `LightGoldenrodYellow` color.
    pub fn light_goldenrod_yellow() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_GOLDENROD_YELLOW.to_brush()
    }

    /// Gets a brush with the `LightGray` color.
    pub fn light_gray() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_GRAY.to_brush()
    }

    /// Gets a brush with the `LightGreen` color.
    pub fn light_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_GREEN.to_brush()
    }

    /// Gets a brush with the `LightPink` color.
    pub fn light_pink() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_PINK.to_brush()
    }

    /// Gets a brush with the `LightSalmon` color.
    pub fn light_salmon() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_SALMON.to_brush()
    }

    /// Gets a brush with the `LightSeaGreen` color.
    pub fn light_sea_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_SEA_GREEN.to_brush()
    }

    /// Gets a brush with the `LightSkyBlue` color.
    pub fn light_sky_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_SKY_BLUE.to_brush()
    }

    /// Gets a brush with the `LightSlateGray` color.
    pub fn light_slate_gray() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_SLATE_GRAY.to_brush()
    }

    /// Gets a brush with the `LightSteelBlue` color.
    pub fn light_steel_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_STEEL_BLUE.to_brush()
    }

    /// Gets a brush with the `LightYellow` color.
    pub fn light_yellow() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIGHT_YELLOW.to_brush()
    }

    /// Gets a brush with the `Lime` color.
    pub fn lime() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIME.to_brush()
    }

    /// Gets a brush with the `LimeGreen` color.
    pub fn lime_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LIME_GREEN.to_brush()
    }

    /// Gets a brush with the `Linen` color.
    pub fn linen() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::LINEN.to_brush()
    }

    /// Gets a brush with the `Magenta` color.
    pub fn magenta() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MAGENTA.to_brush()
    }

    /// Gets a brush with the `Maroon` color.
    pub fn maroon() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MAROON.to_brush()
    }

    /// Gets a brush with the `MediumAquamarine` color.
    pub fn medium_aquamarine() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_AQUAMARINE.to_brush()
    }

    /// Gets a brush with the `MediumBlue` color.
    pub fn medium_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_BLUE.to_brush()
    }

    /// Gets a brush with the `MediumOrchid` color.
    pub fn medium_orchid() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_ORCHID.to_brush()
    }

    /// Gets a brush with the `MediumPurple` color.
    pub fn medium_purple() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_PURPLE.to_brush()
    }

    /// Gets a brush with the `MediumSeaGreen` color.
    pub fn medium_sea_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_SEA_GREEN.to_brush()
    }

    /// Gets a brush with the `MediumSlateBlue` color.
    pub fn medium_slate_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_SLATE_BLUE.to_brush()
    }

    /// Gets a brush with the `MediumSpringGreen` color.
    pub fn medium_spring_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_SPRING_GREEN.to_brush()
    }

    /// Gets a brush with the `MediumTurquoise` color.
    pub fn medium_turquoise() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_TURQUOISE.to_brush()
    }

    /// Gets a brush with the `MediumVioletRed` color.
    pub fn medium_violet_red() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MEDIUM_VIOLET_RED.to_brush()
    }

    /// Gets a brush with the `MidnightBlue` color.
    pub fn midnight_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MIDNIGHT_BLUE.to_brush()
    }

    /// Gets a brush with the `MintCream` color.
    pub fn mint_cream() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MINT_CREAM.to_brush()
    }

    /// Gets a brush with the `MistyRose` color.
    pub fn misty_rose() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MISTY_ROSE.to_brush()
    }

    /// Gets a brush with the `Moccasin` color.
    pub fn moccasin() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::MOCCASIN.to_brush()
    }

    /// Gets a brush with the `NavajoWhite` color.
    pub fn navajo_white() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::NAVAJO_WHITE.to_brush()
    }

    /// Gets a brush with the `Navy` color.
    pub fn navy() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::NAVY.to_brush()
    }

    /// Gets a brush with the `OldLace` color.
    pub fn old_lace() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::OLD_LACE.to_brush()
    }

    /// Gets a brush with the `Olive` color.
    pub fn olive() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::OLIVE.to_brush()
    }

    /// Gets a brush with the `OliveDrab` color.
    pub fn olive_drab() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::OLIVE_DRAB.to_brush()
    }

    /// Gets a brush with the `Orange` color.
    pub fn orange() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::ORANGE.to_brush()
    }

    /// Gets a brush with the `OrangeRed` color.
    pub fn orange_red() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::ORANGE_RED.to_brush()
    }

    /// Gets a brush with the `Orchid` color.
    pub fn orchid() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::ORCHID.to_brush()
    }

    /// Gets a brush with the `PaleGoldenrod` color.
    pub fn pale_goldenrod() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PALE_GOLDENROD.to_brush()
    }

    /// Gets a brush with the `PaleGreen` color.
    pub fn pale_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PALE_GREEN.to_brush()
    }

    /// Gets a brush with the `PaleTurquoise` color.
    pub fn pale_turquoise() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PALE_TURQUOISE.to_brush()
    }

    /// Gets a brush with the `PaleVioletRed` color.
    pub fn pale_violet_red() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PALE_VIOLET_RED.to_brush()
    }

    /// Gets a brush with the `PapayaWhip` color.
    pub fn papaya_whip() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PAPAYA_WHIP.to_brush()
    }

    /// Gets a brush with the `PeachPuff` color.
    pub fn peach_puff() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PEACH_PUFF.to_brush()
    }

    /// Gets a brush with the `Peru` color.
    pub fn peru() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PERU.to_brush()
    }

    /// Gets a brush with the `Pink` color.
    pub fn pink() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PINK.to_brush()
    }

    /// Gets a brush with the `Plum` color.
    pub fn plum() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PLUM.to_brush()
    }

    /// Gets a brush with the `PowderBlue` color.
    pub fn powder_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::POWDER_BLUE.to_brush()
    }

    /// Gets a brush with the `Purple` color.
    pub fn purple() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::PURPLE.to_brush()
    }

    /// Gets a brush with the `Red` color.
    pub fn red() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::RED.to_brush()
    }

    /// Gets a brush with the `RosyBrown` color.
    pub fn rosy_brown() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::ROSY_BROWN.to_brush()
    }

    /// Gets a brush with the `RoyalBlue` color.
    pub fn royal_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::ROYAL_BLUE.to_brush()
    }

    /// Gets a brush with the `SaddleBrown` color.
    pub fn saddle_brown() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SADDLE_BROWN.to_brush()
    }

    /// Gets a brush with the `Salmon` color.
    pub fn salmon() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SALMON.to_brush()
    }

    /// Gets a brush with the `SandyBrown` color.
    pub fn sandy_brown() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SANDY_BROWN.to_brush()
    }

    /// Gets a brush with the `SeaGreen` color.
    pub fn sea_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SEA_GREEN.to_brush()
    }

    /// Gets a brush with the `SeaShell` color.
    pub fn sea_shell() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SEA_SHELL.to_brush()
    }

    /// Gets a brush with the `Sienna` color.
    pub fn sienna() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SIENNA.to_brush()
    }

    /// Gets a brush with the `Silver` color.
    pub fn silver() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SILVER.to_brush()
    }

    /// Gets a brush with the `SkyBlue` color.
    pub fn sky_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SKY_BLUE.to_brush()
    }

    /// Gets a brush with the `SlateBlue` color.
    pub fn slate_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SLATE_BLUE.to_brush()
    }

    /// Gets a brush with the `SlateGray` color.
    pub fn slate_gray() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SLATE_GRAY.to_brush()
    }

    /// Gets a brush with the `Snow` color.
    pub fn snow() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SNOW.to_brush()
    }

    /// Gets a brush with the `SpringGreen` color.
    pub fn spring_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::SPRING_GREEN.to_brush()
    }

    /// Gets a brush with the `SteelBlue` color.
    pub fn steel_blue() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::STEEL_BLUE.to_brush()
    }

    /// Gets a brush with the `Tan` color.
    pub fn tan() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::TAN.to_brush()
    }

    /// Gets a brush with the `Teal` color.
    pub fn teal() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::TEAL.to_brush()
    }

    /// Gets a brush with the `Thistle` color.
    pub fn thistle() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::THISTLE.to_brush()
    }

    /// Gets a brush with the `Tomato` color.
    pub fn tomato() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::TOMATO.to_brush()
    }

    /// Gets a brush with the `Transparent` color.
    pub fn transparent() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::TRANSPARENT.to_brush()
    }

    /// Gets a brush with the `Turquoise` color.
    pub fn turquoise() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::TURQUOISE.to_brush()
    }

    /// Gets a brush with the `Violet` color.
    pub fn violet() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::VIOLET.to_brush()
    }

    /// Gets a brush with the `Wheat` color.
    pub fn wheat() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::WHEAT.to_brush()
    }

    /// Gets a brush with the `White` color.
    pub fn white() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::WHITE.to_brush()
    }

    /// Gets a brush with the `WhiteSmoke` color.
    pub fn white_smoke() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::WHITE_SMOKE.to_brush()
    }

    /// Gets a brush with the `Yellow` color.
    pub fn yellow() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::YELLOW.to_brush()
    }

    /// Gets a brush with the `YellowGreen` color.
    pub fn yellow_green() -> Rc<ImmutableSolidColorBrush> {
        KnownColor::YELLOW_GREEN.to_brush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Colors;

    #[test]
    fn brushes_have_the_colors_of_the_same_name() {
        assert_eq!(Colors::ALICE_BLUE, Brushes::alice_blue().color());
        assert_eq!(Colors::TRANSPARENT, Brushes::transparent().color());
        assert_eq!(Colors::YELLOW_GREEN, Brushes::yellow_green().color());
        assert!(Rc::ptr_eq(&Brushes::red(), &Brushes::red()));
    }
}
