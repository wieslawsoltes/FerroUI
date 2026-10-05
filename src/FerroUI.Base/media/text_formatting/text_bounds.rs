use crate::media::text_formatting::TextRunBounds;
use crate::media::FlowDirection;
use crate::Rect;

/// The bounding rectangle of a range of characters.
#[derive(Clone)]
pub struct TextBounds {
    rectangle: Rect,
    flow_direction: FlowDirection,
    text_run_bounds: Vec<TextRunBounds>,
}

impl TextBounds {
    pub(crate) fn new(bounds: Rect, flow_direction: FlowDirection, run_bounds: Vec<TextRunBounds>) -> Self {
        Self { rectangle: bounds, flow_direction, text_run_bounds: run_bounds }
    }

    /// Bounds rectangle.
    pub fn rectangle(&self) -> Rect {
        self.rectangle
    }

    pub(crate) fn set_rectangle(&mut self, value: Rect) {
        self.rectangle = value;
    }

    /// Text flow direction inside the bounding rectangle.
    pub fn flow_direction(&self) -> FlowDirection {
        self.flow_direction
    }

    /// Get a list of run bounding rectangles.
    pub fn text_run_bounds(&self) -> &[TextRunBounds] {
        &self.text_run_bounds
    }

    pub(crate) fn text_run_bounds_mut(&mut self) -> &mut Vec<TextRunBounds> {
        &mut self.text_run_bounds
    }
}
