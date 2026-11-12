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
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..]
        .iter()
        .rev()
        .zip(b[prefix..].iter().rev())
        .take(common - prefix)
        .take_while(|(x, y)| x == y)
        .count();
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
            for glyph in 0..piece_end {
                let mut info = piece_glyphs[glyph];
                info.glyph_cluster += from as i32;
                glyphs.push(info);
                flags.push(piece_flags[glyph]);
            }
            for glyph in old_from..count {
                let mut info = old_glyphs[glyph];
                info.glyph_cluster = (info.glyph_cluster as isize + shift) as i32;
                glyphs.push(info);
                flags.push(old_flags[glyph]);
            }
            let shaped = ShapedBuffer::from_glyphs(
                new_text.clone(),
                glyphs,
                old.glyph_typeface().clone(),
                old.font_rendering_em_size(),
                old.bidi_level(),
            );
            shaped.set_concat_flags(flags.into());
            return Some(shaped);
        }
    }
}
