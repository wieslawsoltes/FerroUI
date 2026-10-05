/// Encoding IDs. The meaning depends on the platform; common values are listed here.
///
/// Several names share a value (one per platform), so this is a value type
/// with associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct CmapEncoding(pub u16);

#[allow(non_upper_case_globals)]
impl CmapEncoding {
    // Unicode platform encodings
    pub const Unicode_1_0: CmapEncoding = CmapEncoding(0);
    pub const Unicode_1_1: CmapEncoding = CmapEncoding(1);
    pub const Unicode_ISO_10646: CmapEncoding = CmapEncoding(2);
    pub const Unicode_2_0_BMP: CmapEncoding = CmapEncoding(3);
    pub const Unicode_2_0_full: CmapEncoding = CmapEncoding(4);

    // Macintosh encodings (selected)
    pub const Macintosh_Roman: CmapEncoding = CmapEncoding(0);
    pub const Macintosh_Japanese: CmapEncoding = CmapEncoding(1);
    pub const Macintosh_ChineseTraditional: CmapEncoding = CmapEncoding(2);
    pub const Macintosh_Korean: CmapEncoding = CmapEncoding(3);
    pub const Macintosh_Arabic: CmapEncoding = CmapEncoding(4);
    pub const Macintosh_Hebrew: CmapEncoding = CmapEncoding(5);
    pub const Macintosh_Greek: CmapEncoding = CmapEncoding(6);
    pub const Macintosh_Russian: CmapEncoding = CmapEncoding(7);
    pub const Macintosh_RSymbol: CmapEncoding = CmapEncoding(8);

    // Microsoft encodings
    pub const Microsoft_Symbol: CmapEncoding = CmapEncoding(0);
    /// UCS-2 / UTF-16 (BMP).
    pub const Microsoft_UnicodeBMP: CmapEncoding = CmapEncoding(1);
    pub const Microsoft_ShiftJIS: CmapEncoding = CmapEncoding(2);
    pub const Microsoft_PRChina: CmapEncoding = CmapEncoding(3);
    pub const Microsoft_Big5: CmapEncoding = CmapEncoding(4);
    pub const Microsoft_Wansung: CmapEncoding = CmapEncoding(5);
    pub const Microsoft_Johab: CmapEncoding = CmapEncoding(6);
    /// UTF-32 (format 12).
    pub const Microsoft_UCS4: CmapEncoding = CmapEncoding(10);
}

impl From<u16> for CmapEncoding {
    #[inline]
    fn from(value: u16) -> Self {
        CmapEncoding(value)
    }
}
