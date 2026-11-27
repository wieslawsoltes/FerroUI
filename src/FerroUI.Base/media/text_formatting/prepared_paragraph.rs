//! A text shaped once and laid out at any width (a FerroUI addition, steps 3 and 6 of
//! docs/porting/text-layout-performance.md).
//!
//! [`PreparedParagraph`] keeps the shaping of its text and the break opportunities found with
//! it, so a new width costs the wrapping and the line objects and no shaping. The lines are made
//! by the text formatter, the one way a line is made, so they are the lines of a [`TextLayout`]
//! made from nothing at that width. Their ink, and with it their platform glyph runs, is measured
//! when it is drawn or asked for.
//!
//! [`PreparedParagraph::shared`] is the service over it: one prepared paragraph for a text and
//! its properties, for any control or application that lays the text out at widths of its own.
//! Nothing in the framework calls it, so an application that does not use it does not link it.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::media::text_formatting::text_layout_cache::{is_plain_paragraph, same_run_properties, ParagraphKey};
use crate::media::text_formatting::{
    FormattedTextSource, ITextSource, TextLayout, TextParagraphProperties, TextRunCache, TextRunProperties,
};
use crate::media::{FlowDirection, FontManager, TextAlignment};
use crate::utilities::ReadOnlyMemory;
use crate::{FerroLocator, LocatorExtensions};

/// The prepared paragraphs nothing else holds that [`PreparedParagraph::shared`] keeps.
pub(crate) const SHARED_CAPACITY: usize = 512;

/// The text (in UTF-8 bytes) of the prepared paragraphs that are kept.
pub(crate) const SHARED_TEXT_CAPACITY: usize = 1 << 20;

/// The longest text (in UTF-8 bytes) that is shared.
pub(crate) const SHARED_MAX_TEXT_LENGTH: usize = 64 * 1024;

/// How many texts seen once are remembered for their second use.
const SEEN_CAPACITY: usize = 4096;

/// A line of a [`PreparedParagraph`] at a width: its range of the text and its size, without a
/// line object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedLine {
    /// The first text source index of the line (UTF-16 code units).
    pub start: i32,
    /// The length of the line (UTF-16 code units), with its trailing white space and line break.
    pub length: i32,
    /// The width of the line without its trailing white space.
    pub width: f64,
    /// The width of the line with its trailing white space.
    pub width_including_trailing_whitespace: f64,
    /// The height of the line.
    pub height: f64,
    /// The distance from the top of the line to its baseline.
    pub baseline: f64,
}

/// A text shaped once, to be laid out at any width.
pub struct PreparedParagraph {
    text_source: Rc<dyn ITextSource>,
    paragraph_properties: Rc<dyn TextParagraphProperties>,
    text_run_cache: Rc<TextRunCache>,
    /// The last layout. A width inside its hold has the same lines.
    current: RefCell<Option<Rc<TextLayout>>>,
    natural_width: Cell<Option<f64>>,
}

impl PreparedParagraph {
    /// Prepares `text`, in the default run properties of `paragraph_properties`.
    pub fn new(text: &str, paragraph_properties: Rc<dyn TextParagraphProperties>) -> Self {
        let text_source: Rc<dyn ITextSource> = Rc::new(FormattedTextSource::new(
            ReadOnlyMemory::<u16>::from_str(text),
            paragraph_properties.default_text_run_properties().clone(),
            None,
        ));
        Self::from_text_source(text_source, paragraph_properties)
    }

    /// Prepares the text of `text_source`.
    pub fn from_text_source(
        text_source: Rc<dyn ITextSource>,
        paragraph_properties: Rc<dyn TextParagraphProperties>,
    ) -> Self {
        Self {
            text_source,
            paragraph_properties,
            text_run_cache: Rc::new(TextRunCache::new()),
            current: RefCell::new(None),
            natural_width: Cell::new(None),
        }
    }

