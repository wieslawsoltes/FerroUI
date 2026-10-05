use crate::harf_buzz_typeface::HarfBuzzTypeface;
use crate::hb;
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::text_formatting::unicode::Codepoint;
use ferroui_base::media::text_formatting::{GlyphInfo, ShapedBuffer, TextShaperOptions};
use ferroui_base::media::{GlyphTypeface, ITextShaperTypeface};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::{CultureInfo, ReadOnlyMemory};
use ferroui_base::Vector;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// The shaping buffer, reused between calls.
    static BUFFER: RefCell<hb::Buffer> = RefCell::new(hb::Buffer::new());
}

const ZERO_WIDTH_NON_JOINER: u32 = 0x200C;

/// A text shaper backed by HarfBuzz.
#[derive(Default)]
pub struct HarfBuzzTextShaper;

impl HarfBuzzTextShaper {
    /// Creates the text shaper.
    pub fn new() -> Self {
        Self
    }

    /// Replaces a trailing line break by zero width non joiners so that it
    /// shapes to nothing; a `\r\n` pair becomes one cluster.
    fn merge_break_pair(buffer: &mut hb::Buffer) {
        let glyph_infos = buffer.glyph_infos_mut();
        let length = glyph_infos.len();

        if length == 0 {
            return;
        }

        let second = glyph_infos[length - 1];

        if !Codepoint::new(second.codepoint).is_break_char() {
            return;
        }

        if length > 1 && glyph_infos[length - 2].codepoint == '\r' as u32 && second.codepoint == '\n' as u32 {
            let first_cluster = glyph_infos[length - 2].cluster;

            glyph_infos[length - 2].codepoint = ZERO_WIDTH_NON_JOINER;

            glyph_infos[length - 1].codepoint = ZERO_WIDTH_NON_JOINER;
            glyph_infos[length - 1].cluster = first_cluster;
        } else {
            glyph_infos[length - 1].codepoint = ZERO_WIDTH_NON_JOINER;
        }
    }

    fn get_glyph_offset(position: &hb::GlyphPosition, text_scale: f64) -> Vector {
        let offset_x = position.x_offset as f64 * text_scale;

        let offset_y = -(position.y_offset as f64) * text_scale;

        Vector::new(offset_x, offset_y)
    }

    fn get_glyph_advance(position: &hb::GlyphPosition, text_scale: f64) -> f64 {
        // Depends on the direction of the layout: vertical layout would use
        // the y advance.
        position.x_advance as f64 * text_scale
    }

    fn get_features(options: &TextShaperOptions) -> Vec<hb::Feature> {
        let Some(font_features) = options.font_features() else {
            return Vec::new();
        };

        font_features
            .iter()
            .map(|font_feature| hb::Feature {
                tag: OpenTypeTag::parse(&font_feature.tag).value(),
                value: font_feature.value as u32,
                start: font_feature.start as u32,
                end: font_feature.end as u32,
            })
            .collect()
    }
}

impl ITextShaperImpl for HarfBuzzTextShaper {
    fn shape_text(&self, text: &ReadOnlyMemory<u16>, options: &TextShaperOptions) -> Rc<ShapedBuffer> {
        let glyph_typeface = options.glyph_typeface();
        let font_rendering_em_size = options.font_rendering_em_size();
        let bidi_level = options.bidi_level();

        if text.is_empty() {
            return ShapedBuffer::new(text.clone(), 0, glyph_typeface.clone(), font_rendering_em_size, bidi_level);
        }

        let text_shaper_typeface = glyph_typeface.text_shaper_typeface();
        let harf_buzz_typeface = text_shaper_typeface
            .as_any()
            .downcast_ref::<HarfBuzzTypeface>()
            .unwrap_or_else(|| panic!("The provided GlyphTypeface is not supported by this text shaper."));

        let culture = options.culture();

        // HarfBuzz needs the surrounding characters to correctly shape the
        // text.
        let (containing_text, start): (&[u16], usize) = match text.owner() {
            Some(owner) => (owner, text.offset_in_owner()),
            None => (text.span(), 0),
        };
        let length = text.len();

        BUFFER.with(|buffer| {
            let mut buffer = buffer.borrow_mut();

            buffer.reset();

            buffer.add_utf16(containing_text, start, length);

            Self::merge_break_pair(&mut buffer);

            buffer.guess_segment_properties();

            buffer.set_direction(if (bidi_level & 1) == 0 {
                hb::Direction::LeftToRight
            } else {
                hb::Direction::RightToLeft
            });

            let current_culture;
            let used_culture = match culture {
                Some(culture) => culture,
                None => {
                    current_culture = CultureInfo::current_culture();
                    &current_culture
                }
            };

            buffer.set_language(used_culture.two_letter_iso_language_name());

            let features = Self::get_features(options);

            // HarfBuzz produces glyphs in visual order for RTL by default:
            // the first glyph in the buffer is the leftmost visual glyph
            // (highest cluster value). LTR output already has clusters in
            // ascending (logical = visual) order. That order is preserved in
            // the shaped buffer so downstream rendering and hit-testing can
            // operate on visual-order glyphs without an extra bidi reversal
            // step.
            let scale_x = harf_buzz_typeface.with_font(|font| {
                font.shape(&mut buffer, &features);
                font.scale().0
            });

            let text_scale = font_rendering_em_size / scale_x as f64;

            let buffer_length = buffer.len();

            let shaped_buffer =
                ShapedBuffer::new(text.clone(), buffer_length, glyph_typeface.clone(), font_rendering_em_size, bidi_level);

            let glyph_infos = buffer.glyph_infos();
            let glyph_positions = buffer.glyph_positions();

            for i in 0..buffer_length {
                let source_info = &glyph_infos[i];

                let mut glyph_index = source_info.codepoint as u16;

                let original_cluster = source_info.cluster as usize;

                let glyph_cluster = original_cluster as i32 - start as i32;

                let mut glyph_advance =
                    Self::get_glyph_advance(&glyph_positions[i], text_scale) + options.letter_spacing();

                let glyph_offset = Self::get_glyph_offset(&glyph_positions[i], text_scale);

                if original_cluster < containing_text.len() && containing_text[original_cluster] == '\t' as u16 {
                    glyph_index = glyph_typeface.character_to_glyph_map().get_glyph(' ' as i32);

                    if options.incremental_tab_width() > 0.0 {
                        glyph_advance = options.incremental_tab_width();
                    } else {
                        let advance = glyph_typeface.try_get_horizontal_glyph_advance(glyph_index).unwrap_or(0);
                        glyph_advance = 4.0 * advance as f64 * text_scale;
                    }
                }

                shaped_buffer.set(i, GlyphInfo::with_offset(glyph_index, glyph_cluster, glyph_advance, glyph_offset));
            }

            shaped_buffer
        })
    }

    fn create_typeface(&self, glyph_typeface: &GlyphTypeface) -> Rc<dyn ITextShaperTypeface> {
        Rc::new(HarfBuzzTypeface::new(glyph_typeface))
    }
}
