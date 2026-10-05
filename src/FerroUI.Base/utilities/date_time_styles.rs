//! Options of date and time parsing (.NET `System.Globalization.DateTimeStyles`).

use bitflags::bitflags;

bitflags! {
    /// Options that customise how a date and time string is parsed.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DateTimeStyles: u32 {
        /// Leading white space is allowed (always allowed by `parse`).
        const ALLOW_LEADING_WHITE = 0x01;
        /// Trailing white space is allowed (always allowed by `parse`).
        const ALLOW_TRAILING_WHITE = 0x02;
        /// Extra white space inside the string is allowed.
        const ALLOW_INNER_WHITE = 0x04;
        /// Leading, trailing and inner white space is allowed.
        const ALLOW_WHITE_SPACES = 0x07;
        /// A string with a time but no date gives the date 0001-01-01
        /// instead of the current date.
        const NO_CURRENT_DATE_DEFAULT = 0x08;
        /// The result is converted to UTC.
        const ADJUST_TO_UNIVERSAL = 0x10;
        /// A string without time zone information is taken as local time.
        const ASSUME_LOCAL = 0x20;
        /// A string without time zone information is taken as UTC.
        const ASSUME_UNIVERSAL = 0x40;
        /// The kind stated by the string (`Z` or an offset) is preserved.
        const ROUNDTRIP_KIND = 0x80;
    }
}

impl DateTimeStyles {
    /// The default options (.NET `DateTimeStyles.None`).
    pub const NONE: DateTimeStyles = DateTimeStyles::empty();
}
