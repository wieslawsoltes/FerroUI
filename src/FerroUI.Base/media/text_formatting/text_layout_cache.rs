//! The layouts of plain text a font manager keeps by what they were made of (a FerroUI
//! addition, docs/porting/desktop-performance.md, section 9.29).
//!
//! A text that is laid out again with the same run properties, paragraph properties, trimming
//! and constraint gets the layout that was made for it before: the text is not shaped again,
//! its lines are not built again and its glyph runs keep what the renderer made for them. A
//! layout is immutable once it is made, so every text block that shows the text can hold the
//! same one.
//!
//! The cache owns what it hands out: [`TextLayout::dispose`] leaves a shared layout alone and
//! the cache releases it when it lets it go. Layouts that something still holds are never let
//! go; of the ones nothing holds, the [`IDLE_CAPACITY`] used last are kept, as far as their
//! text is within [`IDLE_TEXT_CAPACITY`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::rc::Rc;

use crate::media::text_formatting::{
    TextLayout, TextLineMetricsMode, TextParagraphProperties, TextRunProperties,
};
use crate::media::{FlowDirection, TextAlignment, TextLineBounds, TextTrimming, TextWrapping};

/// The layouts nothing holds that are kept for the next use of their text.
pub(crate) const IDLE_CAPACITY: usize = 4096;

/// The text (in UTF-8 bytes) of the layouts nothing holds that are kept: what a layout keeps
/// grows with its text, so long texts are let go sooner than their number alone would.
pub(crate) const IDLE_TEXT_CAPACITY: usize = 64 * 1024;

/// The longest text (in UTF-8 bytes) whose layout is shared: a longer one is a document, which
/// is rarely shown twice and whose key would cost what it saves.
pub(crate) const MAX_TEXT_LENGTH: usize = 256;

/// How many layouts are added between two looks for layouts to let go.
const SWEEP_INTERVAL: usize = IDLE_CAPACITY / 4;

/// What a plain text layout is made of, beside its text.
pub(crate) struct PlainTextLayoutRequest<'a> {
    pub text: &'a str,
    pub paragraph_properties: &'a dyn TextParagraphProperties,
    pub text_trimming: &'a Rc<dyn TextTrimming>,
    pub max_width: f64,
    pub max_height: f64,
    pub max_lines: i32,
}

impl PlainTextLayoutRequest<'_> {
    /// Whether a layout of the request can be shared. Text decorations, font features and tab
    /// stops are collections a layout is made of and that can change after it: such a layout
    /// stays with its owner.
    fn is_shareable(&self) -> bool {
        let run_properties = self.paragraph_properties.default_text_run_properties();
        self.text.len() <= MAX_TEXT_LENGTH
            && run_properties.text_decorations().is_none()
            && run_properties.font_features().is_none()
            && self.paragraph_properties.text_decorations().is_none()
            && self.paragraph_properties.tab_stops().is_none()
    }

    fn hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.text.hash(&mut hasher);
        let run_properties = self.paragraph_properties.default_text_run_properties();
        run_properties.font_rendering_em_size().to_bits().hash(&mut hasher);
        self.max_width.to_bits().hash(&mut hasher);
        hasher.finish()
    }
}

/// The values of paragraph properties a layout depends on.
#[derive(Clone, Copy, PartialEq, Eq)]
struct ParagraphKey {
    flow_direction: FlowDirection,
    text_alignment: TextAlignment,
    text_wrapping: TextWrapping,
    text_line_bounds: TextLineBounds,
    line_metrics_mode: TextLineMetricsMode,
    first_line_in_paragraph: bool,
    always_collapsible: bool,
    line_height: u64,
    line_spacing: u64,
    indent: u64,
    paragraph_indent: u64,
    default_incremental_tab: u64,
    tab_origin: u64,
    letter_spacing: u64,
}

impl ParagraphKey {
    fn new(properties: &dyn TextParagraphProperties) -> Self {
        Self {
            flow_direction: properties.flow_direction(),
            text_alignment: properties.text_alignment(),
            text_wrapping: properties.text_wrapping(),
            text_line_bounds: properties.text_line_bounds(),
            line_metrics_mode: properties.line_metrics_mode(),
            first_line_in_paragraph: properties.first_line_in_paragraph(),
            always_collapsible: properties.always_collapsible(),
            line_height: properties.line_height().to_bits(),
            line_spacing: properties.line_spacing().to_bits(),
            indent: properties.indent().to_bits(),
            paragraph_indent: properties.paragraph_indent().to_bits(),
            default_incremental_tab: properties.default_incremental_tab().to_bits(),
            tab_origin: properties.tab_origin().to_bits(),
            letter_spacing: properties.letter_spacing().to_bits(),
        }
    }
}

