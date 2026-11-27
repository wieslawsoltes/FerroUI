//! The `unicode-range` descriptor (CSS Fonts 4, section 4.5).

use super::CssParseError;

const INVALID: CssParseError = CssParseError::InvalidValue("unicode-range");
const MAX_CODE_POINT: u32 = 0x10FFFF;

/// A set of code points: sorted intervals that neither overlap nor touch, so a lookup is a
/// binary search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnicodeRangeSet {
    ranges: Box<[(u32, u32)]>,
}

impl Default for UnicodeRangeSet {
    /// The initial value of the descriptor, `U+0-10FFFF`.
    fn default() -> Self {
        Self { ranges: Box::new([(0, MAX_CODE_POINT)]) }
    }
}

impl UnicodeRangeSet {
    /// A set of the given inclusive intervals, in any order. Intervals that are reversed or
    /// beyond U+10FFFF are left out.
    pub fn from_ranges(ranges: impl IntoIterator<Item = (u32, u32)>) -> Self {
        let mut ranges: Vec<(u32, u32)> =
            ranges.into_iter().filter(|(start, end)| start <= end && *end <= MAX_CODE_POINT).collect();
        ranges.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(ranges.len());
        for (start, end) in ranges {
            match merged.last_mut() {
                Some(last) if start <= last.1.saturating_add(1) => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }
        Self { ranges: merged.into_boxed_slice() }
    }

    /// The sorted intervals of the set.
    pub fn ranges(&self) -> &[(u32, u32)] {
        &self.ranges
    }

    /// Whether the set includes a code point.
    pub fn contains(&self, code_point: u32) -> bool {
        self.ranges
            .binary_search_by(|&(start, end)| {
                if code_point < start {
                    std::cmp::Ordering::Greater
                } else if code_point > end {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok()
    }

    /// Whether the set includes at least one character of a text.
    pub fn intersects_text(&self, text: &str) -> bool {
        text.chars().any(|c| self.contains(c as u32))
    }
}

/// One `<urange>`: `U+26`, `U+0-7F` or `U+4??`.
fn parse_item(item: &str) -> Result<(u32, u32), CssParseError> {
    let bytes = item.as_bytes();
    if bytes.len() < 3 || !bytes[0].eq_ignore_ascii_case(&b'u') || bytes[1] != b'+' {
        return Err(INVALID);
    }
    let body = &item[2..];
    let hex = |text: &str| -> Result<u32, CssParseError> {
        if text.is_empty() || text.len() > 6 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(INVALID);
        }
        u32::from_str_radix(text, 16).map_err(|_| INVALID)
    };
    let (start, end) = if let Some((first, last)) = body.split_once('-') {
        (hex(first)?, hex(last)?)
    } else if body.contains('?') {
        let digits = body.trim_end_matches('?');
        let wildcards = body.len() - digits.len();
        if body.len() > 6 || digits.contains('?') {
            return Err(INVALID);
        }
        let prefix = if digits.is_empty() { 0 } else { hex(digits)? };
        let shift = 4 * wildcards as u32;
        (prefix << shift, (prefix << shift) | ((1u32 << shift) - 1))
    } else {
        let value = hex(body)?;
        (value, value)
    };
    if start > end || end > MAX_CODE_POINT {
        return Err(INVALID);
    }
    Ok((start, end))
}

/// Removes CSS comments: a comma-separated list of ranges has no strings to protect.
fn without_comments(value: &str) -> std::borrow::Cow<'_, str> {
    if !value.contains("/*") {
        return value.into();
    }
    let mut text = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("/*") {
        text.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + end + 4..],
            None => return text.into(),
        }
    }
    text.push_str(rest);
    text.into()
}

/// Parses the value of `unicode-range`. One invalid range invalidates the descriptor.
pub fn parse_unicode_range(value: &str) -> Result<UnicodeRangeSet, CssParseError> {
    let value = without_comments(value);
    let mut ranges = Vec::new();
    for item in value.split(',') {
        ranges.push(parse_item(item.trim_matches([' ', '\t', '\n', '\r', '\u{c}']))?);
    }
    Ok(UnicodeRangeSet::from_ranges(ranges))
}
