//! Segment embedding levels for prepared logical text.
//!
//! This preserves the source algorithm's first-strong paragraph direction and
//! weak, neutral and implicit resolution. It does not perform visual reordering.

use crate::generated::pretext_bidi_data::{LATIN1_BIDI_TYPES, NON_LATIN1_BIDI_RANGES};

const L: i8 = 0;
const R: i8 = 1;
const AL: i8 = 2;
const WS: i8 = 3;
const EN: i8 = 4;
const AN: i8 = 5;
const ES: i8 = 6;
const ET: i8 = 7;
const CS: i8 = 8;
const ON: i8 = 9;
const NSM: i8 = 13;

/// `starts` contains UTF-16 offsets into the normalized text, matching the
/// source contract and the framework shaping boundary.
pub(crate) fn compute_segment_levels(text: &str, starts: &[usize]) -> Option<Vec<i8>> {
    let levels = compute_levels(text)?;
    Some(starts.iter().map(|&start| levels[start]).collect())
}

fn compute_levels(text: &str) -> Option<Vec<i8>> {
    if text.is_empty() {
        return None;
    }
    let mut types = Vec::with_capacity(text.len());
    let mut saw_bidi = false;
    for ch in text.chars() {
        let kind = classify_code_point(ch as u32);
        saw_bidi |= matches!(kind, R | AL | AN);
        types.extend(std::iter::repeat_n(kind, ch.len_utf16()));
    }
    if !saw_bidi {
        return None;
    }
    let start_level = types
        .iter()
        .find_map(|&kind| match kind {
            L => Some(0),
            R | AL => Some(1),
            _ => None,
        })
        .unwrap_or(0);
    let mut levels = vec![start_level; types.len()];
    let embedding = if start_level & 1 == 1 { R } else { L };
    let sor = embedding;

    // W1: nonspacing marks inherit the preceding type.
    let mut last_type = sor;
    for kind in &mut types {
        if *kind == NSM {
            *kind = last_type;
        } else {
            last_type = *kind;
        }
    }
    // W2: European digits following Arabic letters become Arabic numbers.
    last_type = sor;
    for kind in &mut types {
        if *kind == EN {
            if last_type == AL {
                *kind = AN;
            }
        } else if matches!(*kind, R | L | AL) {
            last_type = *kind;
        }
    }
    // W3.
    for kind in &mut types {
        if *kind == AL {
            *kind = R;
        }
    }
    // W4.
    for i in 1..types.len().saturating_sub(1) {
        if types[i] == ES && types[i - 1] == EN && types[i + 1] == EN {
            types[i] = EN;
        }
        if types[i] == CS && matches!(types[i - 1], EN | AN) && types[i + 1] == types[i - 1] {
            types[i] = types[i - 1];
        }
    }
    // W5. Scanning in source order also covers already converted terminators.
    for i in 0..types.len() {
        if types[i] != EN {
            continue;
        }
        let mut j = i;
        while j > 0 && types[j - 1] == ET {
            j -= 1;
            types[j] = EN;
        }
        let mut j = i + 1;
        while j < types.len() && types[j] == ET {
            types[j] = EN;
            j += 1;
        }
    }
    // W6.
    for kind in &mut types {
        if matches!(*kind, WS | ES | ET | CS) {
            *kind = ON;
        }
    }
    // W7.
    last_type = sor;
    for kind in &mut types {
        if *kind == EN {
            if last_type == L {
                *kind = L;
            }
        } else if matches!(*kind, R | L) {
            last_type = *kind;
        }
    }
    // N1: neutral sequences bounded by one direction acquire that direction.
    let mut i = 0;
    while i < types.len() {
        if types[i] != ON {
            i += 1;
            continue;
        }
        let mut end = i + 1;
        while end < types.len() && types[end] == ON {
            end += 1;
        }
        let before = if i > 0 { types[i - 1] } else { sor };
        let after = types.get(end).copied().unwrap_or(sor);
        let before_dir = if before == L { L } else { R };
        let after_dir = if after == L { L } else { R };
        if before_dir == after_dir {
            types[i..end].fill(before_dir);
        }
        i = end;
    }
    // N2.
    for kind in &mut types {
        if *kind == ON {
            *kind = embedding;
        }
    }
    // I1–I2.
    for (level, kind) in levels.iter_mut().zip(types) {
        if *level & 1 == 0 {
            if kind == R {
                *level += 1;
            } else if matches!(kind, AN | EN) {
                *level += 2;
            }
        } else if matches!(kind, L | AN | EN) {
            *level += 1;
        }
    }
    Some(levels)
}

fn classify_code_point(code_point: u32) -> i8 {
    if code_point <= 0xff {
        return LATIN1_BIDI_TYPES[code_point as usize];
    }
    let index = NON_LATIN1_BIDI_RANGES.partition_point(|range| range.1 < code_point);
    NON_LATIN1_BIDI_RANGES
        .get(index)
        .filter(|range| range.0 <= code_point)
        .map_or(L, |range| range.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ranges_are_sorted_and_disjoint() {
        assert!(NON_LATIN1_BIDI_RANGES.windows(2).all(|r| r[0].1 < r[1].0));
        assert!(NON_LATIN1_BIDI_RANGES.iter().all(|r| r.0 <= r.1));
    }

    #[test]
    fn astral_characters_keep_utf16_segment_offsets() {
        assert_eq!(compute_segment_levels("😀שלום", &[0, 2]), Some(vec![1, 1]));
        assert_eq!(compute_segment_levels("𞤀 A", &[0, 3]), Some(vec![1, 2]));
    }
}
