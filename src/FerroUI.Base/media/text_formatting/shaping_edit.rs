//! Shaping a text again after an edit, keeping the glyphs of the old shaping that the edit leaves
//! as they were (step 7 of docs/porting/text-layout-performance.md).
//!
//! The shaper records for every glyph whether the text may be cut at its cluster and the two
//! sides shaped apart (HarfBuzz's unsafe-to-break and unsafe-to-concat flags,
//! [`ShapedBuffer::concat_flags`]). The new text is compared with the old one; the glyphs before
//! the last cluster start ahead of the change that is clear of both flags are kept, the new text is
//! shaped from there (with the text around it as context) a little past the change, and the old
//! glyphs are taken again from the first cluster start past the change at which both shapings are
//! clear. That is HarfBuzz's own procedure for reshaping a piece of a text (`hb-buffer.h`,
//! `HB_GLYPH_FLAG_UNSAFE_TO_CONCAT`): the glyphs are those a shaping of the whole new text gives,
//! which `shaping_edit_tests` checks for random edits of texts in several fonts.

use std::rc::Rc;

use crate::media::text_formatting::{GlyphInfo, ShapedBuffer, TextShaper, TextShaperOptions};
use crate::utilities::ReadOnlyMemory;

/// How far past the end of the changed range the new text is shaped at first, in code units: the
/// shaper reports no flag at the end of a text, so the piece is shaped further and the junction is
/// found inside it. Doubled when no junction is found.
const OVERLAP: usize = 32;

/// The texts the run cache keeps the shaping of for an edit: at least this many code units. A
/// shorter text is shaped again at no cost worth keeping its glyphs for.
pub(crate) const MIN_KEPT_LENGTH: usize = 256;

fn clear(flags: &[u8], glyph: usize) -> bool {
    flags[glyph] & (ShapedBuffer::UNSAFE_TO_BREAK | ShapedBuffer::UNSAFE_TO_CONCAT) == 0
}

fn starts_cluster(glyphs: &[GlyphInfo], glyph: usize) -> bool {
    glyph == 0 || glyphs[glyph].glyph_cluster != glyphs[glyph - 1].glyph_cluster
}

/// Whether `text` is a whole text, not a range of a longer one: the context of its shaping is
/// then the text itself on both shapings.
fn is_whole(text: &ReadOnlyMemory<u16>) -> bool {
    match text.owner() {
        Some(owner) => text.offset_in_owner() == 0 && text.len() == owner.len(),
        None => true,
    }
}

/// How many code units are compared at once: a long text that an edit leaves mostly as it was is
/// compared as memory, and only the piece with the first difference unit by unit.
const COMPARED_AT_ONCE: usize = 64;

/// The length of the common start of `a` and `b`.
fn common_prefix(a: &[u16], b: &[u16]) -> usize {
    let limit = a.len().min(b.len());
    let mut at = 0;
    while at + COMPARED_AT_ONCE <= limit && a[at..at + COMPARED_AT_ONCE] == b[at..at + COMPARED_AT_ONCE] {
        at += COMPARED_AT_ONCE;
    }
    at + a[at..limit].iter().zip(&b[at..limit]).take_while(|(x, y)| x == y).count()
}

/// The length of the common end of `a` and `b`, at most `limit`.
fn common_suffix(a: &[u16], b: &[u16], limit: usize) -> usize {
    let (a, b) = (&a[a.len() - limit..], &b[b.len() - limit..]);
    let mut left = limit;
    while left >= COMPARED_AT_ONCE && a[left - COMPARED_AT_ONCE..left] == b[left - COMPARED_AT_ONCE..left] {
        left -= COMPARED_AT_ONCE;
    }
    limit - left + a[..left].iter().rev().zip(b[..left].iter().rev()).take_while(|(x, y)| x == y).count()
}

