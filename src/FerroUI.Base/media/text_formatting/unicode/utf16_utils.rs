/// Helpers for UTF-16 text.
pub(crate) struct Utf16Utils;

#[allow(dead_code)] // used by input method code upstream, which is not ported yet
impl Utf16Utils {
    /// Converts an offset counted in characters (surrogate pairs count once)
    /// into an offset counted in UTF-16 code units.
    ///
    /// Panics when the offset is out of range and `throw_on_out_of_range` is
    /// set (upstream throws `IndexOutOfRangeException`).
    pub(crate) fn character_offset_to_string_offset(s: &[u16], off: usize, throw_on_out_of_range: bool) -> usize {
        if off == 0 {
            return 0;
        }

        let mut symbol_offset = 0;

        for c in 0..s.len() {
            if symbol_offset == off {
                return c;
            }

            if !is_surrogate_pair(s, c) {
                symbol_offset += 1;
            }
        }

        if throw_on_out_of_range {
            panic!("index out of range");
        }

        s.len()
    }
}

/// `char.IsSurrogatePair(string, int)`.
fn is_surrogate_pair(s: &[u16], index: usize) -> bool {
    index + 1 < s.len() && (0xD800..=0xDBFF).contains(&s[index]) && (0xDC00..=0xDFFF).contains(&s[index + 1])
}
