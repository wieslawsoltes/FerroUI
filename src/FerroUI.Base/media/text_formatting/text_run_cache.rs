use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;
use std::sync::Arc;

use crate::media::i_immutable_glyph_run_reference::release_platform_impl;
use crate::media::text_formatting::shaped_buffer::GlyphRange;
use crate::media::text_formatting::{ShapedBuffer, ShapedTextRun, TextLineMetricsMode, TextRun, TextShaperOptions};
use crate::media::{FlowDirection, GlyphInfoList};
use crate::platform::IGlyphRunImpl;

/// Caches shaped text runs and bidi processing results to avoid redundant
/// shaping when only the paragraph width constraint changes (e.g., between
/// Measure and Arrange).
///
/// Uses an inline single-entry store for the common case of a single
/// paragraph, and only promotes to a dictionary when multiple entries are
/// added.
///
/// This API is in preview and subject to change without deprecation.
#[derive(Default)]
pub struct TextRunCache {
    state: RefCell<State>,
}

#[derive(Default)]
struct State {
    // Single-entry inline store (avoids a map allocation for the common single-paragraph case).
    single: Option<(i32, CachedShapingResult)>,
    // Multi-entry store (only allocated when 2+ distinct keys are added).
    entries: Option<HashMap<i32, CachedShapingResult>>,
    /// What is kept of a long text for an edit of it and for its next layout (step 7 of
    /// text-layout-performance.md). Made with the first shaping that is kept and dropped when
    /// nothing is kept any more: the cache of a short text has none.
    long: Option<Box<LongText>>,
    /// Whether a layout is being made.
    in_layout: bool,
}

/// What a run cache keeps of a long text.
#[derive(Default)]
struct LongText {
    /// The shapings of long texts made for the current text, with their concatenation flags,
    /// for the next text of the owner (`retire`).
    kept: Vec<KeptShaping>,
    /// The shapings kept for the text before the current one: an edit of that text shapes the
    /// new one from them (`shaping_edit`).
    retired: Vec<KeptShaping>,
    /// The platform glyph runs of the lines of the layout that is being made, by the glyphs they
    /// are made of (`keep_line_glyph_runs`).
    line_runs: GlyphRunTable,
    /// While a layout is made (`begin_layout`): the platform glyph runs of the lines of the
    /// layout before, which a line of the same glyphs takes. Empty between layouts.
    previous_line_runs: GlyphRunTable,
}

impl LongText {
    fn is_empty(&self) -> bool {
        self.kept.is_empty() && self.retired.is_empty() && self.line_runs.is_empty()
    }
}

/// The platform glyph runs of the lines of a layout, by the storage and the first glyph of the
/// glyphs they are made of.
type GlyphRunTable = HashMap<(u64, usize), KeptGlyphRun, BuildHasherDefault<GlyphRangeHasher>>;

struct KeptGlyphRun {
    length: usize,
    generation: i32,
    platform_impl: Arc<dyn IGlyphRunImpl>,
}

/// The hash of a storage number and a glyph index: two multiplications, no more is needed for
/// numbers that count up.
#[derive(Default)]
struct GlyphRangeHasher(u64);

impl Hasher for GlyphRangeHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_u64(byte as u64);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x517C_C1B7_2722_0A95);
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }
}

fn release_glyph_runs(table: &mut GlyphRunTable) {
    for (_, kept) in table.drain() {
        release_platform_impl(Some(kept.platform_impl));
    }
}

/// A layout that is being made with a run cache: [`TextRunCache::begin_layout`] when it starts,
/// [`TextRunCache::end_layout`] when it is dropped.
pub(crate) struct TextRunCacheLayout(Rc<TextRunCache>);

impl TextRunCacheLayout {
    pub(crate) fn begin(cache: &Rc<TextRunCache>) -> Self {
        cache.begin_layout();
        Self(cache.clone())
    }
}

impl Drop for TextRunCacheLayout {
    fn drop(&mut self) {
        self.0.end_layout();
    }
}

/// A shaping kept for an edit of its text.
struct KeptShaping {
    options: TextShaperOptions,
    buffer: Rc<ShapedBuffer>,
    /// The buffer of the runs, for the break table the layout makes on it after the shaping is
    /// kept (a disposed buffer keeps its table).
    source: Rc<ShapedBuffer>,
}

