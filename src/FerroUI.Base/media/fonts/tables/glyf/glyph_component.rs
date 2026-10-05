use super::composite_flags::CompositeFlags;

/// One component of a composite glyph.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct GlyphComponent {
    pub flags: CompositeFlags,
    pub glyph_index: u16,
    pub arg1: i16,
    pub arg2: i16,
    pub scale: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub scale01: f32,
    pub scale10: f32,
}
