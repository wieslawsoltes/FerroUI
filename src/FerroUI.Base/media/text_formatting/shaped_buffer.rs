use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

use crate::media::text_formatting::{GlyphInfo, SplitResult};
use crate::media::GlyphTypeface;
use crate::utilities::{ArraySlice, MathUtilities, ReadOnlyMemory};

/// The lazily computed cluster-width cache of a shaped buffer.
///
/// `MeasureLength`-style queries and metrics both fold multi-glyph clusters
/// and accumulate per-cluster advances; the result depends only on the shaped
/// glyphs and the text. To make wrap-time splits cheap the arrays are shared
/// by reference across split halves: a buffer produced by `split` points at
/// its parent's arrays and `start_idx` is the offset of its first cluster.
///
/// `prefix[i]` = sum of cluster advances `[0..i)` in logical order over the
/// full source buffer; the width of a sub-buffer is
/// `prefix[start_idx + count] - prefix[start_idx]`. `start_chars[i]` = char
/// offset into the parent's text where the i-th cluster begins.
///
/// Fast path: when every cluster is exactly one character wide, `start_chars`
/// is omitted (`start_chars[k] == k` is implicit).
///
/// Sums in the prefix are in logical (not visual) order, which can differ by
/// ULPs from a visual-order sum for right-to-left buffers.
#[derive(Default)]
struct ClusterCache {
    prefix: Option<Rc<[f64]>>,
    start_chars: Option<Rc<[i32]>>,
    start_idx: usize,
    count: usize,
    /// Generation observed on the shared glyph storage when the cache was built.
    generation: i32,
}

/// A buffer of shaped glyphs: the output of the text shaper for one run.
///
/// Buffers produced by [`ShapedBuffer::split`] and
/// [`ShapedBuffer::with_bidi_level`] share the glyph storage of their source.
pub struct ShapedBuffer {
    text: ReadOnlyMemory<u16>,
    glyph_infos: ArraySlice<GlyphInfo>,
    glyph_indices: ArraySlice<u16>,
    /// Generation counter of the shared glyph storage; bumped by every write
    /// so siblings detect a stale cluster cache. `None` for caller-owned
    /// storage.
    glyph_generation: Option<Rc<Cell<i32>>>,
    glyph_typeface: Rc<GlyphTypeface>,
    font_rendering_em_size: f64,
    bidi_level: i8,
    cache: RefCell<ClusterCache>,
    /// The UAX #14 break opportunities of the text this buffer was shaped for, found once and
    /// shared with the buffers split from it (desktop-performance.md section 9.16).
    breaks: RefCell<Option<Rc<BreakTable>>>,
    /// The concatenation flags of the glyphs, when the shaper recorded them
    /// (`TextShaperOptions::produce_concat_flags`): one byte a glyph, [`Self::UNSAFE_TO_BREAK`]
    /// and [`Self::UNSAFE_TO_CONCAT`]. Only the buffer the shaper made has them, not the buffers
    /// split from it.
    concat_flags: RefCell<Option<Rc<[u8]>>>,
    disposed: Cell<bool>,
}

/// The break opportunities of a text, from its start in the storage it shares.
pub(crate) struct BreakTable {
    start: usize,
    end: usize,
    breaks: Box<[crate::media::text_formatting::unicode::LineBreak]>,
}

impl BreakTable {
    /// How far a rule of UAX #14 looks ahead of the place it decides, in code units, with room to
    /// spare: the old breaks kept before a change end this far ahead of it, and the old breaks
    /// taken again start this far past it.
    const LOOK: usize = 4;

    /// Whether `text` has at `at` a break opportunity no rule looks across: a space between two
    /// ASCII letters or digits. The rules that look back past the unit before a place look for
    /// an opening, a quotation mark, a number or a mark that leans on its base, and stop at the
    /// space or at the letter; the breaks from `at` on are those of the text from `at` alone.
    fn settles(text: &[u16], at: usize) -> bool {
        let word = |unit: u16| matches!(unit, 0x30..=0x39 | 0x41..=0x5A | 0x61..=0x7A);
        at >= 2 && at < text.len() && word(text[at - 2]) && text[at - 1] == 0x20 && word(text[at])
    }

    /// The table of `new`, a text that starts at `start` of its storage, from this table of
    /// `old`, the text before an edit (step 7 of docs/porting/text-layout-performance.md): the
    /// breaks before the change are kept up to a break that settles, the text is enumerated from
    /// there past the change to the next break that settles in both texts, and the breaks of the
    /// old text follow, shifted. The breaks are those of an enumeration of `new` from its start,
    /// which `text_break_table_tests` checks for random edits. `None` when the table is not that
    /// of the whole of `old`.
    fn edited(&self, old: &[u16], new: &[u16], start: usize) -> Option<BreakTable> {
        self.edited_counting(old, new, start).map(|(table, _)| table)
    }

