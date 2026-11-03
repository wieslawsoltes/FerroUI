use super::{
    Utf16Text,
    casing_data::{CULTURES, UPPER_TABLES},
};
/// Explicit core casing bridge for cultures measured against the pinned .NET runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextCulture {
    name: &'static str,
    profile: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsupportedCulture(pub String);
impl std::fmt::Display for UnsupportedCulture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported text culture: {:?}", self.0)
    }
}
impl std::error::Error for UnsupportedCulture {}
impl TextCulture {
    /// Resolve the current host locale only when an operation requires casing.
    pub fn current() -> Result<Self, super::TextCaseError> {
        let name = super::locale_text::current_locale_name()
            .map_err(super::TextCaseError::CurrentCulture)?;
        Self::get(&name).map_err(super::TextCaseError::UnsupportedCulture)
    }
    pub fn invariant() -> Self {
        Self::get("").expect("invariant table is generated")
    }
    pub fn get(name: &str) -> Result<Self, UnsupportedCulture> {
        CULTURES
            .iter()
            .find(|(known, _)| known.eq_ignore_ascii_case(name))
            .map(|(name, profile)| Self {
                name,
                profile: *profile,
            })
            .ok_or_else(|| UnsupportedCulture(name.into()))
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn to_upper(&self, text: &Utf16Text) -> Utf16Text {
        // Source TextInfo returns the original only for empty strings or its unchanged
        // invariant-compatible ASCII fast path. Localization paths allocate even if all
        // resulting units happen to be equal to the input (including unchanged ligatures).
        if text.is_empty()
            || (super::ascii_casing_data::NON_INVARIANT_ASCII_CASING
                .binary_search(&self.name)
                .is_err()
                && text.units().iter().all(|unit| {
                    *unit <= 0x7f && !(u16::from(b'a')..=u16::from(b'z')).contains(unit)
                }))
        {
            return text.clone();
        }
        map_case(text, UPPER_TABLES[self.profile], false)
    }
    pub fn to_lower_invariant(text: &Utf16Text) -> Utf16Text {
        map_case(text, super::lowercase_data::LOWER, true)
    }
    /// A single keyboard code unit, without allocating a temporary UTF-16 string.
    /// Surrogates remain individual units; this never performs scalar-pair decoding.
    pub(crate) fn lower_unit_invariant(unit: u16) -> u16 {
        if unit <= 0x7f {
            return u16::from((unit as u8).to_ascii_lowercase());
        }
        let table = super::lowercase_data::LOWER;
        table
            .binary_search_by_key(&u32::from(unit), |&(from, _)| from)
            .map_or(unit, |i| {
                u16::try_from(table[i].1).expect("single-unit lowercase mapping stays in the BMP")
            })
    }
}
fn map_case(text: &Utf16Text, table: &[(u32, u32)], retain_unchanged: bool) -> Utf16Text {
    let mut result = Vec::with_capacity(text.len());
    for item in char::decode_utf16(text.units().iter().copied()) {
        match item {
            Ok(c) => {
                let cp = c as u32;
                let upper = table
                    .binary_search_by_key(&cp, |(a, _)| *a)
                    .map_or(cp, |index| table[index].1);
                let mut units = [0; 2];
                result.extend_from_slice(
                    char::from_u32(upper)
                        .expect("validated generated scalar")
                        .encode_utf16(&mut units),
                );
            }
            Err(e) => result.push(e.unpaired_surrogate()),
        }
    }
    if retain_unchanged && result == text.units() {
        text.clone()
    } else {
        Utf16Text::from_units(result)
    }
}

#[cfg(test)]
mod lower_tests {
    use sha2::{Digest, Sha256};
    #[test]
    fn single_unit_lowercase_matches_source_for_all_utf16_units() {
        let expected = include_bytes!("../../../tests/fixtures/glide-unit-lowercase.bin");
        assert_eq!(expected.len(), 65536 * 2);
        assert_eq!(
            format!("{:x}", Sha256::digest(expected)),
            "0e94cbc3c9ac8c6c6e9de29332958307a3d7ed852066bb5c5a06fd034d1b2f1f"
        );
        for unit in 0..=u16::MAX {
            let at = usize::from(unit) * 2;
            let source = u16::from_le_bytes([expected[at], expected[at + 1]]);
            assert_eq!(
                super::TextCulture::lower_unit_invariant(unit),
                source,
                "U+{unit:04X}"
            );
        }
    }
    #[test]
    fn source_string_is_lower_for_every_unicode_scalar() {
        let flags = (0..0x110000)
            .map(|cp| u8::from(super::super::is_lower_code_point(cp)))
            .collect::<Vec<_>>();
        let row: serde_json::Value = serde_json::from_str(
            include_str!("../../../tests/fixtures/engine-input.jsonl")
                .lines()
                .last()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(flags)), row["sha256"]);
    }
}
