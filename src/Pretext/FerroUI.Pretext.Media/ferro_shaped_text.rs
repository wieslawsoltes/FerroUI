use std::rc::Rc;

use ferroui_base::media::text_formatting::ITextDrawingSink;
use ferroui_base::media::{GlyphRun, IBrush};
use ferroui_base::{Matrix, Point};
use ferroui_pretext::PretextShapedRun;

/// Positioned portable glyph data together with its actual native font faces.
///
/// Native runs are already in visual order and have their horizontal baseline
/// origins set. Keeping this value keeps their glyph typefaces alive.
pub struct FerroShapedText {
    pub(crate) shaped_run: PretextShapedRun,
    pub(crate) glyph_runs: Vec<Rc<GlyphRun>>,
}

impl FerroShapedText {
    pub fn shaped_run(&self) -> &PretextShapedRun {
        &self.shaped_run
    }

    pub fn glyph_runs(&self) -> &[Rc<GlyphRun>] {
        &self.glyph_runs
    }

    pub fn into_shaped_run(self) -> PretextShapedRun {
        self.shaped_run
    }

    /// Draws the retained glyph runs at the requested baseline origin.
    pub fn draw(
        &self,
        sink: &mut dyn ITextDrawingSink,
        foreground: Option<&Rc<dyn IBrush>>,
        baseline_origin: Point,
    ) {
        sink.push_transform(Matrix::create_translation(
            baseline_origin.x,
            baseline_origin.y,
        ));
        for run in &self.glyph_runs {
            sink.draw_glyph_run(foreground, run);
        }
        sink.pop_transform();
    }
}
