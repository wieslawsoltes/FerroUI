use crate::media::{GradientSpreadMethod, IBrush, IGradientStop};
use std::rc::Rc;

/// A brush that draws with a gradient.
pub trait IGradientBrush: IBrush {
    /// The brush's gradient stops.
    fn gradient_stops(&self) -> Vec<Rc<dyn IGradientStop>>;

    /// The brush's spread method that defines how to draw a gradient that
    /// doesn't fill the bounds of the destination control.
    fn spread_method(&self) -> GradientSpreadMethod;
}
