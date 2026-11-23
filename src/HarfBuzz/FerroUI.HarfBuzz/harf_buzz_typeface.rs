use crate::hb;
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::{GlyphTypeface, ITextShaperTypeface};
use std::any::Any;
use std::cell::RefCell;

/// The HarfBuzz face and font of a glyph typeface.
///
/// The font tables are read from the platform typeface of the glyph
/// typeface without copying: each table blob keeps the table memory alive.
pub struct HarfBuzzTypeface {
    // The font is declared first: it is released before the face it was
    // created from.
    font: RefCell<Option<hb::Font>>,
    face: RefCell<Option<hb::Face>>,
}

impl HarfBuzzTypeface {
    /// Creates the shaper typeface of a glyph typeface.
    pub fn new(glyph_typeface: &GlyphTypeface) -> Self {
        let platform_typeface = glyph_typeface.platform_typeface().clone();

        // A typeface of a variable font is one instance of it: its glyphs are shaped at that
        // position of the variation space, which does not change, so it is set once here.
        let variation_settings = platform_typeface.variation_settings();

        let face = hb::Face::new(
            Box::new(move |tag| platform_typeface.try_get_table(OpenTypeTag::new(tag))),
            glyph_typeface.metrics().design_em_height as u32,
        );
        let mut font = hb::Font::new(&face);

        if !variation_settings.is_empty() {
            font.set_variations(variation_settings.variations());
        }

        Self { font: RefCell::new(Some(font)), face: RefCell::new(Some(face)) }
    }

    /// Runs `f` with the HarfBuzz font, which `f` may change (its point size).
    ///
    /// # Panics
    /// Panics when the typeface has been disposed.
    pub(crate) fn with_font<R>(&self, f: impl FnOnce(&mut hb::Font) -> R) -> R {
        let mut font = self.font.borrow_mut();
        f(font.as_mut().expect("HarfBuzzTypeface has been disposed"))
    }
}

impl ITextShaperTypeface for HarfBuzzTypeface {
    fn dispose(&self) {
        self.font.borrow_mut().take();
        self.face.borrow_mut().take();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