/// `new_text` shaped with `options`, from `old`, the shaping of the text before an edit with the
/// same options: the glyphs a shaping of the whole of `new_text` gives, with their concatenation
/// flags. `None` when the edit cannot keep glyphs of `old`: a right-to-left shaping, a text that is
/// a range of a longer one, a shaping without flags, or a change that reaches the start of the
/// text; the caller shapes the text whole.
pub(crate) fn shape_edited(
    old: &ShapedBuffer,
    new_text: &ReadOnlyMemory<u16>,
    options: &TextShaperOptions,
    shaper: &TextShaper,
) -> Option<Rc<ShapedBuffer>> {
    if options.bidi_level() & 1 != 0 || !old.is_left_to_right() || !is_whole(old.text()) || !is_whole(new_text)
    {
        return None;
    }
    let old_flags = old.concat_flags()?;
    let old_glyphs = old.glyph_infos();
    let (a, b) = (old.text().span(), new_text.span());
    let (old_length, new_length) = (a.len(), b.len());
    let count = old_glyphs.len();
    if count == 0 || old_flags.len() != count {
        return None;
    }

    // The changed range: old [prefix, old_length - suffix), new [prefix, new_length - suffix).
    let common = old_length.min(new_length);
    let prefix = common_prefix(a, b);
    let suffix = common_suffix(&a[prefix..], &b[prefix..], common - prefix);
    let shift = new_length as isize - old_length as isize;
    let cluster = |glyph: usize| old_glyphs[glyph].glyph_cluster as usize;

    // The start: the last cluster start at or before the change that is clear in the old shaping.
    let mut start = old_glyphs.partition_point(|glyph| glyph.glyph_cluster as usize <= prefix).saturating_sub(1);
    let options = options.clone().with_concat_flags(true);
    loop {
        while start > 0 && !(starts_cluster(&old_glyphs, start) && clear(&old_flags, start)) {
            start -= 1;
        }
        if start == 0 {
            return None;
        }
        let from = cluster(start);
        let change_end = new_length - suffix;
        let mut until = new_length.min(change_end + OVERLAP);
        loop {
            let piece = shaper.shape_text(&new_text.slice(from, until - from), &options);
            let piece_flags = piece.concat_flags()?;
            let piece_glyphs = piece.glyph_infos();
            if piece_glyphs.is_empty() || !clear(&piece_flags, 0) {
                // The new piece is not clear at its start: start further back.
                start -= 1;
                break;
            }
            // The end: the first cluster start past the change at which both shapings are clear.
            let mut junction = None;
            for glyph in 0..piece_glyphs.len() {
                let at = from + piece_glyphs[glyph].glyph_cluster as usize;
                if at < change_end || !starts_cluster(&piece_glyphs, glyph) || !clear(&piece_flags, glyph) {
                    continue;
                }
                let old_at = (at as isize - shift) as usize;
                let old_glyph = old_glyphs.partition_point(|g| (g.glyph_cluster as usize) < old_at);
                if old_glyph < count
                    && cluster(old_glyph) == old_at
                    && starts_cluster(&old_glyphs, old_glyph)
                    && clear(&old_flags, old_glyph)
                {
                    junction = Some((glyph, old_glyph));
                    break;
                }
            }
            let (piece_end, old_from) = match junction {
                Some(found) => found,
                None if until == new_length => (piece_glyphs.len(), count),
                None => {
                    until = new_length.min(until + (until - from).max(OVERLAP));
                    continue;
                }
            };

            let mut glyphs = Vec::with_capacity(start + piece_end + (count - old_from));
            let mut flags = Vec::with_capacity(glyphs.capacity());
            glyphs.extend_from_slice(&old_glyphs[..start]);
            flags.extend_from_slice(&old_flags[..start]);
            glyphs.extend_from_slice(&piece_glyphs[..piece_end]);
            for info in &mut glyphs[start..] {
                info.glyph_cluster += from as i32;
            }
            flags.extend_from_slice(&piece_flags[..piece_end]);
            let tail = glyphs.len();
            glyphs.extend_from_slice(&old_glyphs[old_from..]);
            if shift != 0 {
                for info in &mut glyphs[tail..] {
                    info.glyph_cluster = (info.glyph_cluster as isize + shift) as i32;
                }
            }
            flags.extend_from_slice(&old_flags[old_from..]);
            let shaped =
                ShapedBuffer::from_glyphs_of_edit(new_text.clone(), glyphs, old, start, start + piece_end, old_from);
            shaped.set_concat_flags(flags.into());
            return Some(shaped);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{common_prefix, common_suffix};

    #[test]
    fn the_common_start_and_end_of_two_texts() {
        let text = |length: usize, changed: &[usize]| -> Vec<u16> {
            (0..length).map(|at| if changed.contains(&at) { 1 } else { 7 + (at % 5) as u16 }).collect()
        };
        for length in [0, 1, 63, 64, 65, 200, 1_000] {
            let a = text(length, &[]);
            assert_eq!(common_prefix(&a, &a), length);
            assert_eq!(common_suffix(&a, &a, length), length);
            for at in [0, 1, 62, 63, 64, 65, 127, 128, 199, 999] {
                if at >= length {
                    continue;
                }
                let b = text(length, &[at]);
                assert_eq!(common_prefix(&a, &b), at, "length {length}, changed at {at}");
                assert_eq!(common_suffix(&a, &b, length), length - at - 1, "length {length}, changed at {at}");
                // The end is looked for in what the start leaves, as `shape_edited` does.
                assert_eq!(common_suffix(&a[at..], &b[at..], length - at), length - at - 1);
            }
        }
        // Texts of different lengths: the shorter one bounds both.
        let (a, b) = (text(300, &[]), text(130, &[]));
        assert_eq!(common_prefix(&a, &b), 130);
        assert_eq!(common_suffix(&a[130..], &b[130..], 0), 0);
    }
}
