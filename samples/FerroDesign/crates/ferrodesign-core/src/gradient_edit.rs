//! Bounded gradient authoring. Canvas edits retain the existing paint
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

/// Geometric parameters of a native gradient. Radial radii are equal relative
/// scalars, matching `ImmutableRadialGradientBrush::with_radius`; angular
/// angles are degrees clockwise from the upward ray in actual local pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanvasGradientGeometry {
    Linear { start: Point, end: Point },
    Radial { center: Point, radius: f64 },
    Angular { center: Point, angle: f64 },
}

/// Canvas editing keeps the native paint representation and authored stop
/// order. No intermediate linear gradient is written for another paint kind.
#[derive(Clone, Debug, PartialEq)]
pub struct CanvasGradientEdit {
    pub geometry: CanvasGradientGeometry,
    pub stops: Vec<GradientStop>,
}

impl CanvasGradientEdit {
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
        let (geometry, stops) = match &fill.paint {
            Paint::LinearGradient { start, end, stops } => (
                CanvasGradientGeometry::Linear {
                    start: *start,
                    end: *end,
                },
                stops,
            ),
            Paint::RadialGradient {
                center,
                radius,
                stops,
            } => (
                CanvasGradientGeometry::Radial {
                    center: *center,
                    radius: *radius,
                },
                stops,
            ),
            Paint::AngularGradient {
                center,
                angle,
                stops,
            } => (
                CanvasGradientGeometry::Angular {
                    center: *center,
                    angle: *angle,
                },
                stops,
            ),
            _ => {
                return Err(invalid(
                    "Choose a linear, radial or angular gradient fill for canvas editing.",
                ))
            }
        };
        Self::validate_parts(geometry, stops)?;
        Ok(Self {
            geometry,
            stops: stops.clone(),
        })
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        Self::validate_parts(self.geometry, &self.stops)
    }

    fn validate_parts(
        geometry: CanvasGradientGeometry,
        stops: &[GradientStop],
    ) -> Result<(), DocumentError> {
        let (start, end) = match geometry {
            CanvasGradientGeometry::Linear { start, end } => (start, end),
            CanvasGradientGeometry::Radial { center, radius } => {
                if !radius.is_finite() || !(0.0..=MAX_GRADIENT_COORDINATE).contains(&radius) {
                    return Err(invalid(
                        "Gradient radius must be finite and between 0 and 1000.",
                    ));
                }
                (center, center)
            }
            CanvasGradientGeometry::Angular { center, angle } => {
                if !angle.is_finite() || angle.abs() > 360_000.0 {
                    return Err(invalid("Canvas gradient angle must be finite and between -360000 and 360000 degrees."));
                }
                (center, center)
            }
        };
        LinearGradientEdit::validate_parts(start, end, stops)
    }

    pub fn kind(&self) -> &'static str {
        match self.geometry {
            CanvasGradientGeometry::Linear { .. } => "linear",
            CanvasGradientGeometry::Radial { .. } => "radial",
            CanvasGradientGeometry::Angular { .. } => "angular",
        }
    }

    pub fn paint(&self) -> Paint {
        let stops = self.stops.clone();
        match self.geometry {
            CanvasGradientGeometry::Linear { start, end } => {
                Paint::LinearGradient { start, end, stops }
            }
            CanvasGradientGeometry::Radial { center, radius } => Paint::RadialGradient {
                center,
                radius,
                stops,
            },
            CanvasGradientGeometry::Angular { center, angle } => Paint::AngularGradient {
                center,
                angle,
                stops,
            },
        }
    }

    pub fn is_angular(&self) -> bool {
        matches!(self.geometry, CanvasGradientGeometry::Angular { .. })
    }

    /// Radius of the angular editing ring only. Conic artwork has no radius.
    /// A local-pixel circle preserves the native conic angle on nonsquare boxes;
    /// transforming a relative-coordinate circle would change those angles.
    pub fn angular_ring_radius(bounds: Rect) -> f64 {
        bounds.width.min(bounds.height) * 0.5
    }

    pub fn local_endpoints(&self, bounds: Rect) -> [Point; 2] {
        let local = |point: Point| {
            Point::new(
                bounds.x + point.x * bounds.width,
                bounds.y + point.y * bounds.height,
            )
        };
        match self.geometry {
            CanvasGradientGeometry::Linear { start, end } => [local(start), local(end)],
            CanvasGradientGeometry::Radial { center, radius } => {
                let center = local(center);
                [center, center + Point::new(radius * bounds.width, 0.0)]
            }
            CanvasGradientGeometry::Angular { center, angle } => {
                let center = local(center);
                let radians = angle.rem_euclid(360.0).to_radians();
                [
                    center,
                    center
                        + Point::new(radians.sin(), -radians.cos())
                            * Self::angular_ring_radius(bounds),
                ]
            }
        }
    }

    pub fn local_stop(&self, bounds: Rect, offset: f64) -> Point {
        let axis = self.local_endpoints(bounds);
        if let CanvasGradientGeometry::Angular { angle, .. } = self.geometry {
            let radians = (angle.rem_euclid(360.0) + offset * 360.0).to_radians();
            axis[0] + Point::new(radians.sin(), -radians.cos()) * Self::angular_ring_radius(bounds)
        } else {
            axis[0] + (axis[1] - axis[0]) * offset
        }
    }

    /// Native radial boundary or angular editing ring, in local pixels. This
    /// method describes an adorner; it never substitutes for artwork geometry.
    pub fn local_ring(&self, bounds: Rect, fraction: f64) -> Option<Point> {
        let center = self.local_endpoints(bounds)[0];
        let radians = fraction * std::f64::consts::TAU;
        match self.geometry {
            CanvasGradientGeometry::Linear { .. } => None,
            CanvasGradientGeometry::Radial { radius, .. } => Some(
                center
                    + Point::new(
                        radians.cos() * radius * bounds.width,
                        radians.sin() * radius * bounds.height,
                    ),
            ),
            CanvasGradientGeometry::Angular { .. } => Some(
                center
                    + Point::new(radians.sin(), -radians.cos()) * Self::angular_ring_radius(bounds),
            ),
        }
    }

    pub fn translated(&self, delta: Point) -> Option<Self> {
        self.validate().ok()?;
        if !delta.x.is_finite() || !delta.y.is_finite() {
            return None;
        }
        let mut value = self.clone();
        match &mut value.geometry {
            CanvasGradientGeometry::Linear { start, end } => {
                let moved = LinearGradientEdit {
                    start: *start,
                    end: *end,
                    stops: self.stops.clone(),
                }
                .translated(delta)?;
                *start = moved.start;
                *end = moved.end;
            }
            CanvasGradientGeometry::Radial { center, .. }
            | CanvasGradientGeometry::Angular { center, .. } => {
                center.x =
                    (center.x + delta.x).clamp(-MAX_GRADIENT_COORDINATE, MAX_GRADIENT_COORDINATE);
                center.y =
                    (center.y + delta.y).clamp(-MAX_GRADIENT_COORDINATE, MAX_GRADIENT_COORDINATE);
            }
        }
        Some(value)
    }

    /// Displace a geometric handle from its logical position, independently of
    /// any visual separation applied to coincident or nearby screen markers.
    pub fn moved_endpoint(&self, end: bool, local_delta: Point, bounds: Rect) -> Option<Self> {
        self.validate().ok()?;
        if !local_delta.x.is_finite()
            || !local_delta.y.is_finite()
            || !bounds.width.is_finite()
            || !bounds.height.is_finite()
            || bounds.width <= 0.0
            || bounds.height <= 0.0
        {
            return None;
        }
        let delta = Point::new(local_delta.x / bounds.width, local_delta.y / bounds.height);
        let mut value = self.clone();
        match &mut value.geometry {
            CanvasGradientGeometry::Linear { start, end: finish } => {
                let point = if end { finish } else { start };
                point.x =
                    (point.x + delta.x).clamp(-MAX_GRADIENT_COORDINATE, MAX_GRADIENT_COORDINATE);
                point.y =
                    (point.y + delta.y).clamp(-MAX_GRADIENT_COORDINATE, MAX_GRADIENT_COORDINATE);
            }
            CanvasGradientGeometry::Radial { radius, .. } if end => {
                // Radius is a relative scalar on both axes, exactly as in the
                // native brush. Layer transforms are already inverted by input.
                *radius = (*radius + delta.x)
                    .hypot(delta.y)
                    .clamp(0.0, MAX_GRADIENT_COORDINATE);
            }
            CanvasGradientGeometry::Angular { angle, .. } if end => {
                let axis = self.local_endpoints(bounds);
                let before = axis[1] - axis[0];
                let after = before + local_delta;
                if let Some(change) = gradient_angle_delta(before, after) {
                    *angle = (*angle + change.to_degrees()).clamp(-360_000.0, 360_000.0);
                }
            }
            _ => return self.translated(delta),
        }
        Some(value)
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

/// Signed shortest angle from one local direction to another. Normalize
/// independently before dot/cross operations to avoid squared overflow.
/// Screen reflection never changes this result because input is inverted first.
pub fn gradient_angle_delta(before: Point, after: Point) -> Option<f64> {
    let a = before.x.hypot(before.y);
    let b = after.x.hypot(after.y);
    if !a.is_finite() || !b.is_finite() || a <= 1e-12 || b <= 1e-12 {
        return None;
    }
    let before = before * (1.0 / a);
    let after = after * (1.0 / b);
    let angle =
        (before.x * after.y - before.y * after.x).atan2(before.x * after.x + before.y * after.y);
    angle.is_finite().then_some(angle)
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
    /// Edit one native gradient in place. Paint-kind conversion belongs to the
    /// type picker; a stale canvas session must never silently change the kind.
    pub fn set_canvas_gradient(
        &mut self,
        id: NodeId,
        fill: usize,
        value: CanvasGradientEdit,
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
        let before = CanvasGradientEdit::from_node(node, fill)?;
        if before.kind() != value.kind() {
            return Err(invalid(
                "The gradient type changed while the canvas editor was active.",
            ));
        }
        if before == value {
            return Ok(false);
        }
        self.update_node(id, "Edit gradient", |node| {
            node.fills[fill].paint = value.paint();
            node.fill_style = None;
            node.variable_bindings.remove("fill");
            node.overridden_properties.insert("fills".into());
        });
        Ok(true)
    }

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