    /// [`Self::edited`], with the count of the code units that were enumerated again.
    fn edited_counting(&self, old: &[u16], new: &[u16], start: usize) -> Option<(BreakTable, usize)> {
        use crate::media::text_formatting::unicode::{LineBreak, LineBreakEnumerator};
        let (old_length, new_length) = (old.len(), new.len());
        if self.end - self.start != old_length || new_length == 0 {
            return None;
        }
        let common = old_length.min(new_length);
        let prefix = old.iter().zip(new).take_while(|(x, y)| x == y).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take(common - prefix)
            .take_while(|(x, y)| x == y)
            .count();
        let change_end = new_length - suffix;
        let shift = new_length as isize - old_length as isize;

        let mut kept = self.breaks.partition_point(|line_break| line_break.position_wrap() + Self::LOOK <= prefix);
        while kept > 0 && !Self::settles(new, self.breaks[kept - 1].position_wrap()) {
            kept -= 1;
        }
        let from = if kept > 0 { self.breaks[kept - 1].position_wrap() } else { 0 };
        let mut breaks = Vec::with_capacity(self.breaks.len() + 16);
        breaks.extend_from_slice(&self.breaks[..kept]);
        let mut enumerator = LineBreakEnumerator::new(&new[from..]);
        let mut enumerated = new_length - from;
        while let Some(line_break) = enumerator.move_next() {
            let at = from + line_break.position_wrap();
            breaks.push(LineBreak::new(from + line_break.position_measure(), at, line_break.required()));
            if at < change_end + Self::LOOK || !Self::settles(new, at) {
                continue;
            }
            let old_at = (at as isize - shift) as usize;
            if let Ok(found) = self.breaks.binary_search_by(|old_break| old_break.position_wrap().cmp(&old_at)) {
                breaks.extend(self.breaks[found + 1..].iter().map(|old_break| {
                    LineBreak::new(
                        (old_break.position_measure() as isize + shift) as usize,
                        (old_break.position_wrap() as isize + shift) as usize,
                        old_break.required(),
                    )
                }));
                enumerated = at - from;
                break;
            }
        }
        Some((BreakTable { start, end: start + new_length, breaks: breaks.into_boxed_slice() }, enumerated))
    }

    /// The table of `text` enumerated from its start, and the table an edit of the table of
    /// `old` gives for it (the tests compare the two).
    #[cfg(test)]
    pub(crate) fn of_edit(old: &[u16], text: &[u16]) -> (Vec<LineBreak>, Vec<LineBreak>) {
        use crate::media::text_formatting::unicode::LineBreakEnumerator;
        let whole = |text: &[u16]| LineBreakEnumerator::new(text).collect::<Vec<_>>();
        let table = BreakTable { start: 0, end: old.len(), breaks: whole(old).into_boxed_slice() };
        let edited = table.edited(old, text, 0).map(|table| table.breaks.into_vec()).unwrap_or_default();
        (whole(text), edited)
    }

    /// How many code units of `text` an edit of the table of `old` enumerates again.
    #[cfg(test)]
    pub(crate) fn edit_enumerates(old: &[u16], text: &[u16]) -> usize {
        use crate::media::text_formatting::unicode::LineBreakEnumerator;
        let breaks = LineBreakEnumerator::new(old).collect::<Vec<_>>().into_boxed_slice();
        let table = BreakTable { start: 0, end: old.len(), breaks };
        table.edited_counting(old, text, 0).map_or(text.len(), |(_, enumerated)| enumerated)
    }
}

#[cfg(test)]
use crate::media::text_formatting::unicode::LineBreak;

/// The break opportunities of a run's text, from the shared table of the text it was split from
/// or found again from its start.
pub(crate) enum RunLineBreaks<'a> {
    Table { table: Rc<BreakTable>, index: usize, shift: usize },
    Enumerator(crate::media::text_formatting::unicode::LineBreakEnumerator<'a>),
}

impl RunLineBreaks<'_> {
    /// The next break opportunity, relative to the start of the run's text.
    pub(crate) fn move_next(&mut self) -> Option<crate::media::text_formatting::unicode::LineBreak> {
        use crate::media::text_formatting::unicode::LineBreak;
        match self {
            RunLineBreaks::Table { table, index, shift } => {
                let line_break = *table.breaks.get(*index)?;
                *index += 1;
                Some(LineBreak::new(
                    line_break.position_measure() - *shift,
                    line_break.position_wrap() - *shift,
                    line_break.required(),
                ))
            }
            RunLineBreaks::Enumerator(enumerator) => enumerator.move_next(),
        }
    }
}

#[cfg(test)]
crate::ferro_thread_local! {
    /// Off: every run finds its breaks again from its start, as upstream does (the tests compare).
    pub(crate) static BREAK_TABLES: Cell<bool> = Cell::new(true);
}

