/// Aggregate geometry of wrapped text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WrapMetrics {
    pub line_count: usize,
    pub height: f64,
    pub max_line_width: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositionedLine {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RectObstacle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl RectObstacle {
    pub fn right(self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(self) -> f64 {
        self.y + self.height
    }
}

/// A circle whose `x` and `y` identify the center.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CircleObstacle {
    pub x: f64,
    pub y: f64,
    pub radius: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Interval {
    pub left: f64,
    pub right: f64,
}

impl Interval {
    pub fn width(self) -> f64 {
        self.right - self.left
    }
}
