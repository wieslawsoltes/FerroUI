// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

/// Encoding IDs.
///
/// The value comes straight from font data, so any 16 bit value is
/// representable; the named values are associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct EncodingIDs(pub u16);

#[allow(non_upper_case_globals)]
impl EncodingIDs {
    /// Unicode 1.0 semantics.
    pub const Unicode1: EncodingIDs = EncodingIDs(0);
    /// Unicode 1.1 semantics.
    pub const Unicode11: EncodingIDs = EncodingIDs(1);
    /// ISO/IEC 10646 semantics.
    pub const ISO10646: EncodingIDs = EncodingIDs(2);
    /// Unicode 2.0 and onwards semantics, Unicode BMP only (cmap subtable formats 0, 4, 6).
    pub const Unicode2: EncodingIDs = EncodingIDs(3);
    /// Unicode 2.0 and onwards semantics, Unicode full repertoire (cmap subtable formats 0, 4, 6, 10, 12).
    pub const Unicode2Plus: EncodingIDs = EncodingIDs(4);
    /// Unicode Variation Sequences (cmap subtable format 14).
    pub const UnicodeVariationSequences: EncodingIDs = EncodingIDs(5);
    /// Unicode full repertoire (cmap subtable formats 0, 4, 6, 10, 12, 13).
    pub const UnicodeFull: EncodingIDs = EncodingIDs(6);
}

impl From<u16> for EncodingIDs {
    #[inline]
    fn from(value: u16) -> Self {
        EncodingIDs(value)
    }
}