impl ShapedBuffer {
    /// Creates a buffer with room for `buffer_length` glyphs; the shaper fills
    /// it with [`ShapedBuffer::set`].
    pub fn new(
        text: ReadOnlyMemory<u16>,
        buffer_length: usize,
        glyph_typeface: Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        bidi_level: i8,
    ) -> Rc<Self> {
        Rc::new(Self {
            text,
            glyph_infos: ArraySlice::new(vec![GlyphInfo::default(); buffer_length]),
            glyph_indices: ArraySlice::new(vec![0u16; buffer_length]),
            glyph_generation: Some(Rc::new(Cell::new(0))),
            glyph_typeface,
            font_rendering_em_size,
            bidi_level,
            cache: RefCell::new(ClusterCache::default()),
            breaks: RefCell::new(None),
            concat_flags: RefCell::new(None),
            disposed: Cell::new(false),
        })
    }

    /// The flag of a glyph whose cluster the text may not be cut at, with the two sides shaped
    /// apart (HarfBuzz's `HB_GLYPH_FLAG_UNSAFE_TO_BREAK`).
    pub const UNSAFE_TO_BREAK: u8 = 1;
    /// The flag of a glyph at whose cluster the text may not be changed on one side without
    /// changing the shaping of the other (HarfBuzz's `HB_GLYPH_FLAG_UNSAFE_TO_CONCAT`).
    pub const UNSAFE_TO_CONCAT: u8 = 2;

    /// Records the concatenation flags of the glyphs, one byte a glyph ([`Self::UNSAFE_TO_BREAK`],
    /// [`Self::UNSAFE_TO_CONCAT`]): a shaper that was asked for them
    /// (`TextShaperOptions::produce_concat_flags`) sets them once it filled the buffer.
    pub fn set_concat_flags(&self, flags: Rc<[u8]>) {
        debug_assert_eq!(flags.len(), self.glyph_infos.length());
        *self.concat_flags.borrow_mut() = Some(flags);
    }

    /// The concatenation flags of the glyphs, when the shaper recorded them.
    pub fn concat_flags(&self) -> Option<Rc<[u8]>> {
        self.concat_flags.borrow().clone()
    }

    /// Creates a buffer of the glyphs `glyphs`, already shaped (an edit of a shaping,
    /// `shaping_edit`).
    pub(crate) fn from_glyphs(
        text: ReadOnlyMemory<u16>,
        glyphs: Vec<GlyphInfo>,
        glyph_typeface: Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        bidi_level: i8,
    ) -> Rc<Self> {
        let indices: Vec<u16> = glyphs.iter().map(|glyph| glyph.glyph_index).collect();
        Rc::new(Self {
            text,
            glyph_infos: ArraySlice::new(glyphs),
            glyph_indices: ArraySlice::new(indices),
            glyph_generation: Some(Rc::new(Cell::new(0))),
            glyph_typeface,
            font_rendering_em_size,
            bidi_level,
            cache: RefCell::new(ClusterCache::default()),
            breaks: RefCell::new(None),
            concat_flags: RefCell::new(None),
            disposed: Cell::new(false),
        })
    }

    /// Creates a buffer over caller-owned glyph storage.
    pub(crate) fn from_slices(
        text: ReadOnlyMemory<u16>,
        glyph_infos: ArraySlice<GlyphInfo>,
        glyph_indices: ArraySlice<u16>,
        glyph_typeface: Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        bidi_level: i8,
    ) -> Rc<Self> {
        Rc::new(Self {
            text,
            glyph_infos,
            glyph_indices,
            glyph_generation: None,
            glyph_typeface,
            font_rendering_em_size,
            bidi_level,
            cache: RefCell::new(ClusterCache::default()),
            breaks: RefCell::new(None),
            concat_flags: RefCell::new(None),
            disposed: Cell::new(false),
        })
    }

    /// Creates a buffer that shares the storage (and, when present, the
    /// cluster cache) of `source`.
    #[allow(clippy::too_many_arguments)]
    fn from_source(
        source: &ShapedBuffer,
        text: ReadOnlyMemory<u16>,
        glyph_infos: ArraySlice<GlyphInfo>,
        glyph_indices: ArraySlice<u16>,
        bidi_level: i8,
        share_cache: bool,
        cluster_start_idx: usize,
        cluster_count: usize,
    ) -> Rc<Self> {
        let cache = {
            let source_cache = source.cache.borrow();
            match (&source_cache.prefix, share_cache) {
                (Some(prefix), true) => ClusterCache {
                    prefix: Some(prefix.clone()),
                    start_chars: source_cache.start_chars.clone(),
                    start_idx: cluster_start_idx,
                    count: cluster_count,
                    generation: source_cache.generation,
                },
                _ => ClusterCache::default(),
            }
        };

        Rc::new(Self {
            text,
            glyph_infos,
            glyph_indices,
            glyph_generation: source.glyph_generation.clone(),
            glyph_typeface: source.glyph_typeface.clone(),
            font_rendering_em_size: source.font_rendering_em_size,
            bidi_level,
            cache: RefCell::new(cache),
            breaks: RefCell::new(source.breaks.borrow().clone()),
            concat_flags: RefCell::new(None),
            disposed: Cell::new(false),
        })
    }