    /// The prepared paragraph of `text` with these properties, which every asker holds.
    ///
    /// A text is kept from its second use, so a text that is seen once costs a lookup. A
    /// paragraph with text decorations, font features or tab stops, whose collections can change
    /// after it is made, and a very long text get a paragraph of their own. Of the paragraphs
    /// nothing else holds, those used last are kept; a change of the fonts lets all of them go.
    pub fn shared(text: &str, paragraph_properties: &Rc<dyn TextParagraphProperties>) -> Rc<Self> {
        let create = || Rc::new(Self::new(text, paragraph_properties.clone()));
        if text.len() > SHARED_MAX_TEXT_LENGTH || !is_plain_paragraph(&**paragraph_properties) {
            return create();
        }
        let Some(font_manager) = FerroLocator::current().get_service::<FontManager>() else {
            return create();
        };
        font_manager.prepared_paragraphs().get_or_create(text, paragraph_properties, create)
    }

    /// The layout of the paragraph at `width`. A width at which the last layout has the same
    /// lines gets that layout, whose [`max_width`](TextLayout::max_width) is the width it was
    /// made for. The layout stays usable after the paragraph is laid out at another width.
    pub fn layout(&self, width: f64) -> Rc<TextLayout> {
        let mut current = self.current.borrow_mut();
        if let Some(layout) = current.as_ref().filter(|layout| self.holds(layout, width)) {
            return layout.clone();
        }
        // A layout a caller still holds is left to it: its lines stay valid.
        if let Some(previous) = current.take().and_then(|layout| Rc::try_unwrap(layout).ok()) {
            previous.dispose();
        }
        let layout = Rc::new(self.make_layout(width));
        *current = Some(layout.clone());
        layout
    }

    /// The lines of the paragraph at `width`.
    pub fn lines(&self, width: f64) -> Vec<PreparedLine> {
        self.layout(width)
            .text_lines()
            .iter()
            .map(|line| PreparedLine {
                start: line.first_text_source_index(),
                length: line.length(),
                width: line.width(),
                width_including_trailing_whitespace: line.width_including_trailing_whitespace(),
                height: line.height(),
                baseline: line.baseline(),
            })
            .collect()
    }

    /// The number of lines of the paragraph at `width`.
    pub fn line_count(&self, width: f64) -> usize {
        self.layout(width).text_lines().len()
    }

    /// The height of the paragraph at `width`.
    pub fn height(&self, width: f64) -> f64 {
        self.layout(width).height()
    }

    /// The width of the paragraph without a constraint: its widest line, without trailing white
    /// space, when only its required breaks end a line.
    pub fn natural_width(&self) -> f64 {
        if let Some(width) = self.natural_width.get() {
            return width;
        }
        let layout = self.make_layout(f64::INFINITY);
        let width = layout.width();
        layout.dispose();
        self.natural_width.set(Some(width));
        width
    }

    fn make_layout(&self, width: f64) -> TextLayout {
        TextLayout::from_text_source(
            self.text_source.clone(),
            self.paragraph_properties.clone(),
            None,
            width,
            f64::INFINITY,
            0,
            Some(self.text_run_cache.clone()),
        )
    }

    /// Whether `layout` is the layout at `width`: made for it, or holding it with lines that do
    /// not move with the width (an alignment to the left).
    fn holds(&self, layout: &TextLayout, width: f64) -> bool {
        if layout.max_width().to_bits() == width.to_bits() {
            return true;
        }
        let properties = &self.paragraph_properties;
        properties.flow_direction() == FlowDirection::LeftToRight
            && matches!(properties.text_alignment(), TextAlignment::Left | TextAlignment::Start)
            && layout.holds_width(width)
    }
}

impl Drop for PreparedParagraph {
    fn drop(&mut self) {
        if let Some(layout) = self.current.get_mut().take().and_then(|layout| Rc::try_unwrap(layout).ok()) {
            layout.dispose();
        }
        self.text_run_cache.invalidate();
    }
}

