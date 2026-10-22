//! Bounded linear-gradient authoring. Canvas edits retain the existing paint
//! representation and never approximate a shape's native brush bounds.
use crate::{
    Color, DocumentError, Editor, GradientStop, Node, NodeId, NodeKind, Paint, Point, Rect,
};

pub const MAX_CANVAS_GRADIENT_STOPS: usize = 256;
pub const MAX_GRADIENT_COORDINATE: f64 = 1_000.0;

fn invalid(message: &str) -> DocumentError {
    DocumentError::Invalid(message.into())
}
fn valid_point(point: Point) -> bool {
    point.x.is_finite()
        && point.y.is_finite()
        && point.x.abs() <= MAX_GRADIENT_COORDINATE
        && point.y.abs() <= MAX_GRADIENT_COORDINATE
}

/// These native fill geometries have precisely the layer's local box as their
/// brush bounds. Text, arbitrary paths, polygons, Boolean results, groups and
/// network-region paints need their actual geometry bounds and remain numeric.
pub fn linear_gradient_canvas_bounds(node: &Node) -> Option<Rect> {
    (matches!(
        node.kind,
        NodeKind::Rectangle
            | NodeKind::Ellipse
            | NodeKind::Frame
            | NodeKind::Component
            | NodeKind::Instance
            | NodeKind::Section
    ) && node.vector_network.is_none()
        && node.width.is_finite()
        && node.height.is_finite()
        && node.width > 0.0
        && node.height > 0.0)
        .then(|| node.local_bounds())
}

/// An owned, bounded paint snapshot. Source stop indices and equal-offset
/// ordering are retained; the existing renderer sorts a copy stably.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradientEdit {
    pub start: Point,
    pub end: Point,
    pub stops: Vec<GradientStop>,
}

impl LinearGradientEdit {
    pub fn from_node(node: &Node, fill: usize) -> Result<Self, DocumentError> {
        linear_gradient_canvas_bounds(node).ok_or_else(|| invalid(
            "Canvas gradient editing supports rectangles, ellipses and box containers with positive finite bounds."))?;
        let fill = node
            .fills
            .get(fill)
            .ok_or_else(|| invalid("The selected fill no longer exists."))?;
        if !fill.visible || !fill.opacity.is_finite() || fill.opacity <= 0.0 {
            return Err(invalid(
                "Enable the selected fill before editing its gradient on canvas.",
            ));
        }
        let Paint::LinearGradient { start, end, stops } = &fill.paint else {
            return Err(invalid("Choose a linear-gradient fill for canvas editing."));
        };
        Self::validate_parts(*start, *end, stops)?;
        Ok(Self {
            start: *start,
            end: *end,
            stops: stops.clone(),
        })
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        Self::validate_parts(self.start, self.end, &self.stops)
    }

    fn validate_parts(
        start: Point,
        end: Point,
        stops: &[GradientStop],
    ) -> Result<(), DocumentError> {
        if !valid_point(start) || !valid_point(end) {
            return Err(invalid(
                "Gradient coordinates must be finite and between -1000 and 1000.",
            ));
        }
        if !(2..=MAX_CANVAS_GRADIENT_STOPS).contains(&stops.len()) {
            return Err(invalid("Canvas gradient editing requires 2 to 256 color stops; use numeric controls for other gradients."));
        }
        if stops
            .iter()
            .any(|stop| !stop.offset.is_finite() || !(0.0..=1.0).contains(&stop.offset))
        {
            return Err(invalid(
                "Gradient stop positions must be finite and between 0 and 1.",
            ));
        }
        Ok(())
    }

    pub fn paint(&self) -> Paint {
        Paint::LinearGradient {
            start: self.start,
            end: self.end,
            stops: self.stops.clone(),
        }
    }

    pub fn local_endpoints(&self, bounds: Rect) -> [Point; 2] {
        [self.start, self.end].map(|point| {
            Point::new(
                bounds.x + point.x * bounds.width,
                bounds.y + point.y * bounds.height,
            )
        })
    }

