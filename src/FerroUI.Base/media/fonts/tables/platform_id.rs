// Derived from SixLabors.Fonts (Apache License 2.0), see NOTICE.md.

/// Platform ids.
///
/// The value comes straight from font data, so any 16 bit value is
/// representable; the named values are associated constants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct PlatformID(pub u16);

#[allow(non_upper_case_globals)]
impl PlatformID {
    /// Unicode platform.
    pub const Unicode: PlatformID = PlatformID(0);
    /// Script manager code.
    pub const Macintosh: PlatformID = PlatformID(1);
    /// [deprecated] ISO encoding.
    pub const ISO: PlatformID = PlatformID(2);
    /// Window encoding.
    pub const Windows: PlatformID = PlatformID(3);
    /// Custom platform.
    pub const Custom: PlatformID = PlatformID(4);
}

impl From<u16> for PlatformID {
    #[inline]
    fn from(value: u16) -> Self {
        PlatformID(value)
    }
}
