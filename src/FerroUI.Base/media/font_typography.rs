//! Typed choices for the inherited OpenType typography properties.
//! Exact source declarations and the font-dependent behavior boundary are
//! recorded in docs/porting/winui-rich-text.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontEastAsianLanguage {
    #[default]
    Normal = 0,
    HojoKanji = 1,
    Jis04 = 2,
    Jis78 = 3,
    Jis83 = 4,
    Jis90 = 5,
    NlcKanji = 6,
    Simplified = 7,
    Traditional = 8,
    TraditionalNames = 9,
}

crate::ferro_markup_enum!(FontEastAsianLanguage {
    Normal, HojoKanji, Jis04, Jis78, Jis83, Jis90, NlcKanji, Simplified, Traditional, TraditionalNames
}, { namespace: "FerroUI.Media" });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontEastAsianWidths {
    #[default]
    Normal = 0,
    Full = 1,
    Half = 2,
    Proportional = 3,
    Quarter = 4,
    Third = 5,
}

crate::ferro_markup_enum!(FontEastAsianWidths {
    Normal, Full, Half, Proportional, Quarter, Third
}, { namespace: "FerroUI.Media" });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontCapitals {
    #[default]
    Normal = 0,
    AllSmallCaps = 1,
    SmallCaps = 2,
    AllPetiteCaps = 3,
    PetiteCaps = 4,
    Unicase = 5,
    Titling = 6,
}

crate::ferro_markup_enum!(FontCapitals {
    Normal, AllSmallCaps, SmallCaps, AllPetiteCaps, PetiteCaps, Unicase, Titling
}, { namespace: "FerroUI.Media" });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontFraction {
    #[default]
    Normal = 0,
    Stacked = 1,
    Slashed = 2,
}

crate::ferro_markup_enum!(FontFraction {
    Normal, Stacked, Slashed
}, { namespace: "FerroUI.Media" });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontNumeralStyle {
    #[default]
    Normal = 0,
    Lining = 1,
    OldStyle = 2,
}

crate::ferro_markup_enum!(FontNumeralStyle {
    Normal, Lining, OldStyle
}, { namespace: "FerroUI.Media" });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontNumeralAlignment {
    #[default]
    Normal = 0,
    Proportional = 1,
    Tabular = 2,
}

crate::ferro_markup_enum!(FontNumeralAlignment {
    Normal, Proportional, Tabular
}, { namespace: "FerroUI.Media" });

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum FontVariants {
    #[default]
    Normal = 0,
    Superscript = 1,
    Subscript = 2,
    Ordinal = 3,
    Inferior = 4,
    Ruby = 5,
}

crate::ferro_markup_enum!(FontVariants {
    Normal, Superscript, Subscript, Ordinal, Inferior, Ruby
}, { namespace: "FerroUI.Media" });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typography_enum_values_match_all_38_pinned_constants() {
        assert_eq!(
            [
                FontEastAsianLanguage::Normal as i32,
                FontEastAsianLanguage::HojoKanji as i32,
                FontEastAsianLanguage::Jis04 as i32,
                FontEastAsianLanguage::Jis78 as i32,
                FontEastAsianLanguage::Jis83 as i32,
                FontEastAsianLanguage::Jis90 as i32,
                FontEastAsianLanguage::NlcKanji as i32,
                FontEastAsianLanguage::Simplified as i32,
                FontEastAsianLanguage::Traditional as i32,
                FontEastAsianLanguage::TraditionalNames as i32
            ],
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        );
        assert_eq!(
            [
                FontEastAsianWidths::Normal as i32,
                FontEastAsianWidths::Full as i32,
                FontEastAsianWidths::Half as i32,
                FontEastAsianWidths::Proportional as i32,
                FontEastAsianWidths::Quarter as i32,
                FontEastAsianWidths::Third as i32
            ],
            [0, 1, 2, 3, 4, 5],
        );
        assert_eq!(
            [
                FontCapitals::Normal as i32,
                FontCapitals::AllSmallCaps as i32,
                FontCapitals::SmallCaps as i32,
                FontCapitals::AllPetiteCaps as i32,
                FontCapitals::PetiteCaps as i32,
                FontCapitals::Unicase as i32,
                FontCapitals::Titling as i32
            ],
            [0, 1, 2, 3, 4, 5, 6],
        );
        assert_eq!(
            [
                FontFraction::Normal as i32,
                FontFraction::Stacked as i32,
                FontFraction::Slashed as i32
            ],
            [0, 1, 2],
        );
        assert_eq!(
            [
                FontNumeralStyle::Normal as i32,
                FontNumeralStyle::Lining as i32,
                FontNumeralStyle::OldStyle as i32
            ],
            [0, 1, 2],
        );
        assert_eq!(
            [
                FontNumeralAlignment::Normal as i32,
                FontNumeralAlignment::Proportional as i32,
                FontNumeralAlignment::Tabular as i32
            ],
            [0, 1, 2],
        );
        assert_eq!(
            [
                FontVariants::Normal as i32,
                FontVariants::Superscript as i32,
                FontVariants::Subscript as i32,
                FontVariants::Ordinal as i32,
                FontVariants::Inferior as i32,
                FontVariants::Ruby as i32
            ],
            [0, 1, 2, 3, 4, 5],
        );
    }
}