    /// Relative translation is clamped as a pair, retaining direction/extent.
    pub fn translated(&self, delta: Point) -> Option<Self> {
        self.validate().ok()?;
        if !delta.x.is_finite() || !delta.y.is_finite() {
            return None;
        }
        let bound = MAX_GRADIENT_COORDINATE;
        let delta = Point::new(
            delta.x.clamp(
                -bound - self.start.x.min(self.end.x),
                bound - self.start.x.max(self.end.x),
            ),
            delta.y.clamp(
                -bound - self.start.y.min(self.end.y),
                bound - self.start.y.max(self.end.y),
            ),
        );
        let mut result = self.clone();
        result.start = result.start + delta;
        result.end = result.end + delta;
        Some(result)
    }

    pub fn move_stop(&mut self, index: usize, offset: f64) -> Result<bool, DocumentError> {
        if !offset.is_finite() || !(0.0..=1.0).contains(&offset) {
            return Err(invalid(
                "Gradient stop positions must be finite and between 0 and 1.",
            ));
        }
        let stop = self
            .stops
            .get_mut(index)
            .ok_or_else(|| invalid("The selected gradient stop no longer exists."))?;
        let changed = stop.offset != offset;
        stop.offset = offset;
        Ok(changed)
    }

    /// Append without changing any existing stop index or hard-transition
    /// ordering. Interpolation follows the existing straight-RGBA paint editor.
    pub fn add_stop(&mut self, offset: f64) -> Result<usize, DocumentError> {
        self.validate()?;
        if !offset.is_finite() || !(0.0..=1.0).contains(&offset) {
            return Err(invalid(
                "Gradient stop positions must be finite and between 0 and 1.",
            ));
        }
        if self.stops.len() >= MAX_CANVAS_GRADIENT_STOPS {
            return Err(invalid(
                "This gradient has reached the 256-stop canvas editing limit.",
            ));
        }
        let color = sample_stops(&self.stops, offset);
        let index = self.stops.len();
        self.stops.push(GradientStop { offset, color });
        Ok(index)
    }

    pub fn remove_stop(&mut self, index: usize) -> Result<(), DocumentError> {
        if index >= self.stops.len() {
            return Err(invalid("The selected gradient stop no longer exists."));
        }
        if self.stops.len() <= 2 {
            return Err(invalid("Keep at least two gradient stops."));
        }
        self.stops.remove(index);
        Ok(())
    }
}

fn sample_stops(stops: &[GradientStop], offset: f64) -> Color {
    let mut ordered: Vec<_> = stops.iter().collect();
    ordered.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    let right = ordered.partition_point(|stop| stop.offset <= offset);
    if right == 0 {
        return ordered[0].color;
    }
    if right == ordered.len() {
        return ordered[right - 1].color;
    }
    let (a, b) = (ordered[right - 1], ordered[right]);
    a.color
        .lerp(b.color, (offset - a.offset) / (b.offset - a.offset))
}

/// Project displacement onto a gradient axis in the caller's coordinate space.
/// Dividing by length before taking the dot product avoids squared overflow.
pub fn gradient_offset_delta(delta: Point, axis: Point) -> Option<f64> {
    let length = axis.x.hypot(axis.y);
    if !length.is_finite() || length <= 0.0 || !delta.x.is_finite() || !delta.y.is_finite() {
        return None;
    }
    let value = (delta.x * (axis.x / length) + delta.y * (axis.y / length)) / length;
    value.is_finite().then_some(value)
}

impl Editor {
    /// Change one existing linear fill without touching another stack entry,
    /// geometry or layout. No-op values retain style/binding metadata and redo.
    pub fn set_linear_gradient(
        &mut self,
        id: NodeId,
        fill: usize,
        value: LinearGradientEdit,
    ) -> Result<bool, DocumentError> {
        value.validate()?;
        let node = self
            .document
            .nodes
            .get(&id)
            .ok_or(DocumentError::MissingNode(id))?;
        if self.document.effective_locked(id) {
            return Ok(false);
        }
        let before = LinearGradientEdit::from_node(node, fill)?;
        if before == value {
            return Ok(false);
        }
        self.update_node(id, "Edit linear gradient", |node| {
            node.fills[fill].paint = value.paint();
            node.fill_style = None;
            node.variable_bindings.remove("fill");
            node.overridden_properties.insert("fills".into());
        });
        Ok(true)
    }
}

#[cfg(test)]
#[path = "gradient_edit_tests.rs"]
mod tests;
