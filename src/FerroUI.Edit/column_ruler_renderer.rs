use crate::background_renderer_connections::BackgroundRendererConnections;
use crate::{
    BackgroundGeometryBuilder, IBackgroundRenderer, ITextViewConnect, KnownLayer, TextArea,
    VisualLineError,
};
use ferroui_base::input::IScrollable;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{Color, DrawingContext, IPen};
use ferroui_base::{Matrix, Point};
use ferroui_controls::TopLevel;
use std::cell::RefCell;
use std::rc::Rc;

/// Draws vertical column guides through the native background layer.
///
/// Columns are measured in wide-space advances from the content origin. Add an
/// `Rc` of this renderer to [`TextArea::background_renderers`] to connect it;
/// changing its state invalidates every connected view without reshaping text.
pub struct ColumnRulerRenderer {
    columns: RefCell<Vec<i32>>,
    pen: RefCell<Option<Rc<dyn IPen>>>,
    connections: BackgroundRendererConnections,
}

impl Default for ColumnRulerRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ColumnRulerRenderer {
    pub const DEFAULT_FOREGROUND: Color = Color::from_rgb(211, 211, 211);

    pub fn new() -> Self {
        Self {
            columns: RefCell::new(Vec::new()),
            pen: RefCell::new(Some(Rc::new(ImmutablePen::from_uint32(0xffd3d3d3, 1.0)))),
            connections: BackgroundRendererConnections::default(),
        }
    }

    /// Returns the ordered, owned column snapshot, including duplicate columns.
    pub fn columns(&self) -> Vec<i32> {
        self.columns.borrow().clone()
    }

    pub fn pen(&self) -> Option<Rc<dyn IPen>> {
        self.pen.borrow().clone()
    }

    /// Replaces both the columns and stroke in one invalidation. A missing pen
    /// hides the ruler while retaining the column settings.
    pub fn set_ruler(&self, columns: impl IntoIterator<Item = i32>, pen: Option<Rc<dyn IPen>>) {
        self.columns.replace(columns.into_iter().collect());
        self.pen.replace(pen);
        self.connections.invalidate(KnownLayer::Background);
    }

    pub fn set_columns(&self, columns: impl IntoIterator<Item = i32>) {
        let columns: Vec<_> = columns.into_iter().collect();
        if *self.columns.borrow() == columns {
            return;
        }
        self.columns.replace(columns);
        self.connections.invalidate(KnownLayer::Background);
    }

    pub fn set_pen(&self, pen: Option<Rc<dyn IPen>>) {
        self.pen.replace(pen);
        self.connections.invalidate(KnownLayer::Background);
    }
}

impl ITextViewConnect for ColumnRulerRenderer {
    fn add_to_text_view(&self, view: &TextArea) {
        self.connections.add(view);
    }

    fn remove_from_text_view(&self, view: &TextArea) {
        self.connections.remove(view);
    }
}

impl IBackgroundRenderer for ColumnRulerRenderer {
    fn layer(&self) -> KnownLayer {
        KnownLayer::Background
    }

    fn draw(&self, view: &TextArea, context: &mut DrawingContext) -> Result<(), VisualLineError> {
        let Some(pen) = self.pen() else {
            return Ok(());
        };
        let columns = self.columns();
        let viewport = view.content_viewport();
        let pixels = BackgroundGeometryBuilder::pixel_size(view);
        let offset = view.offset();
        let height = view.extent().height.max(viewport.height);
        let space = view.wide_space_width();
        let device_transform = TopLevel::get_top_level(Some(view)).and_then(|root| {
            view.transform_to_visual(&root).map(|transform| {
                transform * Matrix::create_scale(root.render_scaling(), root.render_scaling())
            })
        });
        for column in columns {
            let marker = pixel_align(space * f64::from(column), pixels.width);
            let x = viewport.x + marker - offset.x;
            let start = Point::new(x, viewport.y);
            let end = Point::new(x, viewport.y + height);
            if !start.x.is_finite() || !start.y.is_finite() || !end.y.is_finite() {
                return Err(VisualLineError::Extension(
                    "Column ruler coordinates are not finite".into(),
                ));
            }
            context.draw_line(
                &pen,
                snap_to_device_pixels(start, device_transform),
                snap_to_device_pixels(end, device_transform),
            );
        }
        Ok(())
    }
}

/// Upstream midpoint rounding is away from zero, including negative columns.
pub(crate) fn pixel_align(value: f64, pixel_size: f64) -> f64 {
    pixel_size * ((value / pixel_size + 0.5).round() - 0.5)
}

/// The second snap includes the visual transform and root scaling. Preserve the
/// upstream truncation toward zero; a detached or singular visual falls back
/// to the already column-aligned local point.
pub(crate) fn snap_to_device_pixels(point: Point, transform: Option<Matrix>) -> Point {
    let Some(transform) = transform else {
        return point;
    };
    let Some(inverse) = transform.try_invert() else {
        return point;
    };
    let device = point.transform(transform);
    if !device.x.is_finite() || !device.y.is_finite() {
        return point;
    }
    let snapped = Point::new(device.x.trunc() + 0.5, device.y.trunc() + 0.5).transform(inverse);
    if snapped.x.is_finite() && snapped.y.is_finite() {
        snapped
    } else {
        point
    }
}
