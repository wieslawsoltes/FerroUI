//! epaint (the text layout of egui): one `Fonts` whose definitions have Roboto Regular and nothing
//! else (no default fonts: the crate is built without `default_fonts`), kept for the life of the
//! target as an egui context keeps it. A string is a `LayoutJob::simple` (the job egui builds for
//! a label) at a wrap width, laid out into a `Galley` at one pixel per point.
//!
//! epaint keeps no shaping of a string apart from the galley of the whole job: the galley cache
//! keys on the job, which holds the text and the wrap width, so a new width is a new job and a
//! cache miss that lays the text out from the start. The cold path is `Fonts::layout_uncached`,
//! which bypasses the galley cache; the other paths go through it (`FontsView::layout_job`), with
//! `Fonts::begin_pass` before each pass over the corpus as egui calls it before each frame (it
//! evicts the galleys the previous pass did not use).

use crate::font::{FAMILY, FONT_SIZE, ROBOTO_REGULAR};
use crate::target::{TextTarget, Totals};
use crate::LibraryKind;
use epaint::text::{FontData, FontDefinitions, FontFamily, FontId, Fonts, Galley, LayoutJob, TextOptions, ViewportKey};
use epaint::Color32;
use std::sync::Arc;

/// One pixel a point, as for every library.
const PIXELS_PER_POINT: f32 = 1.0;

pub struct EpaintTarget {
    fonts: Fonts,
    options: TextOptions,
    font_id: FontId,
    texts: Vec<String>,
}

fn totals(galley: &Galley) -> Totals {
    Totals::new(galley.rows.len(), f64::from(galley.size().y))
}

impl EpaintTarget {
    fn job(&self, index: usize, width: f32) -> LayoutJob {
        LayoutJob::simple(self.texts[index].clone(), self.font_id.clone(), Color32::BLACK, width)
    }

    /// Every string through the galley cache at `width`, after `Fonts::begin_pass` when
    /// `begin_pass`.
    fn layout_cached(&mut self, width: f32, begin_pass: bool) -> Totals {
        if begin_pass {
            self.fonts.begin_pass(self.options, ViewportKey::default());
        }
        let Self { fonts, font_id, texts, .. } = self;
        let mut view = fonts.with_pixels_per_point(PIXELS_PER_POINT);
        let mut result = Totals::default();
        for text in texts.iter() {
            // egui's labels take their text by value: the job owns a copy.
            let job = LayoutJob::simple(text.clone(), font_id.clone(), Color32::BLACK, width);
            let galley = view.layout_job(job);
            result += std::hint::black_box(totals(&galley));
        }
        result
    }

    /// The galley cache at the same width without a pass in between: every string a cache hit,
    /// the cheapest way to get a laid-out galley from epaint (`rewrap_cached_same_width`).
    pub fn relayout_cached_without_pass(&mut self, width: f32) -> Totals {
        self.layout_cached(width, false)
    }
}

impl TextTarget for EpaintTarget {
    const KIND: LibraryKind = LibraryKind::Epaint;

    fn new(texts: Vec<String>) -> Self {
        let mut definitions = FontDefinitions::empty();
        definitions.font_data.insert(FAMILY.to_owned(), Arc::new(FontData::from_static(ROBOTO_REGULAR)));
        definitions.families.insert(FontFamily::Proportional, vec![FAMILY.to_owned()]);
        let options = TextOptions::default();
        let fonts = Fonts::new(options, definitions);
        Self { fonts, options, font_id: FontId::proportional(FONT_SIZE), texts }
    }

    fn len(&self) -> usize {
        self.texts.len()
    }

    fn cold_one(&mut self, index: usize, width: f32) -> Totals {
        let job = Arc::new(self.job(index, width));
        totals(&self.fonts.layout_uncached(PIXELS_PER_POINT, job))
    }

    fn prepare(&mut self) {
        let _ = self.layout_cached(crate::corpus::LAYOUT_WIDTH, true);
    }

    fn relayout(&mut self, width: f32) -> Totals {
        self.layout_cached(width, true)
    }
}
