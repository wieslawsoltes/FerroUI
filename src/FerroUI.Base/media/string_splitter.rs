//! Splitting of strings on separators that are not nested inside brackets.

use crate::utilities::FormatError;

/// Options for [`split_respecting_brackets`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StringSplitOptions {
    pub remove_empty_entries: bool,
    pub trim_entries: bool,
}

impl StringSplitOptions {
    pub const NONE: Self = Self { remove_empty_entries: false, trim_entries: false };
    pub const REMOVE_EMPTY_ENTRIES: Self = Self { remove_empty_entries: true, trim_entries: false };
    pub const TRIM_ENTRIES: Self = Self { remove_empty_entries: false, trim_entries: true };
}

/// Splits `s` on any of `separators`, ignoring separators nested inside
/// brackets.
///
/// Panics if `opening_bracket` equals `closing_bracket`.
pub fn split_respecting_brackets<'a>(
    s: &'a str,
    separators: &[char],
    opening_bracket: char,
    closing_bracket: char,
    options: StringSplitOptions,
) -> Result<Vec<&'a str>, FormatError> {
    assert!(
        opening_bracket != closing_bracket,
        "Opening bracket and closing bracket cannot be the same character '{opening_bracket}'."
    );

    let mut result = Vec::new();
    let mut depth = 0usize;
    let mut seg_start = 0usize;

    let mut process_segment = |start: usize, end: usize| {
        let mut segment = &s[start..end];
        if options.trim_entries {
            segment = segment.trim();
        }
        if !segment.is_empty() || !options.remove_empty_entries {
            result.push(segment);
        }
    };

    for (i, ch) in s.char_indices() {
        if ch == opening_bracket {
            depth += 1;
        } else if ch == closing_bracket {
            if depth == 0 {
                return Err(FormatError::from_string(format!(
                    "Unmatched closing bracket '{closing_bracket}' at position {i}."
                )));
            }
            depth -= 1;
        } else if separators.contains(&ch) {
            if depth != 0 {
                continue;
            }
            process_segment(seg_start, i);
            seg_start = i + ch.len_utf8();
        }
    }

    if depth != 0 {
        return Err(FormatError::from_string(format!(
            "Unmatched opening bracket '{opening_bracket}' in input string."
        )));
    }

    // last segment
    process_segment(seg_start, s.len());
    Ok(result)
}
