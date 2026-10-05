use std::rc::Rc;
use std::str::FromStr;

use crate::utilities::FormatError;

/// The `UnicodeRange` class is used to specify which specific characters
/// should be considered in a font fallback.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnicodeRange {
    single: UnicodeRangeSegment,
    segments: Option<Rc<[UnicodeRangeSegment]>>,
}

impl UnicodeRange {
    /// Creates a range with a single segment.
    pub fn new(start: i32, end: i32) -> Self {
        Self { single: UnicodeRangeSegment::new(start, end), segments: None }
    }

    /// Creates a range from one segment.
    pub fn from_segment(single: UnicodeRangeSegment) -> Self {
        Self { single, segments: None }
    }

    /// Creates a range from several segments.
    ///
    /// Panics when `segments` is empty.
    pub fn from_segments(segments: Vec<UnicodeRangeSegment>) -> Self {
        if segments.is_empty() {
            panic!("segments must not be empty");
        }

        Self { single: segments[0], segments: Some(Rc::from(segments)) }
    }

    /// The default range, `0-10FFFD`.
    pub fn default_range() -> Self {
        Self::new(0, 0x10FFFD)
    }

    #[allow(dead_code)] // internal accessor kept for parity; used by font collections
    pub(crate) fn single(&self) -> UnicodeRangeSegment {
        self.single
    }

    #[allow(dead_code)] // internal accessor kept for parity; used by font collections
    pub(crate) fn segments(&self) -> Option<&[UnicodeRangeSegment]> {
        self.segments.as_deref()
    }

    /// Determines if given value is in range.
    pub fn is_in_range(&self, value: i32) -> bool {
        match &self.segments {
            None => self.single.is_in_range(value),
            Some(segments) => segments.iter().any(|segment| segment.is_in_range(value)),
        }
    }

    /// Parses a `UnicodeRange`.
    pub fn parse(s: &str) -> Result<UnicodeRange, FormatError> {
        if s.is_empty() {
            return Err(FormatError::new("Could not parse specified Unicode range."));
        }

        let mut parts = s.split(',');

        if !s.contains(',') {
            return Ok(UnicodeRange::from_segment(UnicodeRangeSegment::parse(s)?));
        }

        let segments = parts.by_ref().map(|part| UnicodeRangeSegment::parse(part.trim())).collect::<Result<Vec<_>, _>>()?;

        Ok(UnicodeRange::from_segments(segments))
    }
}

impl Default for UnicodeRange {
    fn default() -> Self {
        Self::default_range()
    }
}

impl FromStr for UnicodeRange {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// One contiguous range of code points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnicodeRangeSegment {
    start: i32,
    end: i32,
}

impl UnicodeRangeSegment {
    pub const fn new(start: i32, end: i32) -> Self {
        Self { start, end }
    }

    /// Get the start of the unicode range.
    pub const fn start(&self) -> i32 {
        self.start
    }

    /// Get the end of the unicode range.
    pub const fn end(&self) -> i32 {
        self.end
    }

    /// Determines if given value is in range.
    pub const fn is_in_range(&self, value: i32) -> bool {
        self.start <= value && value <= self.end
    }

    /// Matches `^(?:[uU]\+)?([0-9a-fA-F][0-9a-fA-F?]{1,5})?$` and returns the
    /// captured hex digits (one hex digit followed by one to five hex digits or
    /// wildcards, or a single hex digit).
    fn match_value(s: &str) -> Option<&str> {
        let value = s.strip_prefix("u+").or_else(|| s.strip_prefix("U+")).unwrap_or(s);
        let bytes = value.as_bytes();

        if bytes.is_empty() || bytes.len() > 6 || !bytes[0].is_ascii_hexdigit() {
            return None;
        }

        if bytes[1..].iter().all(|b| b.is_ascii_hexdigit() || *b == b'?') {
            Some(value)
        } else {
            None
        }
    }

    fn parse_hex(value: &str) -> Result<i32, FormatError> {
        i32::from_str_radix(value, 16)
            .map_err(|_| FormatError::new("Could not parse specified Unicode range segment."))
    }

    /// Parses a `UnicodeRangeSegment`: `U+20`, `U+30??` or `0-10FFFD`.
    pub fn parse(s: &str) -> Result<UnicodeRangeSegment, FormatError> {
        const ERROR: FormatError = FormatError::new("Could not parse specified Unicode range segment.");

        if s.is_empty() {
            return Err(ERROR);
        }

        let mut parts = s.split('-');
        let first = parts.next().unwrap_or("");
        let second = parts.next();

        if parts.next().is_some() {
            return Err(ERROR);
        }

        match second {
            None => {
                // e.g. U+20, U+3F U+30??
                let single = Self::match_value(first).ok_or(ERROR)?;

                if !single.contains('?') {
                    let start = Self::parse_hex(single)?;
                    Ok(UnicodeRangeSegment::new(start, start))
                } else {
                    let start = Self::parse_hex(&single.replace('?', "0"))?;
                    let end = Self::parse_hex(&single.replace('?', "F"))?;
                    Ok(UnicodeRangeSegment::new(start, end))
                }
            }
            Some(second) => {
                let first = Self::match_value(first).ok_or(ERROR)?;
                let second = Self::match_value(second).ok_or(ERROR)?;

                Ok(UnicodeRangeSegment::new(Self::parse_hex(first)?, Self::parse_hex(second)?))
            }
        }
    }
}

impl FromStr for UnicodeRangeSegment {
    type Err = FormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_segments() {
        let range = UnicodeRange::parse("U+0, U+1, U+2, U+3").unwrap();

        let segments = range.segments().expect("the range has segments");

        assert_eq!(segments.iter().map(UnicodeRangeSegment::start).collect::<Vec<_>>(), [0, 1, 2, 3]);
    }

    #[test]
    fn should_parse() {
        for (s, expected_start, expected_end) in
            [("u+00-FF", 0, 255), ("U+00-FF", 0, 255), ("U+00-U+FF", 0, 255), ("U+AB??", 43776, 44031)]
        {
            let segment = UnicodeRangeSegment::parse(s).unwrap();

            assert_eq!(segment.start(), expected_start, "{s}");
            assert_eq!(segment.end(), expected_end, "{s}");
        }
    }

    #[test]
    fn in_range_should_return_false_for_values_outside_range() {
        let segment = UnicodeRangeSegment::new(20, 25);

        for value in [0, 19, 26, 100] {
            assert!(!segment.is_in_range(value));
        }
    }

    #[test]
    fn in_range_should_return_true_for_values_within_range() {
        let segment = UnicodeRangeSegment::new(20, 22);

        for value in [20, 21, 22] {
            assert!(segment.is_in_range(value));
        }
    }

    #[test]
    fn malformed_ranges_are_errors() {
        for s in ["", "U+", "U+G", "U+1-2-3", "U+1234567", "x"] {
            assert!(UnicodeRangeSegment::parse(s).is_err(), "{s}");
        }

        assert!(UnicodeRange::parse("").is_err());
        assert!(UnicodeRange::parse("U+1, x").is_err());
        assert!(UnicodeRange::default().is_in_range(0x10FFFD));
        assert!(UnicodeRange::parse("U+41").unwrap().is_in_range(0x41));
    }
}