    /// Takes the break table `source` has by now: the run cache keeps a buffer of its own over
    /// the glyphs of a shaping (`sharing`), made before the layout found the breaks of the text.
    pub(crate) fn share_breaks_of(&self, source: &ShapedBuffer) {
        if self.breaks.borrow().is_none() {
            *self.breaks.borrow_mut() = source.breaks.borrow().clone();
        }
    }

    /// Makes the break table of this buffer's text from the table of `old`, the shaping of the
    /// text before an edit, when `old` has the table of its whole text ([`BreakTable::edited`]).
    /// Returns whether the buffer has a table from the edit.
    pub(crate) fn keep_breaks_of_edit(&self, old: &ShapedBuffer) -> bool {
        #[cfg(test)]
        if !BREAK_TABLES.with(Cell::get) {
            return false;
        }
        if self.breaks.borrow().is_some() {
            return false;
        }
        let Some(table) = old.breaks.borrow().clone() else { return false };
        if table.start != old.text.offset_in_owner() {
            return false;
        }
        let edited = table.edited(old.text.span(), self.text.span(), self.text.offset_in_owner());
        let kept = edited.is_some();
        *self.breaks.borrow_mut() = edited.map(Rc::new);
        kept
    }

    /// The buffer's length (number of glyphs).
    #[inline]
    /// The break opportunities of the buffer's text (desktop-performance.md section 9.16): read
    /// from the table of the text it was split from when the buffer reaches the end of that text
    /// and starts at one of its break opportunities (a line starts after the white space of the
    /// line before it, and no rule of UAX #14 looks back across it), found again from its start
    /// otherwise, as upstream finds them for every line.
    pub(crate) fn line_breaks(&self) -> RunLineBreaks<'_> {
        use crate::media::text_formatting::unicode::LineBreakEnumerator;
        #[cfg(test)]
        if !BREAK_TABLES.with(Cell::get) {
            return RunLineBreaks::Enumerator(LineBreakEnumerator::new(self.text.span()));
        }
        let start = self.text.offset_in_owner();
        let end = start + self.text.len();
        let table = self
            .breaks
            .borrow_mut()
            .get_or_insert_with(|| {
                let mut enumerator = LineBreakEnumerator::new(self.text.span());
                let mut breaks = Vec::new();
                while let Some(line_break) = enumerator.move_next() {
                    breaks.push(line_break);
                }
                Rc::new(BreakTable { start, end, breaks: breaks.into_boxed_slice() })
            })
            .clone();
        if table.end == end && table.start <= start {
            let shift = start - table.start;
            if shift == 0 {
                return RunLineBreaks::Table { table, index: 0, shift: 0 };
            }
            if let Ok(found) = table.breaks.binary_search_by(|line_break| line_break.position_wrap().cmp(&shift)) {
                return RunLineBreaks::Table { table, index: found + 1, shift };
            }
        }
        RunLineBreaks::Enumerator(LineBreakEnumerator::new(self.text.span()))
    }

    pub fn length(&self) -> usize {
        if self.disposed.get() {
            return 0;
        }

        self.glyph_infos.length()
    }

    /// The glyph storage.
    #[inline]
    #[allow(dead_code)] // upstream's `GlyphInfos` slice; nothing needs the slice itself yet
    pub(crate) fn glyph_infos_slice(&self) -> &ArraySlice<GlyphInfo> {
        &self.glyph_infos
    }

    /// Borrows the glyphs. Do not hold the borrow across a call to [`ShapedBuffer::set`].
    #[inline]
    pub fn glyph_infos(&self) -> Ref<'_, [GlyphInfo]> {
        let disposed = self.disposed.get();

        // A disposed buffer has no glyphs (upstream resets its views).
        Ref::map(self.glyph_infos.span(), |glyph_infos| if disposed { &glyph_infos[..0] } else { glyph_infos })
    }

    /// The glyph ids of the buffer in glyph order — a contiguous view that
    /// mirrors `glyph_infos()[i].glyph_index`, kept in sync by the setter.
    #[inline]
    pub fn glyph_indices(&self) -> Ref<'_, [u16]> {
        let disposed = self.disposed.get();

        Ref::map(self.glyph_indices.span(), |glyph_indices| if disposed { &glyph_indices[..0] } else { glyph_indices })
    }

    /// The buffer's glyph typeface.
    #[inline]
    pub fn glyph_typeface(&self) -> &Rc<GlyphTypeface> {
        &self.glyph_typeface
    }

    /// The buffer's font rendering em size.
    #[inline]
    pub fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size
    }

    /// The buffer's bidi level.
    #[inline]
    pub fn bidi_level(&self) -> i8 {
        self.bidi_level
    }

    /// The buffer's reading direction.
    #[inline]
    pub fn is_left_to_right(&self) -> bool {
        (self.bidi_level & 1) == 0
    }

    /// The text that is represented by this buffer.
    #[inline]
    pub fn text(&self) -> &ReadOnlyMemory<u16> {
        &self.text
    }

    /// Releases the buffer. Idempotent. Afterwards the buffer has no glyphs
    /// (`length()` is zero); the glyph storage itself is freed when the last
    /// buffer sharing it is dropped.
    pub fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        *self.cache.borrow_mut() = ClusterCache::default();
    }

    /// Reads the glyph at `index`.
    #[inline]
    pub fn get(&self, index: usize) -> GlyphInfo {
        self.glyph_infos.get(index)
    }

    /// Writes the glyph at `index`.
    pub fn set(&self, index: usize, value: GlyphInfo) {
        self.glyph_infos.set(index, value);
        self.glyph_indices.set(index, value.glyph_index);

        // Bump the shared glyph generation so any sibling that built a cluster
        // cache against the pre-mutation glyphs will detect the mismatch on
        // its next query and rebuild.
        if let Some(generation) = &self.glyph_generation {
            generation.set(generation.get().wrapping_add(1));
        }

        self.invalidate_cluster_cache();
    }

    /// True when the cluster cache is in "simple" mode (every cluster is one
    /// character wide).
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn is_cluster_cache_simple(&self) -> bool {
        self.ensure_cluster_cache();
        self.cache.borrow().start_chars.is_none()
    }

    /// Sum of all glyph advances in this buffer (cached).
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn total_glyph_advance(&self) -> f64 {
        self.ensure_cluster_cache();
        let cache = self.cache.borrow();
        let prefix = cache.prefix.as_deref().unwrap_or(&[0.0]);
        prefix[cache.start_idx + cache.count] - prefix[cache.start_idx]
    }

    /// Number of text characters covered by this buffer's first cluster, in
    /// logical order. Zero for an empty buffer.
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn first_cluster_char_length(&self) -> i32 {
        self.ensure_cluster_cache();
        let cache = self.cache.borrow();

        if cache.count == 0 {
            return 0;
        }

        match &cache.start_chars {
            // Simple mode: every cluster is one character wide.
            None => 1,
            Some(starts) => starts[cache.start_idx + 1] - starts[cache.start_idx],
        }
    }

    fn ensure_cluster_cache(&self) {
        let current_generation = self.glyph_generation.as_ref().map_or(0, |generation| generation.get());

        {
            let cache = self.cache.borrow();
            if cache.prefix.is_some() {
                if cache.generation == current_generation {
                    return;
                }
                // A sibling mutated the shared glyph array since this cache
                // was built — rebuild below against the current glyph data.
            }
        }

        let glyph_infos = self.glyph_infos();
        let buffer_length = glyph_infos.len();

        if buffer_length == 0 {
            drop(glyph_infos);

            *self.cache.borrow_mut() = ClusterCache {
                prefix: Some(Rc::from([0.0f64])),
                start_chars: Some(Rc::from([0i32])),
                start_idx: 0,
                count: 0,
                generation: current_generation,
            };
            return;
        }

        let is_ltr = self.is_left_to_right();
        // Logical order: forward for LTR, backward for RTL.
        let logical = |i: usize| -> &GlyphInfo {
            if is_ltr {
                &glyph_infos[i]
            } else {
                &glyph_infos[buffer_length - 1 - i]
            }
        };

        let base_cluster = logical(0).glyph_cluster;
        let text_length = self.text.len();

        // First pass: count clusters by counting cluster-id transitions in
        // logical order. Also track whether this is the "simple" case where
        // buffer_length == text_length == clusters and cluster ids are exactly
        // base_cluster + logical_index — i.e. one glyph per cluster, one char
        // per cluster.
        let mut clusters = 1usize;
        let mut can_be_simple = buffer_length == text_length;
        {
            let mut prev_id = base_cluster;
            for logical_index in 1..buffer_length {
                let id = logical(logical_index).glyph_cluster;
                if id != prev_id {
                    clusters += 1;
                    prev_id = id;
                }
                if can_be_simple && id - base_cluster != logical_index as i32 {
                    can_be_simple = false;
                }
            }
        }

        let simple = can_be_simple && clusters == buffer_length;

        let mut prefix = vec![0.0f64; clusters + 1];
        let mut start_chars = if simple { Vec::new() } else { vec![0i32; clusters + 1] };

        let mut cluster_index = 0usize;
        let mut current_cluster_id = base_cluster;
        let mut current_width = 0.0f64;

        for i in 0..buffer_length {
            let info = logical(i);
            if info.glyph_cluster != current_cluster_id {
                prefix[cluster_index + 1] = prefix[cluster_index] + current_width;
                if !simple {
                    // Cluster ids increase in logical order in both directions.
                    start_chars[cluster_index + 1] = info.glyph_cluster - base_cluster;
                }
                cluster_index += 1;
                current_cluster_id = info.glyph_cluster;
                current_width = info.glyph_advance;
            } else {
                current_width += info.glyph_advance;
            }
        }

        // Close the final cluster.
        prefix[cluster_index + 1] = prefix[cluster_index] + current_width;
        if !simple {
            start_chars[cluster_index + 1] = text_length as i32;
        }

        drop(glyph_infos);

        *self.cache.borrow_mut() = ClusterCache {
            prefix: Some(Rc::from(prefix)),
            start_chars: if simple { None } else { Some(Rc::from(start_chars)) },
            start_idx: 0,
            count: clusters,
            generation: current_generation,
        };
    }

    /// Returns the cluster offset (relative to this buffer's `start_idx`) at
    /// which the split boundary falls, given a split character count (relative
    /// to this buffer's text start). The cache must be built.
    fn find_cluster_offset_for_split(cache: &ClusterCache, split_char_count: i32) -> usize {
        let Some(starts) = &cache.start_chars else {
            // Simple mode: cluster index == char offset within this sub-buffer.
            return split_char_count.max(0) as usize;
        };

        let start_idx = cache.start_idx;
        let target_char = starts[start_idx] + split_char_count;

        // Binary search for the first cluster whose start char >= target_char.
        let mut lo = 0usize;
        let mut hi = cache.count;
        while lo < hi {
            let mid = (lo + hi) >> 1;
            if starts[start_idx + mid] < target_char {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// The shared prefix sum array of the cluster cache, when built.
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn cluster_prefix(&self) -> Option<Rc<[f64]>> {
        self.cache.borrow().prefix.clone()
    }

    fn invalidate_cluster_cache(&self) {
        *self.cache.borrow_mut() = ClusterCache::default();
    }

    /// An alias of this buffer with a different bidi level, sharing the glyph
    /// storage and (when built) the cluster cache.
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn with_bidi_level(self: &Rc<Self>, paragraph_embedding_level: i8) -> Rc<ShapedBuffer> {
        if self.bidi_level == paragraph_embedding_level {
            return self.clone();
        }

        // The bidi level only affects how callers interpret the buffer, not
        // the shaper's glyph order, so the logical-order prefix sums are shared.
        let (start_idx, count) = {
            let cache = self.cache.borrow();
            (cache.start_idx, cache.count)
        };

        Self::from_source(
            self,
            self.text.clone(),
            self.glyph_infos.clone(),
            self.glyph_indices.clone(),
            paragraph_embedding_level,
            true,
            start_idx,
            count,
        )
    }

    /// Creates a writable copy of this buffer backed by freshly allocated
    /// storage. Use this when the caller needs to mutate glyph advances (e.g.
    /// justification, letter spacing) without affecting other buffers that
    /// share the original storage.
    pub(crate) fn clone_writable(&self) -> Rc<ShapedBuffer> {
        let glyphs = self.glyph_infos.span().to_vec();
        let indices = self.glyph_indices.span().to_vec();

        Self::from_slices(
            self.text.clone(),
            ArraySlice::new(glyphs),
            ArraySlice::new(indices),
            self.glyph_typeface.clone(),
            self.font_rendering_em_size,
            self.bidi_level,
        )
    }

    /// A buffer of its own over the glyph storage of this one, with its concatenation flags: the
    /// run cache keeps it for an edit of the text. Disposing this buffer (a run that holds the
    /// whole of a shaping holds the shaped buffer itself, and the cache disposes its runs) leaves
    /// the kept one its glyphs; the storage is never written in place (justification and tab
    /// layout write to a copy).
    pub(crate) fn sharing(&self) -> Rc<ShapedBuffer> {
        let kept = Self::from_source(
            self,
            self.text.clone(),
            self.glyph_infos.clone(),
            self.glyph_indices.clone(),
            self.bidi_level,
            false,
            0,
            0,
        );
        *kept.concat_flags.borrow_mut() = self.concat_flags.borrow().clone();
        kept
    }

    /// Splits the buffer at the specified text length (UTF-16 code units).
    ///
    /// Both halves share this buffer's glyph storage and cluster cache.
    pub fn split(self: &Rc<Self>, text_length: i32) -> SplitResult<Rc<ShapedBuffer>> {
        let text_length = (self.text.len() as i32).min(text_length);

        if text_length <= 0 {
            let empty_buffer = Self::from_slices(
                self.text.slice(0, 0),
                self.glyph_infos.slice(self.glyph_infos.start(), 0),
                self.glyph_indices.slice(self.glyph_indices.start(), 0),
                self.glyph_typeface.clone(),
                self.font_rendering_em_size,
                self.bidi_level,
            );

            return SplitResult::new(Some(empty_buffer), Some(self.clone()));
        }

        if text_length as usize == self.text.len() {
            return SplitResult::new(Some(self.clone()), None);
        }

        if self.is_left_to_right() {
            self.split_ascending(text_length)
        } else {
            self.split_descending(text_length)
        }
    }

    fn split_ascending(self: &Rc<Self>, text_length: i32) -> SplitResult<Rc<ShapedBuffer>> {
        let slice_start = self.glyph_infos.start();
        let glyph_infos_length = self.glyph_infos.length();

        let (split_glyph_index, split_char_count) = {
            let glyph_infos = self.glyph_infos.span();

            let base_cluster = glyph_infos[0].glyph_cluster;
            let target_cluster = base_cluster + text_length;

            let search_value = GlyphInfo::new(0, target_cluster, 0.0);

            match glyph_infos.binary_search_by(|probe| GlyphInfo::cluster_ascending_comparer(probe, &search_value)) {
                Ok(found_index) => {
                    let mut i = found_index;
                    while i > 0 && glyph_infos[i - 1].glyph_cluster == target_cluster {
                        i -= 1;
                    }
                    (i, target_cluster - base_cluster)
                }
                Err(inverted_index) => {
                    if inverted_index >= glyph_infos_length {
                        (glyph_infos_length, self.text.len() as i32)
                    } else {
                        (inverted_index, glyph_infos[inverted_index].glyph_cluster - base_cluster)
                    }
                }
            }
        };

        let first_glyphs = self.glyph_infos.slice(slice_start, split_glyph_index);
        let second_glyphs =
            self.glyph_infos.slice(slice_start + split_glyph_index, glyph_infos_length - split_glyph_index);
        let first_glyph_indices = self.glyph_indices.slice(slice_start, split_glyph_index);
        let second_glyph_indices =
            self.glyph_indices.slice(slice_start + split_glyph_index, glyph_infos_length - split_glyph_index);

        let split_chars = (split_char_count.max(0) as usize).min(self.text.len());
        let first_text = self.text.slice(0, split_chars);
        let second_text = self.text.slice_from(split_chars);

        // Share the parent's cluster cache with both halves so subsequent
        // metric queries and further splits run in O(1) (cached total advance)
        // or O(log clusters) (binary search), instead of re-walking glyphs.
        self.ensure_cluster_cache();
        let (start_idx, count, leading_cluster_count) = {
            let cache = self.cache.borrow();
            (cache.start_idx, cache.count, Self::find_cluster_offset_for_split(&cache, split_char_count).min(cache.count))
        };

        let leading = Self::from_source(
            self,
            first_text,
            first_glyphs,
            first_glyph_indices,
            self.bidi_level,
            true,
            start_idx,
            leading_cluster_count,
        );

        if second_text.is_empty() {
            return SplitResult::new(Some(leading), None);
        }

        let trailing = Self::from_source(
            self,
            second_text,
            second_glyphs,
            second_glyph_indices,
            self.bidi_level,
            true,
            start_idx + leading_cluster_count,
            count - leading_cluster_count,
        );

        SplitResult::new(Some(leading), Some(trailing))
    }

    fn split_descending(self: &Rc<Self>, text_length: i32) -> SplitResult<Rc<ShapedBuffer>> {
        let slice_start = self.glyph_infos.start();
        let glyph_infos_length = self.glyph_infos.length();

        // Boundary between "second" (head / leading-visual) and "first" (tail / trailing-visual).
        let (split_glyph_index, split_char_count) = {
            let glyph_infos = self.glyph_infos.span();

            let base_cluster = glyph_infos[glyph_infos_length - 1].glyph_cluster;
            let target_cluster = base_cluster + text_length;

            // Clusters descend with the glyph index.
            let search_value = GlyphInfo::new(0, target_cluster, 0.0);

            let split_glyph_index =
                match glyph_infos.binary_search_by(|probe| GlyphInfo::cluster_descending_comparer(probe, &search_value)) {
                    Ok(mut found_index) => {
                        while found_index + 1 < glyph_infos_length
                            && glyph_infos[found_index + 1].glyph_cluster == target_cluster
                        {
                            found_index += 1;
                        }
                        found_index + 1
                    }
                    Err(inverted_index) => inverted_index,
                };

            let split_char_count = if split_glyph_index > 0 {
                (glyph_infos[split_glyph_index - 1].glyph_cluster - base_cluster).min(self.text.len() as i32)
            } else {
                self.text.len() as i32
            };

            (split_glyph_index, split_char_count)
        };

        let second_glyphs = self.glyph_infos.slice(slice_start, split_glyph_index);
        let first_glyphs =
            self.glyph_infos.slice(slice_start + split_glyph_index, glyph_infos_length - split_glyph_index);
        let second_glyph_indices = self.glyph_indices.slice(slice_start, split_glyph_index);
        let first_glyph_indices =
            self.glyph_indices.slice(slice_start + split_glyph_index, glyph_infos_length - split_glyph_index);

        let split_chars = (split_char_count.max(0) as usize).min(self.text.len());
        let first_text = self.text.slice(0, split_chars);
        let second_text = self.text.slice_from(split_chars);

        // The cache is stored in logical order, so the leading logical
        // clusters (corresponding to `first`) come first in the prefix array.
        self.ensure_cluster_cache();
        let (start_idx, count, first_cluster_count) = {
            let cache = self.cache.borrow();
            (cache.start_idx, cache.count, Self::find_cluster_offset_for_split(&cache, split_char_count).min(cache.count))
        };

        let first = Self::from_source(
            self,
            first_text,
            first_glyphs,
            first_glyph_indices,
            self.bidi_level,
            true,
            start_idx,
            first_cluster_count,
        );

        if second_text.is_empty() || second_glyphs.length() == 0 {
            return SplitResult::new(Some(first), None);
        }

        let second = Self::from_source(
            self,
            second_text,
            second_glyphs,
            second_glyph_indices,
            self.bidi_level,
            true,
            start_idx + first_cluster_count,
            count - first_cluster_count,
        );

        SplitResult::new(Some(first), Some(second))
    }

    /// Sum of cluster advances over the half-open character range
    /// `[start_char, end_char)` relative to this buffer's text. Snaps both
    /// boundaries down to the largest cluster start at or before the given
    /// character offset.
    ///
    /// Internal upstream; public so that the Skia unit tests reach it.
    pub fn get_char_range_width(&self, start_char: i32, end_char: i32) -> f64 {
        if end_char <= start_char {
            return 0.0;
        }

        self.ensure_cluster_cache();
        let cache = self.cache.borrow();
        let prefix = cache.prefix.as_deref().unwrap_or(&[0.0]);
        let start_idx = cache.start_idx;
        let count = cache.count;

        let (start_boundary, end_boundary) = match &cache.start_chars {
            None => (
                (start_char.max(0) as usize).min(count),
                (end_char.max(0) as usize).min(count),
            ),
            Some(starts) => {
                let base_char = starts[start_idx];
                (
                    Self::find_largest_cluster_at_or_before(starts, start_idx, count, base_char, start_char),
                    Self::find_largest_cluster_at_or_before(starts, start_idx, count, base_char, end_char),
                )
            }
        };

        prefix[start_idx + end_boundary] - prefix[start_idx + start_boundary]
    }

    fn find_largest_cluster_at_or_before(
        starts: &[i32],
        start_idx: usize,
        count: usize,
        base_char: i32,
        char_pos: i32,
    ) -> usize {
        if char_pos < 0 {
            return 0;
        }

        // Largest k in [0, count] with (starts[start_idx + k] - base_char) <= char_pos.
        let mut lo = 0usize;
        let mut hi = count;
        while lo < hi {
            let mid = (lo + hi + 1) >> 1;
            if starts[start_idx + mid] - base_char <= char_pos {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo
    }

    /// The largest count of leading characters (in logical order) whose
    /// cumulative cluster advance fits within `available_width`, or 0 if not
    /// even the first cluster fits.
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn find_leading_char_count_within_width(&self, available_width: f64) -> i32 {
        if available_width <= 0.0 {
            return 0;
        }

        self.ensure_cluster_cache();
        let cache = self.cache.borrow();
        let prefix = cache.prefix.as_deref().unwrap_or(&[0.0]);
        let start_idx = cache.start_idx;
        let base_prefix = prefix[start_idx];

        // Largest k in [0, count] with prefix[start_idx + k] - base_prefix <= available_width.
        let mut lo = 0usize;
        let mut hi = cache.count;
        while lo < hi {
            let mid = (lo + hi + 1) >> 1;
            if MathUtilities::less_than_or_close(prefix[start_idx + mid] - base_prefix, available_width) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }

        match &cache.start_chars {
            None => lo as i32,
            Some(starts) => starts[start_idx + lo] - starts[start_idx],
        }
    }

    /// The largest count of trailing characters (in logical order) whose
    /// cumulative cluster advance fits within `available_width`, together with
    /// the consumed width. `(0, 0.0)` if not even the last cluster fits.
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn find_trailing_char_count_within_width(&self, available_width: f64) -> (i32, f64) {
        if available_width <= 0.0 {
            return (0, 0.0);
        }

        self.ensure_cluster_cache();
        let cache = self.cache.borrow();
        let prefix = cache.prefix.as_deref().unwrap_or(&[0.0]);
        let start_idx = cache.start_idx;
        let count = cache.count;
        let end_prefix = prefix[start_idx + count];

        // Smallest k in [0, count] with end_prefix - prefix[start_idx + k] <= available_width.
        let mut lo = 0usize;
        let mut hi = count;
        while lo < hi {
            let mid = (lo + hi) >> 1;
            if MathUtilities::less_than_or_close(end_prefix - prefix[start_idx + mid], available_width) {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }

        let consumed_width = end_prefix - prefix[start_idx + lo];

        match &cache.start_chars {
            None => ((count - lo) as i32, consumed_width),
            Some(starts) => (starts[start_idx + count] - starts[start_idx + lo], consumed_width),
        }
    }
}