impl TextRunCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Invalidates all cached entries and disposes their shaped buffers.
    pub fn invalidate(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(mut long) = state.long.take() {
            release_glyph_runs(&mut long.line_runs);
            release_glyph_runs(&mut long.previous_line_runs);
        }
        Self::invalidate_entries(&mut state);
    }

    /// A layout starts: the platform glyph runs of the lines of the layout before are kept until
    /// [`end_layout`](Self::end_layout) for the lines of this one, and the lines of this one are
    /// kept for the next layout (docs/porting/text-layout-performance.md, step 7). A line of the
    /// layout before whose glyphs are the glyphs of a line of this one (the text was not edited
    /// there and the line breaks where it did) gives its platform glyph run to the new line:
    /// the bounds of the glyphs are not measured again, and the renderer draws the run it has
    /// drawn before. Only the lines of a text a shaping is kept of are kept (a long text): a
    /// short text is laid out again at no cost worth a table of its lines.
    pub(crate) fn begin_layout(&self) {
        let mut state = self.state.borrow_mut();
        state.in_layout = true;
        if let Some(long) = &mut state.long {
            release_glyph_runs(&mut long.previous_line_runs);
            std::mem::swap(&mut long.line_runs, &mut long.previous_line_runs);
        }
    }

    /// The layout that started is made: the platform glyph runs of the layout before that no
    /// line took are released.
    pub(crate) fn end_layout(&self) {
        let mut state = self.state.borrow_mut();
        state.in_layout = false;
        if let Some(long) = &mut state.long {
            release_glyph_runs(&mut long.previous_line_runs);
            if long.is_empty() {
                state.long = None;
            }
        }
    }

    /// Gives the shaped runs of a line that is about to be finished the platform glyph runs kept
    /// for their glyphs from the layout before. Nothing outside a layout.
    pub(crate) fn adopt_line_glyph_runs(&self, text_runs: &[Rc<dyn TextRun>]) {
        let state = self.state.borrow();
        let Some(long) = state.long.as_deref().filter(|long| state.in_layout && !long.previous_line_runs.is_empty())
        else {
            return;
        };
        for run in text_runs {
            let Some((run, range)) = Self::glyph_range_of(run) else {
                continue;
            };
            let buffer = run.shaped_buffer();
            let kept = [Some(range), buffer.glyph_range_before_edit()].into_iter().flatten().find_map(|range| {
                let kept = long.previous_line_runs.get(&(range.storage, range.start))?;
                (kept.length == range.length && kept.generation == range.generation).then_some(kept)
            });
            if let Some(kept) = kept {
                if run.glyph_run().adopt_platform_impl(&kept.platform_impl) {
                    crate::perf_count!(TextGlyphRunsKept);
                }
            }
        }
    }

    /// Keeps the platform glyph runs of the shaped runs of a finished line for the next layout.
    /// Nothing outside a layout, and nothing for a text no shaping is kept of.
    pub(crate) fn keep_line_glyph_runs(&self, text_runs: &[Rc<dyn TextRun>]) {
        let mut state = self.state.borrow_mut();
        let in_layout = state.in_layout;
        let Some(long) = state.long.as_deref_mut().filter(|long| in_layout && !long.kept.is_empty()) else {
            return;
        };
        for run in text_runs {
            let Some((run, range)) = Self::glyph_range_of(run) else {
                continue;
            };
            let Some(platform_impl) = run.glyph_run().created_platform_impl() else {
                continue;
            };
            let kept = KeptGlyphRun { length: range.length, generation: range.generation, platform_impl };
            if let Some(replaced) = long.line_runs.insert((range.storage, range.start), kept) {
                release_platform_impl(Some(replaced.platform_impl));
            }
        }
    }

    /// The shaped run `run` is and the glyphs its platform glyph run is made of, when those are
    /// glyphs of a storage the shaper filled: a left-to-right run whose glyph run has the glyphs
    /// of the shaped buffer of the run.
    fn glyph_range_of(run: &Rc<dyn TextRun>) -> Option<(&ShapedTextRun, GlyphRange)> {
        let run = run.downcast_ref::<ShapedTextRun>()?;
        let buffer = run.shaped_buffer();
        if run.bidi_level() & 1 != 0 || buffer.length() == 0 {
            return None;
        }
        match run.glyph_run().glyph_infos() {
            GlyphInfoList::ShapedBuffer(glyphs) if Rc::ptr_eq(&glyphs, buffer) => {
                buffer.glyph_range().map(|range| (run, range))
            }
            _ => None,
        }
    }

    /// Invalidates the cached entries for a new text of the owner, as
    /// [`invalidate`](Self::invalidate) does, and keeps the shapings of long texts of the text
    /// before for the new one: an edit of a long text shapes again only what the edit changed.
    pub fn retire(&self) {
        let mut state = self.state.borrow_mut();
        if let Some(long) = &mut state.long {
            long.retired = std::mem::take(&mut long.kept);
        }
        Self::invalidate_entries(&mut state);
    }

    /// Keeps `buffer`, the shaping of a long text with its concatenation flags, for the next text
    /// of the owner.
    pub(crate) fn keep_shaping(&self, options: &TextShaperOptions, buffer: &Rc<ShapedBuffer>) {
        let mut state = self.state.borrow_mut();
        let long = state.long.get_or_insert_with(Default::default);
        // One shaping a set of options: a text shaped again (the cached runs dropped for another
        // reason) replaces the one kept before.
        long.kept.retain(|kept| !kept.options.shapes_as(options));
        // A buffer of its own over the glyphs: the runs of the cache hold `buffer` and dispose it.
        long.kept.push(KeptShaping { options: options.clone(), buffer: buffer.sharing(), source: buffer.clone() });
    }

    /// The kept shaping of the text before the current one that was shaped as `options` shape,
    /// taken out of the cache, with the break table the layout of that text made.
    pub(crate) fn take_retired(&self, options: &TextShaperOptions) -> Option<Rc<ShapedBuffer>> {
        let mut state = self.state.borrow_mut();
        let long = state.long.as_deref_mut()?;
        let index = long.retired.iter().position(|kept| kept.options.shapes_as(options))?;
        let kept = long.retired.swap_remove(index);
        kept.buffer.share_breaks_of(&kept.source);
        Some(kept.buffer)
    }

    fn invalidate_entries(state: &mut State) {
        if let Some((_, value)) = state.single.take() {
            Self::dispose_cached_runs(&value);
            return;
        }

        let Some(entries) = &mut state.entries else {
            return;
        };

        for entry in entries.values() {
            Self::dispose_cached_runs(entry);
        }

        entries.clear();
    }

    /// Invalidates all cached entries at or after the specified text source index.
    pub fn invalidate_from(&self, text_source_index: i32) {
        let mut state = self.state.borrow_mut();

        if let Some((key, value)) = &state.single {
            if *key >= text_source_index {
                Self::dispose_cached_runs(value);
                state.single = None;
            }

            return;
        }

        let Some(entries) = &mut state.entries else {
            return;
        };

        entries.retain(|key, result| {
            if *key >= text_source_index {
                Self::dispose_cached_runs(result);
                false
            } else {
                true
            }
        });
    }

    /// Tries to retrieve cached shaped runs for the given text source index.
    pub(crate) fn try_get_shaped_runs(&self, first_text_source_index: i32) -> Option<CachedShapingResult> {
        let state = self.state.borrow();

        if let Some((key, value)) = &state.single {
            if *key == first_text_source_index {
                return Some(value.clone());
            }
        }

        state.entries.as_ref().and_then(|entries| entries.get(&first_text_source_index).cloned())
    }

    /// Adds shaped runs to the cache for the given text source index. The cache takes
    /// its own reference to each [`ShapedTextRun`]; the caller retains its
    /// original references unchanged.
    pub(crate) fn add(&self, first_text_source_index: i32, result: CachedShapingResult) {
        Self::add_ref_shaped_runs(&result.shaped_runs);

        let mut state = self.state.borrow_mut();

        if let Some(entries) = &mut state.entries {
            if let Some(existing) = entries.insert(first_text_source_index, result) {
                Self::dispose_cached_runs(&existing);
            }

            return;
        }

        match state.single.take() {
            None => {
                state.single = Some((first_text_source_index, result));
            }
            Some((single_key, single_value)) if single_key == first_text_source_index => {
                Self::dispose_cached_runs(&single_value);
                state.single = Some((first_text_source_index, result));
            }
            Some((single_key, single_value)) => {
                // Second distinct key: promote to dictionary.
                let mut entries = HashMap::new();
                entries.insert(single_key, single_value);
                entries.insert(first_text_source_index, result);
                state.entries = Some(entries);
            }
        }
    }

    fn add_ref_shaped_runs(runs: &[Rc<dyn TextRun>]) {
        for run in runs {
            if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                shaped.add_reference();
            }
        }
    }

    /// Releases the cache.
    pub fn dispose(&self) {
        self.invalidate();
        self.state.borrow_mut().entries = None;
    }

    fn dispose_cached_runs(result: &CachedShapingResult) {
        for run in result.shaped_runs.iter() {
            if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                shaped.dispose();
            }
        }
    }

    /// Whether the multi-entry store has been allocated.
    #[cfg(test)]
    pub(crate) fn has_entries_map(&self) -> bool {
        self.state.borrow().entries.is_some()
    }
}