/// What a kept layout was made of.
struct Key {
    text: Box<str>,
    run_properties: Rc<dyn TextRunProperties>,
    paragraph: ParagraphKey,
    text_trimming: Rc<dyn TextTrimming>,
    max_width: u64,
    max_height: u64,
    max_lines: i32,
}

impl Key {
    fn new(request: &PlainTextLayoutRequest<'_>, paragraph: ParagraphKey) -> Self {
        Self {
            text: request.text.into(),
            run_properties: request.paragraph_properties.default_text_run_properties().clone(),
            paragraph,
            text_trimming: request.text_trimming.clone(),
            max_width: request.max_width.to_bits(),
            max_height: request.max_height.to_bits(),
            max_lines: request.max_lines,
        }
    }

    fn matches(&self, request: &PlainTextLayoutRequest<'_>, paragraph: &ParagraphKey) -> bool {
        self.max_width == request.max_width.to_bits()
            && self.max_height == request.max_height.to_bits()
            && self.max_lines == request.max_lines
            && *self.text == *request.text
            && self.paragraph == *paragraph
            && Rc::ptr_eq(&self.text_trimming, request.text_trimming)
            && same_run_properties(&self.run_properties, request.paragraph_properties.default_text_run_properties())
    }
}

/// Whether two sets of run properties of shareable requests shape and draw a text the same
/// way, now and after any change to an object they refer to: brushes are compared by identity,
/// because the runs of a layout draw with the brush object they were made with.
fn same_run_properties(a: &Rc<dyn TextRunProperties>, b: &Rc<dyn TextRunProperties>) -> bool {
    if Rc::ptr_eq(a, b) {
        return true;
    }

    a.font_rendering_em_size().to_bits() == b.font_rendering_em_size().to_bits()
        && a.font_scale().to_bits() == b.font_scale().to_bits()
        && a.baseline_alignment() == b.baseline_alignment()
        && same_object(a.foreground_brush(), b.foreground_brush())
        && same_object(a.background_brush(), b.background_brush())
        && a.typeface() == b.typeface()
        && a.requested_font_family() == b.requested_font_family()
        && a.culture_info() == b.culture_info()
        // Neither has text decorations or font features (`is_shareable`).
}

fn same_object<T: ?Sized>(a: Option<&Rc<T>>, b: Option<&Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

struct Entry {
    key: Key,
    layout: Rc<TextLayout>,
    last_used: u64,
}

impl Entry {
    /// Whether only the cache holds the layout.
    fn is_idle(&self) -> bool {
        Rc::strong_count(&self.layout) == 1
    }

    /// Lets the layout go: released now when nothing else holds it, else when its last holder
    /// drops it.
    fn release(self) {
        if self.is_idle() {
            self.layout.release_shared();
        }
    }
}

/// The keys are hashes already.
#[derive(Default)]
struct HashKeyHasher(u64);

impl Hasher for HashKeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 << 8) | u64::from(*byte);
        }
    }

    fn write_u64(&mut self, value: u64) {
        self.0 = value;
    }
}

#[derive(Default)]
struct State {
    /// The glyph typeface generation the layouts were made in.
    generation: u64,
    /// The entries by the hash of their request; entries of one hash are told apart by key.
    entries: HashMap<u64, Vec<Entry>, BuildHasherDefault<HashKeyHasher>>,
    count: usize,
    tick: u64,
    /// The count at which layouts to let go are looked for next.
    sweep_at: usize,
    /// The text of the layouts added since layouts to let go were looked for.
    added_text: usize,
}

impl State {
    fn clear(&mut self) {
        for (_, bucket) in self.entries.drain() {
            for entry in bucket {
                entry.release();
            }
        }
        self.count = 0;
        self.sweep_at = SWEEP_INTERVAL;
        self.added_text = 0;
    }

    /// Whether enough was added to look for layouts to let go.
    fn is_sweep_due(&self) -> bool {
        self.count >= self.sweep_at.max(SWEEP_INTERVAL) || self.added_text >= IDLE_TEXT_CAPACITY / 4
    }

