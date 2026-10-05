use std::cell::RefCell;
use std::rc::Rc;

use crate::media::text_formatting::ITextDrawingSink;
use crate::media::{GlyphRun, IBrush, IPen};
use crate::{Matrix, Point, Rect};

/// One call recorded by [`RecordingDrawingSink`].
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCall {
    /// A glyph run with its text and the translation in effect.
    GlyphRun { text: String, origin: Point },
    Rectangle(Rect),
    Line(Point, Point),
}

/// A drawing sink that records what text draws.
#[derive(Default)]
pub struct RecordingDrawingSink {
    calls: RefCell<Vec<DrawCall>>,
    transforms: RefCell<Vec<Matrix>>,
}

impl RecordingDrawingSink {
    pub fn new() -> Self {
        Self::default()
    }

    /// The calls recorded so far.
    pub fn calls(&self) -> Vec<DrawCall> {
        self.calls.borrow().clone()
    }

    /// The number of transforms that were pushed and not popped.
    pub fn transform_depth(&self) -> usize {
        self.transforms.borrow().len()
    }

    fn origin(&self) -> Point {
        self.transforms.borrow().iter().fold(Point::new(0.0, 0.0), |origin, matrix| origin.transform(*matrix))
    }
}

impl ITextDrawingSink for RecordingDrawingSink {
    fn draw_glyph_run(&mut self, _foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        let text = glyph_run.characters().to_string_lossy();
        let origin = self.origin();

        self.calls.borrow_mut().push(DrawCall::GlyphRun { text, origin });
    }

    fn draw_rectangle(&mut self, _brush: Option<&Rc<dyn IBrush>>, _pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        self.calls.borrow_mut().push(DrawCall::Rectangle(rect));
    }

    fn draw_line(&mut self, _pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        self.calls.borrow_mut().push(DrawCall::Line(p1, p2));
    }

    fn push_transform(&mut self, matrix: Matrix) {
        self.transforms.borrow_mut().push(matrix);
    }

    fn pop_transform(&mut self) {
        self.transforms.borrow_mut().pop();
    }
}
