use crate::media::text_formatting::{ITextDrawingSink, TextRun};
use crate::{Point, Size};

/// A text run that supports drawing content.
///
/// Implementors also return `Some(self)` from [`TextRun::as_drawable`].
pub trait DrawableTextRun: TextRun {
    /// Gets the size.
    fn size(&self) -> Size;

    /// Run baseline in ratio relative to run height.
    fn baseline(&self) -> f64;

    /// Draws the run at the given origin.
    fn draw(&self, drawing_context: &mut dyn ITextDrawingSink, origin: Point);
}
