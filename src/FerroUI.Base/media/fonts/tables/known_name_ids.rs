// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

/// Provides the well known name ids of the `name` table.
///
/// The value comes straight from font data, so any 16 bit value is
/// representable; the named values are associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct KnownNameIds(pub u16);

#[allow(non_upper_case_globals)]
impl KnownNameIds {
    /// The copyright notice.
    pub const CopyrightNotice: KnownNameIds = KnownNameIds(0);
    /// The font family name; up to four fonts can share the font family name.
    pub const FontFamilyName: KnownNameIds = KnownNameIds(1);
    /// The font subfamily name; distinguishes the fonts in a group with the same font family name.
    pub const FontSubfamilyName: KnownNameIds = KnownNameIds(2);
    /// The unique font identifier.
    pub const UniqueFontID: KnownNameIds = KnownNameIds(3);
    /// The full font name; a combination of the family and subfamily names.
    pub const FullFontName: KnownNameIds = KnownNameIds(4);
    /// The version string.
    pub const Version: KnownNameIds = KnownNameIds(5);
    /// The PostScript name for the font.
    pub const PostscriptName: KnownNameIds = KnownNameIds(6);
    /// The trademark.
    pub const Trademark: KnownNameIds = KnownNameIds(7);
    /// The manufacturer.
    pub const Manufacturer: KnownNameIds = KnownNameIds(8);
    /// The designer.
    pub const Designer: KnownNameIds = KnownNameIds(9);
    /// The description.
    pub const Description: KnownNameIds = KnownNameIds(10);
    /// The vendor url.
    pub const VendorUrl: KnownNameIds = KnownNameIds(11);
    /// The designer url.
    pub const DesignerUrl: KnownNameIds = KnownNameIds(12);
    /// The license description.
    pub const LicenseDescription: KnownNameIds = KnownNameIds(13);
    /// The license info url.
    pub const LicenseInfoUrl: KnownNameIds = KnownNameIds(14);
    /// The typographic family name.
    pub const TypographicFamilyName: KnownNameIds = KnownNameIds(16);
    /// The typographic subfamily name.
    pub const TypographicSubfamilyName: KnownNameIds = KnownNameIds(17);
    /// The sample text.
    pub const SampleText: KnownNameIds = KnownNameIds(19);
}

impl From<u16> for KnownNameIds {
    #[inline]
    fn from(value: u16) -> Self {
        KnownNameIds(value)
    }
}
