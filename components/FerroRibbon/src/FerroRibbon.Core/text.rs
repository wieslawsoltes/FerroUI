//! Shared immutable UTF-16 storage for source-observable text units and wire data.
//! Native UTF-8 conversion is fallible; malformed units are never silently replaced.
use std::{string::FromUtf16Error, sync::Arc};

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RibbonString(Arc<[u16]>);
ferroui_system::styled_value!(RibbonString);
impl RibbonString {
    pub fn from_utf16(units: impl Into<Arc<[u16]>>) -> Self {
        Self(units.into())
    }
    pub fn as_utf16(&self) -> &[u16] {
        &self.0
    }
    pub fn to_utf8(&self) -> Result<String, FromUtf16Error> {
        String::from_utf16(&self.0)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl From<&str> for RibbonString {
    fn from(value: &str) -> Self {
        Self(value.encode_utf16().collect::<Vec<_>>().into())
    }
}
impl From<String> for RibbonString {
    fn from(value: String) -> Self {
        value.as_str().into()
    }
}
impl From<&[u16]> for RibbonString {
    fn from(value: &[u16]) -> Self {
        Self(value.into())
    }
}
pub(crate) fn whitespace(unit: u16) -> bool {
    matches!(unit, 0x0009..=0x000d | 0x0020 | 0x0085 | 0x00a0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000)
}
/// Trims CLR `char.IsWhiteSpace` units using the pinned runtime table.
pub fn trim(value: &[u16]) -> &[u16] {
    let start = value
        .iter()
        .position(|&c| !whitespace(c))
        .unwrap_or(value.len());
    let end = value
        .iter()
        .rposition(|&c| !whitespace(c))
        .map_or(start, |i| i + 1);
    &value[start..end]
}
pub(crate) fn lower_ascii(unit: u16) -> u16 {
    // All BMP mappings whose invariant lowercase result is ASCII. Verified against
    // all 65,536 char values in the pinned runtime by the icon corpus.
    match unit {
        0x41..=0x5a => unit + 0x20,
        0x212a => b'k' as u16,
        _ => unit,
    }
}
pub(crate) fn key_is(value: &[u16], ascii_lower: &str) -> bool {
    value.len() == ascii_lower.len()
        && value
            .iter()
            .zip(ascii_lower.bytes())
            .all(|(&u, b)| lower_ascii(u) == b as u16)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn temporary_gesture_text_is_not_rooted_by_intern_caches() {
        use crate::commands::RibbonKeyGesture;
        for input in ["Esc", "PgUp", "Delete"] {
            let input: RibbonString = input.into();
            let gesture = RibbonKeyGesture::parse(Some(&input)).unwrap();
            drop(input);
            let key = gesture.key.as_ref().unwrap();
            let weak_key = Arc::downgrade(&key.0);
            let display = gesture.to_display_string(false);
            let weak_display = Arc::downgrade(&display.0);
            drop(display);
            drop(gesture);
            assert!(
                weak_key.upgrade().is_none(),
                "canonical gesture key retained"
            );
            assert!(weak_display.upgrade().is_none(), "display alias retained");
        }
        // Empty storage follows std Arc's policy, including shared empty
        // allocations. There is no port-owned singleton or identity contract.
        assert!(empty().is_empty());
    }
    #[test]
    fn all_bmp_whitespace_and_ascii_lower_aliases_match_pinned_runtime() {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/icons.json")).unwrap();
        let aliases = (0..=u16::MAX)
            .filter_map(|u| {
                let lower = lower_ascii(u);
                (lower < 128 && lower != u).then_some([u, lower])
            })
            .collect::<Vec<_>>();
        assert_eq!(serde_json::json!(aliases), source["lowerAsciiAliases"]);
        let spaces = (0..=u16::MAX)
            .filter(|&u| whitespace(u))
            .collect::<Vec<_>>();
        assert_eq!(serde_json::json!(spaces), source["whitespace"]);
    }
}

pub(crate) mod ordinal;

// Shared pinned char tables also used by gesture parsing; char casing processes one
// UTF-16 code unit and is distinct from invariant string/scalar casing.
pub(crate) mod unicode_data;
/// CLR `char.IsDigit`, operating on one UTF-16 unit rather than a Unicode scalar.
pub fn digit(unit: u16) -> bool {
    let ranges = unicode_data::DIGITS;
    let at = ranges.partition_point(|&(start, _)| start <= unit);
    at > 0 && unit <= ranges[at - 1].1
}
/// Pinned CLR OrdinalIgnoreCase comparison with an ASCII string.
pub fn ordinal_equals_ascii(value: &[u16], ascii: &str) -> bool {
    assert!(ascii.is_ascii());
    ordinal::equals_ascii_units(value, ascii)
}
/// Uppercase valid native text using the same pinned scalar casing table as
/// ribbon keyboard gestures. CAD command echoes require simple invariant casing,
/// including Greek precomposed characters; Rust's full case expansion differs.
pub fn invariant_uppercase(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            let scalar = character as u32;
            let upper = unicode_data::UPPER
                .binary_search_by_key(&scalar, |&(from, _)| from)
                .map_or(scalar, |index| unicode_data::UPPER[index].1);
            char::from_u32(upper).expect("validated source scalar casing table")
        })
        .collect()
}
pub(crate) fn upper_char(unit: u16) -> u16 {
    unicode_data::CHAR_UPPER
        .binary_search_by_key(&unit, |&(from, _)| from)
        .map_or(unit, |i| unicode_data::CHAR_UPPER[i].1)
}
pub(crate) fn letter_or_digit(unit: u16) -> bool {
    fn in_ranges(unit: u16, ranges: &[(u16, u16)]) -> bool {
        let at = ranges.partition_point(|&(start, _)| start <= unit);
        at > 0 && unit <= ranges[at - 1].1
    }
    in_ranges(unit, unicode_data::LETTERS) || in_ranges(unit, unicode_data::DIGITS)
}
pub(crate) fn empty() -> RibbonString {
    RibbonString::default()
}

/// Shared pinned CLR ordinal-ignore-case key for the native library boundary.
#[doc(hidden)]
pub use ordinal::OrdinalKey as RibbonOrdinalIgnoreCaseKey;
