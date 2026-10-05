use std::rc::Rc;

use crate::media::text_formatting::{ShapedBuffer, TextShaperOptions};
use crate::media::{GlyphTypeface, ITextShaperTypeface};
use crate::utilities::ReadOnlyMemory;

/// An abstraction that is used to produce shaped text.
pub trait ITextShaperImpl: 'static {
    /// Shapes the specified region within the text and returns a shaped buffer.
    ///
    /// `text` is UTF-16; glyph clusters in the result are UTF-16 code unit
    /// offsets into `text`.
    fn shape_text(&self, text: &ReadOnlyMemory<u16>, options: &TextShaperOptions) -> Rc<ShapedBuffer>;

    /// Creates a text shaper typeface based on the specified glyph typeface.
    fn create_typeface(&self, glyph_typeface: &GlyphTypeface) -> Rc<dyn ITextShaperTypeface>;
}
