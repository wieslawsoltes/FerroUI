use super::RibbonString;
use std::hash::{Hash, Hasher};
#[path = "ordinal_data.rs"]
mod ordinal_data;

/// Dictionary key retaining the original immutable string without normalizing its public value.
#[derive(Clone, Debug)]
pub struct OrdinalKey(pub RibbonString);
fn folded(value: &[u16]) -> impl Iterator<Item = u32> + '_ {
    let mut input = value.iter().copied().peekable();
    std::iter::from_fn(move || {
        let high = input.next()?;
        let scalar = if (0xd800..=0xdbff).contains(&high)
            && input
                .peek()
                .is_some_and(|low| (0xdc00..=0xdfff).contains(low))
        {
            0x10000
                + ((u32::from(high) - 0xd800) << 10)
                + (u32::from(input.next().unwrap()) - 0xdc00)
        } else {
            u32::from(high)
        };
        Some(if scalar < 128 {
            if (97..=122).contains(&scalar) {
                scalar - 32
            } else {
                scalar
            }
        } else {
            ordinal_data::FOLD
                .binary_search_by_key(&scalar, |&(from, _)| from)
                .map_or(scalar, |index| ordinal_data::FOLD[index].1)
        })
    })
}
pub(crate) fn equals_ascii(value: &RibbonString, ascii: &str) -> bool {
    equals_ascii_units(value.as_utf16(), ascii)
}
pub(super) fn equals_ascii_units(value: &[u16], ascii: &str) -> bool {
    debug_assert!(ascii.is_ascii());
    value.len() == ascii.len()
        && folded(value).eq(ascii.bytes().map(|b| u32::from(b.to_ascii_uppercase())))
}
impl PartialEq for OrdinalKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && folded(self.0.as_utf16()).eq(folded(other.0.as_utf16()))
    }
}
impl Eq for OrdinalKey {}
impl Hash for OrdinalKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for scalar in folded(self.0.as_utf16()) {
            scalar.hash(state);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    #[test]
    fn pinned_ordinal_pairs_preserve_unicode_utf16_and_hash_equivalence() {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/ordinal.json")).unwrap();
        let units = |x: &serde_json::Value| {
            RibbonString::from_utf16(
                x.as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_u64().unwrap() as u16)
                    .collect::<Vec<_>>(),
            )
        };
        for pair in source["pairs"].as_array().unwrap() {
            let a = OrdinalKey(units(&pair["a"]));
            let b = OrdinalKey(units(&pair["b"]));
            assert_eq!(a == b, pair["equal"].as_bool().unwrap(), "{pair}");
            if a == b {
                let mut x = DefaultHasher::new();
                let mut y = DefaultHasher::new();
                a.hash(&mut x);
                b.hash(&mut y);
                assert_eq!(x.finish(), y.finish());
            }
        }
    }
}
