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

        let face = hb::Face::new(
            Box::new(move |tag| platform_typeface.try_get_table(OpenTypeTag::new(tag))),
            glyph_typeface.metrics().design_em_height as u32,
        );
        let font = hb::Font::new(&face);

        Self { font: RefCell::new(Some(font)), face: RefCell::new(Some(face)) }
    }

    /// Runs `f` with the HarfBuzz font.
    ///
    /// # Panics
    /// Panics when the typeface has been disposed.
    pub(crate) fn with_font<R>(&self, f: impl FnOnce(&hb::Font) -> R) -> R {
        let font = self.font.borrow();
        f(font.as_ref().expect("HarfBuzzTypeface has been disposed"))
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
