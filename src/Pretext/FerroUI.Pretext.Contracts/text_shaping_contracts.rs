use std::rc::Rc;

use crate::PretextError;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PretextGlyphRunKind {
    Shaped = 0,
    Mapped = 1,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PretextTextDirection {
    #[default]
    Auto = 0,
    LeftToRight = 1,
    RightToLeft = 2,
}

/// A value snapshot: changing the caller's options cannot mutate cached output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PretextShapeOptions {
    pub direction: PretextTextDirection,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PretextShapedGlyph {
    pub glyph_id: u32,
    /// UTF-16 code-unit offset into the shaped input.
    pub cluster: usize,
    pub x: f64,
    pub y: f64,
    pub x_advance: f64,
    pub y_advance: f64,
    pub x_offset: f64,
    pub y_offset: f64,
    pub font_run_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PretextShapedFontRun {
    pub index: usize,
    pub font: String,
    pub first_glyph_index: usize,
    pub glyph_count: usize,
}

/// Immutable positioned glyphs and the font runs that produced them.
#[derive(Clone, Debug, PartialEq)]
pub struct PretextShapedRun {
    kind: PretextGlyphRunKind,
    glyphs: Vec<PretextShapedGlyph>,
    font_runs: Vec<PretextShapedFontRun>,
    advance_x: f64,
    advance_y: f64,
    has_missing_glyphs: bool,
}

impl PretextShapedRun {
    pub fn new(
        kind: PretextGlyphRunKind,
        glyphs: Vec<PretextShapedGlyph>,
        font_runs: Vec<PretextShapedFontRun>,
        advance_x: f64,
        advance_y: f64,
    ) -> Self {
        let has_missing_glyphs = glyphs.iter().any(|glyph| glyph.glyph_id == 0);
        Self { kind, glyphs, font_runs, advance_x, advance_y, has_missing_glyphs }
    }

    pub fn kind(&self) -> PretextGlyphRunKind { self.kind }
    pub fn glyphs(&self) -> &[PretextShapedGlyph] { &self.glyphs }
    pub fn font_runs(&self) -> &[PretextShapedFontRun] { &self.font_runs }
    pub fn advance_x(&self) -> f64 { self.advance_x }
    pub fn advance_y(&self) -> f64 { self.advance_y }
    pub fn has_missing_glyphs(&self) -> bool { self.has_missing_glyphs }
    pub fn is_shaped(&self) -> bool { self.kind == PretextGlyphRunKind::Shaped }
}

pub trait IPretextTextShaper {
    fn shape_text(&self, text: &str, options: PretextShapeOptions) -> Result<PretextShapedRun, PretextError>;
}

pub trait IPretextTextShaperFactory {
    fn name(&self) -> &str;
    fn is_supported(&self) -> bool { true }
    fn priority(&self) -> i32 { 0 }
    fn create_shaper(&self, font: &str) -> Result<Rc<dyn IPretextTextShaper>, PretextError>;
}
