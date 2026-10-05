use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::media::text_formatting::unicode::{CodepointEnumerator, GeneralCategory, GraphemeEnumerator};
use crate::media::text_formatting::{GlyphInfo, ShapedBuffer, TextShaperOptions};
use crate::media::{GlyphTypeface, ITextShaperTypeface};
use crate::platform::ITextShaperImpl;
use crate::utilities::ReadOnlyMemory;

/// A deterministic text shaper.
///
/// * one glyph per codepoint; the glyph is whatever the font's cmap gives
///   (0 for an unmapped codepoint);
/// * the cluster of a glyph is the UTF-16 index of the grapheme cluster it
///   belongs to (marks merge into their base, CR LF is one cluster), as a
///   real shaper reports;
/// * the advance is the font's advance scaled to the em size plus the letter
///   spacing; marks, controls, line breaks and default ignorables have no
///   advance; a tab is four spaces wide or the incremental tab width;
/// * odd bidi levels give the glyphs in visual order (descending clusters);
/// * configured ligatures shape a sequence of characters to one glyph and
///   one cluster;
/// * optionally, line break characters shape to no glyph at all.
pub struct TestTextShaperImpl {
    ligatures: RefCell<Vec<Vec<u16>>>,
    drop_line_breaks: Cell<bool>,
    shape_calls: RefCell<Vec<String>>,
}

#[allow(dead_code)] // the full harness API is kept for the tests built on top
impl TestTextShaperImpl {
    pub fn new() -> Self {
        Self { ligatures: RefCell::new(Vec::new()), drop_line_breaks: Cell::new(false), shape_calls: RefCell::new(Vec::new()) }
    }

    /// Makes the shaper produce one glyph (and one cluster) for `text`.
    pub fn add_ligature(&self, text: &str) {
        self.ligatures.borrow_mut().push(text.encode_utf16().collect());
    }

    /// Makes the shaper produce no glyph for line break characters, as a real
    /// shaper does for default ignorables a font cannot hide behind a space glyph.
    pub fn set_drop_line_breaks(&self, value: bool) {
        self.drop_line_breaks.set(value);
    }

    /// The texts shaped so far.
    pub fn shape_calls(&self) -> Vec<String> {
        self.shape_calls.borrow().clone()
    }

    /// The number of shape calls so far.
    pub fn shape_call_count(&self) -> usize {
        self.shape_calls.borrow().len()
    }

    fn ligature_length_at(&self, text: &[u16]) -> usize {
        self.ligatures
            .borrow()
            .iter()
            .filter(|ligature| text.starts_with(ligature))
            .map(|ligature| ligature.len())
            .max()
            .unwrap_or(0)
    }
}

impl ITextShaperImpl for TestTextShaperImpl {
    fn shape_text(&self, text: &ReadOnlyMemory<u16>, options: &TextShaperOptions) -> Rc<ShapedBuffer> {
        let span = text.span();

        self.shape_calls.borrow_mut().push(String::from_utf16_lossy(span));

        let glyph_typeface = options.glyph_typeface();
        let font_rendering_em_size = options.font_rendering_em_size();
        let bidi_level = options.bidi_level();

        if span.is_empty() {
            return ShapedBuffer::new(text.clone(), 0, glyph_typeface.clone(), font_rendering_em_size, bidi_level);
        }

        let scale = font_rendering_em_size / glyph_typeface.metrics().design_em_height as f64;
        let map = glyph_typeface.character_to_glyph_map();

        let advance_of = |glyph: u16| -> f64 {
            glyph_typeface.try_get_horizontal_glyph_advance(glyph).unwrap_or(0) as f64 * scale
        };

        let mut glyphs: Vec<GlyphInfo> = Vec::with_capacity(span.len());

        let mut offset = 0usize;

        while offset < span.len() {
            let ligature_length = self.ligature_length_at(&span[offset..]);

            if ligature_length > 0 {
                let glyph = map.get_glyph(span[offset] as i32);

                glyphs.push(GlyphInfo::new(glyph, offset as i32, advance_of(glyph) + options.letter_spacing()));

                offset += ligature_length;

                continue;
            }

            let grapheme_length =
                GraphemeEnumerator::new(&span[offset..]).move_next().map_or(1, |grapheme| grapheme.length());

            let mut is_base = true;

            for codepoint in CodepointEnumerator::new(&span[offset..offset + grapheme_length]) {
                if self.drop_line_breaks.get() && codepoint.is_break_char() {
                    continue;
                }

                let mut glyph = map.get_glyph(i32::from(codepoint));

                let advance = if codepoint.value() == '\t' as u32 {
                    glyph = map.get_glyph(' ' as i32);

                    if options.incremental_tab_width() > 0.0 {
                        options.incremental_tab_width()
                    } else {
                        4.0 * advance_of(glyph)
                    }
                } else if !is_base
                    || codepoint.is_break_char()
                    || codepoint.general_category() == GeneralCategory::Control
                    || codepoint.is_default_ignorable()
                {
                    options.letter_spacing()
                } else {
                    advance_of(glyph) + options.letter_spacing()
                };

                glyphs.push(GlyphInfo::new(glyph, offset as i32, advance));

                is_base = false;
            }

            offset += grapheme_length;
        }

        if (bidi_level & 1) != 0 {
            glyphs.reverse();
        }

        let shaped_buffer =
            ShapedBuffer::new(text.clone(), glyphs.len(), glyph_typeface.clone(), font_rendering_em_size, bidi_level);

        for (i, glyph) in glyphs.into_iter().enumerate() {
            shaped_buffer.set(i, glyph);
        }

        shaped_buffer
    }

    fn create_typeface(&self, _glyph_typeface: &GlyphTypeface) -> Rc<dyn ITextShaperTypeface> {
        Rc::new(TestTextShaperTypeface)
    }
}

struct TestTextShaperTypeface;

impl ITextShaperTypeface for TestTextShaperTypeface {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
