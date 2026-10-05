/// cmap format types.
///
/// The value comes straight from font data, so any 16 bit value is
/// representable; the named values are associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct CmapFormat(pub u16);

#[allow(non_upper_case_globals)]
impl CmapFormat {
    /// Byte encoding table.
    pub const Format0: CmapFormat = CmapFormat(0);
    /// High-byte mapping through table (multi-byte charsets).
    pub const Format2: CmapFormat = CmapFormat(2);
    /// Segment mapping to delta values (most common).
    pub const Format4: CmapFormat = CmapFormat(4);
    /// Trimmed table mapping.
    pub const Format6: CmapFormat = CmapFormat(6);
    /// Mixed 16/32-bit coverage.
    pub const Format8: CmapFormat = CmapFormat(8);
    /// Trimmed array mapping (32-bit).
    pub const Format10: CmapFormat = CmapFormat(10);
    /// Segmented coverage (32-bit).
    pub const Format12: CmapFormat = CmapFormat(12);
    /// Many-to-one mappings.
    pub const Format13: CmapFormat = CmapFormat(13);
    /// Unicode Variation Sequences.
    pub const Format14: CmapFormat = CmapFormat(14);
}

impl From<u16> for CmapFormat {
    #[inline]
    fn from(value: u16) -> Self {
        CmapFormat(value)
    }
}