    /// Lets the idle layouts go that were used longest ago, down to [`IDLE_CAPACITY`] layouts
    /// and [`IDLE_TEXT_CAPACITY`] bytes of text.
    fn sweep(&mut self) {
        // When each idle layout was used last, and the length of its text.
        let mut idle: Vec<(u64, usize)> = self
            .entries
            .values()
            .flat_map(|bucket| bucket.iter())
            .filter(|entry| entry.is_idle())
            .map(|entry| (entry.last_used, entry.key.text.len()))
            .collect();

        // The first layout, from the one used last, that is beyond either capacity: it and
        // every idle layout used before it go. Ticks are unique.
        let mut cut = None;
        if idle.len() > IDLE_CAPACITY || idle.iter().map(|(_, length)| length).sum::<usize>() > IDLE_TEXT_CAPACITY {
            idle.sort_unstable_by(|a, b| b.0.cmp(&a.0));
            let mut text = 0;
            for (index, (last_used, length)) in idle.iter().enumerate() {
                text += length;
                if index >= IDLE_CAPACITY || text > IDLE_TEXT_CAPACITY {
                    cut = Some(*last_used);
                    break;
                }
            }
        }

        if let Some(cut) = cut {
            let mut released = 0;
            self.entries.retain(|_, bucket| {
                let mut index = 0;
                while index < bucket.len() {
                    if bucket[index].last_used <= cut && bucket[index].is_idle() {
                        bucket.swap_remove(index).release();
                        released += 1;
                    } else {
                        index += 1;
                    }
                }
                !bucket.is_empty()
            });
            self.count -= released;
            crate::diagnostics::perf_counters::add(
                crate::diagnostics::perf_counters::PerfCounter::TextLayoutCacheEvictions,
                released as u64,
            );
        }

        self.sweep_at = self.count + SWEEP_INTERVAL;
        self.added_text = 0;
    }
}

/// The plain text layouts of a font manager.
#[derive(Default)]
pub(crate) struct TextLayoutCache {
    state: RefCell<State>,
}

impl TextLayoutCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The layout of `request`: the one kept for it, else the one `create` makes, which is
    /// kept. `create` makes the layout of the text of the request in one run of the default
    /// run properties of its paragraph properties, with its trimming and constraint.
    pub(crate) fn get_or_create(
        &self,
        request: &PlainTextLayoutRequest<'_>,
        create: impl FnOnce() -> TextLayout,
    ) -> Rc<TextLayout> {
        if !request.is_shareable() {
            return Rc::new(create());
        }

        let hash = request.hash();
        let paragraph = ParagraphKey::new(request.paragraph_properties);
        let generation = crate::media::font_manager::glyph_typeface_generation();

        {
            let mut state = self.state.borrow_mut();
            if state.generation != generation {
                // The fonts changed: a text can resolve to other glyphs now.
                state.clear();
                state.generation = generation;
            }
            state.tick += 1;
            let tick = state.tick;
            if let Some(entry) = state
                .entries
                .get_mut(&hash)
                .and_then(|bucket| bucket.iter_mut().find(|entry| entry.key.matches(request, &paragraph)))
            {
                entry.last_used = tick;
                crate::perf_count!(TextLayoutCacheHits);
                return entry.layout.clone();
            }
        }

        // Not borrowed while the layout is made: making it can load fonts.
        crate::perf_count!(TextLayoutCacheMisses);
        let layout = Rc::new(create().into_shared());

        let mut state = self.state.borrow_mut();
        if state.generation != crate::media::font_manager::glyph_typeface_generation() {
            // Making the layout changed the fonts (a collection was loaded for it): the layout
            // is what the fonts gave on the way, so it goes to its caller only.
            return layout;
        }
        state.tick += 1;
        let entry = Entry { key: Key::new(request, paragraph), layout: layout.clone(), last_used: state.tick };
        state.entries.entry(hash).or_default().push(entry);
        state.count += 1;
        state.added_text += request.text.len();
        if state.is_sweep_due() {
            state.sweep();
        }

        layout
    }

    /// The number of layouts kept.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.state.borrow().count
    }

    /// The number of kept layouts nothing else holds.
    #[cfg(test)]
    pub(crate) fn idle_len(&self) -> usize {
        self.state.borrow().entries.values().flat_map(|bucket| bucket.iter()).filter(|entry| entry.is_idle()).count()
    }
}

impl Drop for TextLayoutCache {
    fn drop(&mut self) {
        self.state.get_mut().clear();
    }
}