/// Stores the result of text shaping for a paragraph segment starting at a
/// given text source index.
#[derive(Clone)]
pub(crate) struct CachedShapingResult {
    /// The shaped text runs (output of the shaping step).
    pub shaped_runs: Rc<[Rc<dyn TextRun>]>,
    /// The resolved flow direction for the paragraph.
    pub resolved_flow_direction: FlowDirection,
    /// The end of line marker, if any.
    pub text_end_of_line: Option<Rc<dyn TextRun>>,
    /// The total text source length consumed.
    pub text_source_length: i32,
    /// The grouping policy used when the runs were shaped.
    pub line_metrics_mode: TextLineMetricsMode,
}

impl CachedShapingResult {
    pub fn new(
        shaped_runs: Rc<[Rc<dyn TextRun>]>,
        resolved_flow_direction: FlowDirection,
        text_end_of_line: Option<Rc<dyn TextRun>>,
        text_source_length: i32,
    ) -> Self {
        Self {
            shaped_runs,
            resolved_flow_direction,
            text_end_of_line,
            text_source_length,
            line_metrics_mode: TextLineMetricsMode::GlyphTypeface,
        }
    }

    pub fn with_line_metrics_mode(mut self, mode: TextLineMetricsMode) -> Self {
        self.line_metrics_mode = mode;
        self
    }
}
