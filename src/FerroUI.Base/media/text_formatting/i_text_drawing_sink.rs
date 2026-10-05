use std::rc::Rc;

use crate::media::{GlyphRun, IBrush, IPen};
use crate::{Matrix, Point, Rect};

/// The drawing calls text needs.
///
/// Upstream draws text lines, glyph runs and decorations onto a
/// `DrawingContext`. Text is written against this trait, which mirrors the
/// corresponding `DrawingContext` members one to one; the drawing context
/// implements it, so a drawing context is passed wherever a sink is taken.
pub trait ITextDrawingSink {
    /// Draws a glyph run (`DrawingContext.DrawGlyphRun(IBrush? foreground, GlyphRun glyphRun)`).
    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>);

    /// Draws a rectangle with the specified brush and pen
    /// (`DrawingContext.DrawRectangle(IBrush? brush, IPen? pen, Rect rect)`).
    fn draw_rectangle(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect);

    /// Draws a line (`DrawingContext.DrawLine(IPen pen, Point p1, Point p2)`).
    fn draw_line(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point);

    /// Pushes a matrix transformation (`DrawingContext.PushTransform(Matrix matrix)`).
    /// Every push is balanced by one [`ITextDrawingSink::pop_transform`].
    fn push_transform(&mut self, matrix: Matrix);

    /// Pops the transformation pushed last (disposing upstream's `PushedState`).
    fn pop_transform(&mut self);
}