struct Entry {
    text: Box<str>,
    run_properties: Rc<dyn TextRunProperties>,
    paragraph: ParagraphKey,
    prepared: Rc<PreparedParagraph>,
    last_used: u64,
}

#[derive(Default)]
struct State {
    entries: HashMap<u64, Vec<Entry>>,
    seen: HashSet<u64>,
    count: usize,
    text_bytes: usize,
    tick: u64,
    generation: u64,
}

impl State {
    fn clear(&mut self) {
        self.entries.clear();
        self.seen.clear();
        self.count = 0;
        self.text_bytes = 0;
    }

    /// Lets go of the paragraphs nothing else holds that were used longest ago, until the rest
    /// is within the capacity.
    fn trim(&mut self) {
        if self.count <= SHARED_CAPACITY && self.text_bytes <= SHARED_TEXT_CAPACITY {
            return;
        }
        let mut idle: Vec<(u64, u64)> = self
            .entries
            .iter()
            .flat_map(|(hash, bucket)| bucket.iter().map(move |entry| (entry, *hash)))
            .filter(|(entry, _)| Rc::strong_count(&entry.prepared) == 1)
            .map(|(entry, hash)| (entry.last_used, hash))
            .collect();
        idle.sort_unstable();
        for (last_used, hash) in idle {
            if self.count <= SHARED_CAPACITY && self.text_bytes <= SHARED_TEXT_CAPACITY {
                break;
            }
            let Some(bucket) = self.entries.get_mut(&hash) else { continue };
            let Some(index) = bucket.iter().position(|entry| entry.last_used == last_used) else { continue };
            let entry = bucket.swap_remove(index);
            if bucket.is_empty() {
                self.entries.remove(&hash);
            }
            self.count -= 1;
            self.text_bytes -= entry.text.len();
        }
    }
}

/// The prepared paragraphs a font manager keeps ([`PreparedParagraph::shared`]).
#[derive(Default)]
pub(crate) struct PreparedParagraphs {
    state: RefCell<State>,
}

impl PreparedParagraphs {
    /// The set as the font manager stores it: behind `Any`, so the font manager of an application
    /// that never asks for a shared paragraph links none of this.
    pub(crate) fn boxed() -> Box<dyn Any> {
        Box::<Self>::default()
    }

    fn get_or_create(
        &self,
        text: &str,
        paragraph_properties: &Rc<dyn TextParagraphProperties>,
        create: impl FnOnce() -> Rc<PreparedParagraph>,
    ) -> Rc<PreparedParagraph> {
        let run_properties = paragraph_properties.default_text_run_properties();
        let paragraph = ParagraphKey::new(&**paragraph_properties);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        run_properties.font_rendering_em_size().to_bits().hash(&mut hasher);
        let hash = hasher.finish();
        let generation = crate::media::font_manager::glyph_typeface_generation();

        let mut state = self.state.borrow_mut();
        if state.generation != generation {
            // The fonts changed: a text can resolve to other glyphs now.
            state.clear();
            state.generation = generation;
        }
        state.tick += 1;
        let tick = state.tick;
        if let Some(entry) = state.entries.get_mut(&hash).and_then(|bucket| {
            bucket.iter_mut().find(|entry| {
                *entry.text == *text
                    && entry.paragraph == paragraph
                    && same_run_properties(&entry.run_properties, run_properties)
            })
        }) {
            entry.last_used = tick;
            return entry.prepared.clone();
        }

        // A text is kept from its second use.
        if state.seen.len() >= SEEN_CAPACITY {
            state.seen.clear();
        }
        if state.seen.insert(hash) {
            drop(state);
            return create();
        }

        let prepared = create();
        state.entries.entry(hash).or_default().push(Entry {
            text: text.into(),
            run_properties: run_properties.clone(),
            paragraph,
            prepared: prepared.clone(),
            last_used: tick,
        });
        state.count += 1;
        state.text_bytes += text.len();
        state.trim();
        prepared
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.state.borrow().count
    }
}
