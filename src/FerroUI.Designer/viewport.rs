//! Finite, source-independent canvas navigation in logical viewport pixels.
use ferroui_base::{Matrix, Point, Rect, Size, Thickness, Vector};

/// Wheel behavior for the design canvas. Fractional native wheel deltas are preserved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CanvasWheelMode {
    /// Wheel pans; Ctrl/Cmd+wheel zooms about the pointer (Figma-style navigation).
    #[default]
    Pan,
    /// Wheel zooms; Ctrl/Cmd+wheel pans (Blend-style navigation).
    Zoom,
}

/// Maps authored artboard coordinates to a clipped native canvas without remeasuring
/// application content: `viewport = artboard * zoom + offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasViewport {
    zoom: f64,
    offset: Vector,
}

impl Default for CanvasViewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: Vector::ZERO,
        }
    }
}

impl CanvasViewport {
    pub const MIN_ZOOM: f64 = 0.01;
    pub const MAX_ZOOM: f64 = 16.0;
    // Large enough for authoring, while keeping native bounds and inverse transforms useful.
    const MAX_OFFSET: f64 = 1.0e12;

    pub fn new(zoom: f64, offset: Vector) -> Result<Self, String> {
        if !zoom.is_finite()
            || zoom <= 0.0
            || !finite_point(Point::new(offset.x, offset.y))
            || offset.x.abs() > Self::MAX_OFFSET
            || offset.y.abs() > Self::MAX_OFFSET
        {
            return Err(
                "Canvas zoom and offset must be finite and within the supported range".into(),
            );
        }
        Ok(Self {
            zoom: zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM),
            offset,
        })
    }

    pub fn zoom(self) -> f64 {
        self.zoom
    }
    pub fn offset(self) -> Vector {
        self.offset
    }

    pub fn transform(self) -> Matrix {
        Matrix::new(self.zoom, 0.0, 0.0, self.zoom, self.offset.x, self.offset.y)
    }

    pub fn artboard_to_viewport(self, point: Point) -> Point {
        Point::new(
            point.x * self.zoom + self.offset.x,
            point.y * self.zoom + self.offset.y,
        )
    }

    pub fn viewport_to_artboard(self, point: Point) -> Point {
        Point::new(
            (point.x - self.offset.x) / self.zoom,
            (point.y - self.offset.y) / self.zoom,
        )
    }

    /// Keep the authored point under `anchor` stationary, including when a limit is reached.
    pub fn zoom_at(self, zoom: f64, anchor: Point) -> Result<Self, String> {
        if !zoom.is_finite() || zoom <= 0.0 || !finite_point(anchor) {
            return Err("Canvas zoom and anchor must be finite and zoom must be positive".into());
        }
        let zoom = zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        let factor = zoom / self.zoom;
        Self::new(
            zoom,
            Vector::new(
                anchor.x + (self.offset.x - anchor.x) * factor,
                anchor.y + (self.offset.y - anchor.y) * factor,
            ),
        )
    }

    /// Translation is in logical viewport pixels, independently of the current zoom.
    pub fn pan(self, delta: Vector) -> Result<Self, String> {
        Self::new(self.zoom, self.offset + delta)
    }

    /// Center the bounds using the largest supported zoom that fits inside the padding.
    pub fn fit(bounds: Rect, viewport: Size, padding: Thickness) -> Result<Self, String> {
        let available = Size::new(
            viewport.width - padding.left - padding.right,
            viewport.height - padding.top - padding.bottom,
        );
        if !finite_point(Point::new(bounds.x, bounds.y))
            || !finite_size(bounds.size())
            || !finite_size(viewport)
            || !finite_size(available)
            || available.width <= 0.0
            || available.height <= 0.0
            || bounds.width < 0.0
            || bounds.height < 0.0
            || [padding.left, padding.top, padding.right, padding.bottom]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(
                "Arrange the canvas and provide finite visible bounds before fitting".into(),
            );
        }
        let zoom = (available.width / bounds.width.max(1.0))
            .min(available.height / bounds.height.max(1.0));
        if zoom < Self::MIN_ZOOM {
            return Err("The canvas is too small to fit these bounds at the minimum zoom".into());
        }
        let zoom = zoom.min(Self::MAX_ZOOM);
        Self::new(
            zoom,
            Vector::new(
                padding.left + available.width / 2.0 - (bounds.x + bounds.width / 2.0) * zoom,
                padding.top + available.height / 2.0 - (bounds.y + bounds.height / 2.0) * zoom,
            ),
        )
    }
}

pub(crate) fn finite_point(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}
pub(crate) fn finite_size(size: Size) -> bool {
    size.width.is_finite() && size.height.is_finite()
}
