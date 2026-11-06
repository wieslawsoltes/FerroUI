use super::{Utf16Text, ordinal_data::ORDINAL_UPPER};
/// .NET OrdinalIgnoreCase equality: ordinal simple casing, without normalization or expansions.
/// Invalid UTF-16 code units compare literally. Version provenance is recorded in docs/DEPENDENCIES.md.
pub fn ordinal_ignore_case_equal(left: &Utf16Text, right: &Utf16Text) -> bool {
    ordinal_ignore_case_units(left.units(), right.units())
}

/// Borrowed text comparison for keyword searches; slices may end inside a surrogate pair.
pub(crate) fn ordinal_ignore_case_units(left: &[u16], right: &[u16]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    if left == right {
        return true;
    }
    if left.iter().chain(right).all(|&unit| unit <= 0x7f) {
        return left
            .iter()
            .zip(right)
            .all(|(&a, &b)| (a as u8).eq_ignore_ascii_case(&(b as u8)));
    }
    char::decode_utf16(left.iter().copied())
        .map(upper)
        .eq(char::decode_utf16(right.iter().copied()).map(upper))
}

fn upper(c: Result<char, std::char::DecodeUtf16Error>) -> u32 {
    let cp = match c {
        Ok(c) => c as u32,
        Err(e) => return e.unpaired_surrogate() as u32,
    };
    ORDINAL_UPPER
        .binary_search_by_key(&cp, |p| p.0)
        .map_or(cp, |i| ORDINAL_UPPER[i].1)
}

/// Hash key with the same equality as OrdinalIgnoreCase; retains the first inserted spelling.
#[derive(Clone, Debug)]
pub struct OrdinalKey(pub Utf16Text);
impl PartialEq for OrdinalKey {
    fn eq(&self, other: &Self) -> bool {
        ordinal_ignore_case_equal(&self.0, &other.0)
    }
}
impl Eq for OrdinalKey {}
impl std::hash::Hash for OrdinalKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for cp in char::decode_utf16(self.0.units().iter().copied()).map(upper) {
            cp.hash(state);
        }
    }
}
