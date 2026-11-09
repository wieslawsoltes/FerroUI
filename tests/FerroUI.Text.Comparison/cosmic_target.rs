//! cosmic-text: a `FontSystem` over a font database that has Roboto Regular loaded from memory and
//! nothing else (`new_with_locale_and_db`, so that no system font is scanned), kept for the life
//! of the target as an application keeps it. A string is a `Buffer` (one per string, as a text
//! widget has one), wrapping by word and by glyph when a word does not fit (the default), with
//! the line height of the font's `hhea` table (cosmic-text has no default line height); a width
//! is `Buffer::set_size` and `Buffer::shape_until_scroll`, which lays the lines out again from
//! the shaping the buffer keeps.

use crate::font::{hhea_line_height_in_ems, FAMILY, FONT_SIZE, ROBOTO_REGULAR};
use crate::target::{TextTarget, Totals};
use crate::LibraryKind;
use cosmic_text::{fontdb, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Wrap};

pub struct CosmicTextTarget {
    font_system: FontSystem,
    metrics: Metrics,
    texts: Vec<String>,
    /// The prepared buffers, one per string: shaped once.
    buffers: Vec<Buffer>,
}

fn attrs() -> Attrs<'static> {
    Attrs::new().family(Family::Name(FAMILY))
}

/// A buffer of `text` at `width`, laid out.
fn buffer(font_system: &mut FontSystem, metrics: Metrics, text: &str, width: f32) -> Buffer {
    let mut buffer = Buffer::new_empty(metrics);
    buffer.set_wrap(Wrap::WordOrGlyph);
    buffer.set_size(Some(width), None);
    buffer.set_text(text, &attrs(), Shaping::Advanced, None);
    buffer.shape_until_scroll(font_system, false);
    buffer
}

fn totals(buffer: &Buffer) -> Totals {
    let mut lines = 0;
    let mut bottom = 0.0_f32;
    for run in buffer.layout_runs() {
        lines += 1;
        bottom = bottom.max(run.line_top + run.line_height);
    }
    Totals::new(lines, f64::from(bottom))
}

impl TextTarget for CosmicTextTarget {
    const KIND: LibraryKind = LibraryKind::CosmicText;

    fn new(texts: Vec<String>) -> Self {
        let mut db = fontdb::Database::new();
        db.load_font_data(ROBOTO_REGULAR.to_vec());
        db.set_sans_serif_family(FAMILY);
        let font_system = FontSystem::new_with_locale_and_db("en-US".to_string(), db);
        let metrics = Metrics::relative(FONT_SIZE, hhea_line_height_in_ems());
        Self { font_system, metrics, texts, buffers: Vec::new() }
    }

    fn len(&self) -> usize {
        self.texts.len()
    }

    fn cold_one(&mut self, index: usize, width: f32) -> Totals {
        let buffer = buffer(&mut self.font_system, self.metrics, &self.texts[index], width);
        totals(&buffer)
    }

    fn prepare(&mut self) {
        let width = crate::corpus::LAYOUT_WIDTH;
        let mut buffers = Vec::with_capacity(self.texts.len());
        for text in &self.texts {
            buffers.push(buffer(&mut self.font_system, self.metrics, text, width));
        }
        self.buffers = buffers;
    }

    fn relayout(&mut self, width: f32) -> Totals {
        let mut result = Totals::default();
        for buffer in &mut self.buffers {
            // When the width did not change, nothing is dirty and the buffer keeps its layout.
            buffer.set_size(Some(width), None);
            buffer.shape_until_scroll(&mut self.font_system, false);
            result += std::hint::black_box(totals(buffer));
        }
        result
    }
}
