//! Native presentation for model-owned layout adorners.
//!
//! Geometry is transformed into artboard coordinates before drawing. Stroke widths,
//! labels and handles therefore stay legible under panel transforms and canvas zoom.
//! This layer owns no document, runtime layout mutation or authoring transaction.
use crate::LayoutHelperSnapshot;
use ferroui_base::media::{
    BoxShadows, DashStyle, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Pen,
    SolidColorBrush, StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{Point, Rect};
use ferroui_designer_tools::{
    Affine, Axis, LayoutGuide, LayoutGuideGeometry, LayoutGuideRole, SnapGuide,
};
use std::rc::Rc;

#[path = "layout_helper_ghost.rs"]
mod ghost;
#[path = "layout_helper_metrics.rs"]
mod metrics;
pub use ghost::DragVisualGhost;

/// Optional view state; neither hovering nor rendering changes authored layout.
#[derive(Clone, Copy, Debug, Default)]
pub struct LayoutHelperRenderState {
    /// Visible artboard rectangle. Labels are kept inside it; geometry is clipped.
    pub viewport: Option<Rect>,
    pub hovered_handle: Option<usize>,
    pub active_handle: Option<usize>,
}

/// Draw a model scene using the standard designer adorner palette.
pub fn render_layout_helpers(
    context: &mut DrawingContext,
    snapshot: &LayoutHelperSnapshot,
    zoom: f64,
) {
    render_layout_helpers_with_state(context, snapshot, zoom, &LayoutHelperRenderState::default());
}

/// Draw a scene with native hover/drag feedback and optional viewport clipping.
pub fn render_layout_helpers_with_state(
    context: &mut DrawingContext,
    snapshot: &LayoutHelperSnapshot,
    zoom: f64,
    state: &LayoutHelperRenderState,
) {
    let Some(scale) = inverse_zoom(zoom) else {
        return;
    };
    if snapshot.to_artboard.validate().is_err() {
        return;
    }
    let viewport = state.viewport.filter(|rect| valid_rect(*rect));
    let clip = viewport.map(|rect| context.push_clip(rect));
    let mut labels = Vec::new();
    let placement = snapshot.scene.placement.as_ref();
    let provider_guide = |guide: &&LayoutGuide| {
        placement.is_none()
            || !matches!(
                guide.role,
                LayoutGuideRole::Placement | LayoutGuideRole::Rejected
            )
    };
    // Paint fills before strokes and labels so custom providers cannot accidentally
    // cover a dimension or insertion marker with a later translucent region.
    for guide in snapshot.scene.primitives.iter().filter(provider_guide) {
        if matches!(
            guide.geometry,
            LayoutGuideGeometry::Rectangle { filled: true, .. }
        ) {
            draw_guide(
                context,
                guide,
                snapshot.to_artboard,
                scale,
                viewport,
                &mut labels,
            );
        }
    }
    // The accepted transaction may refine or reject a provider's initial candidate.
    // Its final placement is authoritative; never paint stale provider drop geometry.
    if let Some(placement) = placement {
        let role = if placement.rejection.is_some() {
            LayoutGuideRole::Rejected
        } else {
            LayoutGuideRole::Placement
        };
        let bounds = placement.bounds;
        let geometry = if bounds.width <= 0.001 || bounds.height <= 0.001 {
            LayoutGuideGeometry::Line {
                start: ferroui_designer_tools::Point::new(bounds.x, bounds.y),
                end: ferroui_designer_tools::Point::new(bounds.right(), bounds.bottom()),
            }
        } else {
            LayoutGuideGeometry::Rectangle {
                bounds,
                filled: true,
            }
        };
        draw_guide(
            context,
            &LayoutGuide { role, geometry },
            snapshot.to_artboard,
            scale,
            viewport,
            &mut labels,
        );
    }
    for guide in snapshot.scene.primitives.iter().filter(provider_guide) {
        if !matches!(
            guide.geometry,
            LayoutGuideGeometry::Rectangle { filled: true, .. }
        ) {
            draw_guide(
                context,
                guide,
                snapshot.to_artboard,
                scale,
                viewport,
                &mut labels,
            );
        }
    }
    if let Some(bounds) = snapshot.scene.primitives.iter().find_map(|guide| {
        if guide.role == LayoutGuideRole::Container {
            if let LayoutGuideGeometry::Rectangle { bounds, .. } = guide.geometry {
                return transformed_rect(bounds, snapshot.to_artboard).map(quad_bounds);
            }
        }
        None
    }) {
        badge(
            context,
            &snapshot.scene.title,
            Point::new(bounds.x, bounds.y - 11.0 * scale),
            LayoutGuideRole::Container,
            scale,
            viewport,
            false,
            &mut labels,
        );
    }
    if let Some(placement) = placement {
        if let Some(anchor) = transform_point(placement.bounds.center(), snapshot.to_artboard) {
            let rejected = placement.rejection.as_deref();
            let role = if rejected.is_some() {
                LayoutGuideRole::Rejected
            } else {
                LayoutGuideRole::Placement
            };
            badge(
                context,
                rejected.unwrap_or(&placement.description),
                Point::new(anchor.x, anchor.y - 16.0 * scale),
                role,
                scale,
                viewport,
                true,
                &mut Vec::new(),
            );
        }
    }
    for (index, handle) in snapshot.scene.handles.iter().enumerate() {
        let Some(point) = transform_point(handle.position, snapshot.to_artboard) else {
            continue;
        };
        if viewport.is_some_and(|view| !view.inflate(16.0 * scale).contains(point)) {
            continue;
        }
        let disabled = handle.disabled_reason.is_some();
        let active = state.active_handle == Some(index);
        let hovered = state.hovered_handle == Some(index);
        let (width, height) = match handle.axis {
            Axis::Horizontal => (5.0, 13.0),
            Axis::Vertical => (13.0, 5.0),
        };
        let color = if disabled { 0xff777d88 } else { 0xffe7a1f5 };
        let outline = pen(color, 1.0 * scale, false);
        let fill = brush(if active || hovered { color } else { 0xff25262b });
        context.draw_rectangle(
            Some(&fill),
            Some(&outline),
            Rect::new(
                point.x - width * scale * 0.5,
                point.y - height * scale * 0.5,
                width * scale,
                height * scale,
            ),
            2.0 * scale,
            2.0 * scale,
            &BoxShadows::default(),
        );
        if active || hovered {
            let label = handle.disabled_reason.as_deref().map_or_else(
                || format!("{}  {}", handle.label, metrics::number(handle.value)),
                str::to_owned,
            );
            badge(
                context,
                &label,
                Point::new(point.x, point.y - 18.0 * scale),
                if disabled {
                    LayoutGuideRole::Rejected
                } else {
                    LayoutGuideRole::Spacing
                },
                scale,
                viewport,
                true,
                &mut Vec::new(),
            );
        }
    }
    if let Some(clip) = clip {
        context.pop(clip);
    }
}

/// Find the nearest editable grip in screen space. Disabled or singular scenes
/// remain visible but cannot intercept a pointer gesture.
pub fn hit_test_layout_handle(
    snapshot: &LayoutHelperSnapshot,
    point: Point,
    zoom: f64,
) -> Option<usize> {
    let scale = inverse_zoom(zoom)?;
    snapshot.to_artboard.inverse().ok()?;
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    let radius = 8.0 * scale;
    snapshot
        .scene
        .handles
        .iter()
        .enumerate()
        .filter(|(_, handle)| handle.disabled_reason.is_none())
        .filter_map(|(index, handle)| {
            let anchor = transform_point(handle.position, snapshot.to_artboard)?;
            let dx = point.x - anchor.x;
            let dy = point.y - anchor.y;
            (dx.abs() <= radius && dy.abs() <= radius).then_some((index, dx * dx + dy * dy))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(index, _)| index)
}

/// Paint bounded alignment guides, neighboring gaps and measured selection size.
/// Inputs are artboard-space bounds. This provides visual feedback only; snapping
/// and accepted layout changes remain owned by the gesture/session engines.
pub fn render_manipulation_metrics(
    context: &mut DrawingContext,
    moving: Rect,
    peers: &[Rect],
    guides: &[SnapGuide],
    zoom: f64,
) {
    let Some(scale) = inverse_zoom(zoom) else {
        return;
    };
    if !valid_rect(moving) {
        return;
    }
    let mut labels = Vec::new();
    for guide in metrics::manipulation_guides(moving, peers, guides, scale) {
        draw_guide(context, &guide, Affine::IDENTITY, scale, None, &mut labels);
    }
    badge(
        context,
        &format!(
            "{} × {}",
            metrics::number(moving.width),
            metrics::number(moving.height)
        ),
        Point::new(moving.center().x, moving.bottom() + 15.0 * scale),
        LayoutGuideRole::Measurement,
        scale,
        None,
        true,
        &mut labels,
    );
}

fn draw_guide(
    context: &mut DrawingContext,
    guide: &LayoutGuide,
    transform: Affine,
    scale: f64,
    viewport: Option<Rect>,
    labels: &mut Vec<Rect>,
) {
    let color = role_color(guide.role);
    let strong = matches!(
        guide.role,
        LayoutGuideRole::Placement | LayoutGuideRole::Rejected
    );
    let line = pen(
        color,
        if strong { 2.0 * scale } else { scale },
        matches!(
            guide.role,
            LayoutGuideRole::Track | LayoutGuideRole::Content | LayoutGuideRole::Relation
        ),
    );
    match &guide.geometry {
        LayoutGuideGeometry::Line { start, end } => {
            let (Some(start), Some(end)) = (
                transform_point(*start, transform),
                transform_point(*end, transform),
            ) else {
                return;
            };
            if viewport.is_some_and(|view| {
                !view
                    .inflate(8.0 * scale)
                    .intersects(line_bounds(start, end))
            }) {
                return;
            }
            context.draw_line(&line, start, end);
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let length = dx.hypot(dy);
            if length > 0.001 {
                let ux = dx / length;
                let uy = dy / length;
                if guide.role == LayoutGuideRole::Flow {
                    for sign in [-1.0, 1.0] {
                        context.draw_line(
                            &line,
                            end,
                            Point::new(
                                end.x - (ux * 5.0 + uy * sign * 3.0) * scale,
                                end.y - (uy * 5.0 - ux * sign * 3.0) * scale,
                            ),
                        );
                    }
                } else if matches!(
                    guide.role,
                    LayoutGuideRole::Spacing
                        | LayoutGuideRole::Measurement
                        | LayoutGuideRole::Padding
                        | LayoutGuideRole::Placement
                        | LayoutGuideRole::Rejected
                ) {
                    let cap = if strong { 4.0 } else { 3.0 } * scale;
                    for point in [start, end] {
                        context.draw_line(
                            &line,
                            Point::new(point.x - uy * cap, point.y + ux * cap),
                            Point::new(point.x + uy * cap, point.y - ux * cap),
                        );
                    }
                }
            }
        }
        LayoutGuideGeometry::Rectangle { bounds, filled } => {
            let Some(points) = transformed_rect(*bounds, transform) else {
                return;
            };
            if viewport.is_some_and(|view| !view.intersects(quad_bounds(points))) {
                return;
            }
            let geometry = StreamGeometry::new();
            {
                let mut path = geometry.open();
                path.begin_figure(points[0], *filled);
                for point in &points[1..] {
                    path.line_to(*point, true);
                }
                path.end_figure(true);
            }
            let fill = filled.then(|| {
                brush((color & 0x00ff_ffff) | if strong { 0x2600_0000 } else { 0x1600_0000 })
            });
            context.draw_geometry(fill.as_ref(), Some(&line), &geometry.upcast());
        }
        LayoutGuideGeometry::Label { position, text } => {
            let Some(position) = transform_point(*position, transform) else {
                return;
            };
            // Offscreen providers must not force expensive text shaping or pull every
            // label to a viewport edge. A nearby label can be nudged fully onscreen.
            if viewport.is_some_and(|view| !view.inflate(24.0 * scale).contains(position)) {
                return;
            }
            badge(
                context, text, position, guide.role, scale, viewport, true, labels,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn badge(
    context: &mut DrawingContext,
    value: &str,
    anchor: Point,
    role: LayoutGuideRole,
    scale: f64,
    viewport: Option<Rect>,
    centered: bool,
    occupied: &mut Vec<Rect>,
) {
    if value.is_empty() {
        return;
    }
    let value = value.chars().take(96).collect::<String>();
    let text = FormattedText::new(
        &value,
        CultureInfo::current_culture(),
        FlowDirection::LeftToRight,
        Typeface::default(),
        10.5 * scale,
        Some(brush(role_color(role))),
    );
    let width = text.width().ceil() + 10.0 * scale;
    let height = text.height().ceil() + 5.0 * scale;
    let mut bounds = Rect::new(
        anchor.x - if centered { width * 0.5 } else { 0.0 },
        anchor.y - height * 0.5,
        width,
        height,
    );
    if let Some(view) = viewport {
        bounds.x = bounds
            .x
            .max(view.x + scale)
            .min((view.right() - width - scale).max(view.x + scale));
        bounds.y = bounds
            .y
            .max(view.y + scale)
            .min((view.bottom() - height - scale).max(view.y + scale));
    }
    if occupied.iter().any(|other| other.intersects(bounds)) {
        return;
    }
    occupied.push(bounds.inflate(scale));
    let fill = brush(0xf524252b);
    let outline = pen((role_color(role) & 0x00ff_ffff) | 0x7000_0000, scale, false);
    context.draw_rectangle(
        Some(&fill),
        Some(&outline),
        bounds,
        3.0 * scale,
        3.0 * scale,
        &BoxShadows::default(),
    );
    context.draw_text(
        &text,
        Point::new(bounds.x + 5.0 * scale, bounds.y + 2.5 * scale),
    );
}

fn transform_point(point: ferroui_designer_tools::Point, transform: Affine) -> Option<Point> {
    transform
        .transform(point)
        .ok()
        .map(|point| Point::new(point.x, point.y))
}
fn transformed_rect(bounds: ferroui_designer_tools::Rect, transform: Affine) -> Option<[Point; 4]> {
    bounds.validate().ok()?;
    Some([
        transform_point(
            ferroui_designer_tools::Point::new(bounds.x, bounds.y),
            transform,
        )?,
        transform_point(
            ferroui_designer_tools::Point::new(bounds.right(), bounds.y),
            transform,
        )?,
        transform_point(
            ferroui_designer_tools::Point::new(bounds.right(), bounds.bottom()),
            transform,
        )?,
        transform_point(
            ferroui_designer_tools::Point::new(bounds.x, bounds.bottom()),
            transform,
        )?,
    ])
}
fn quad_bounds(points: [Point; 4]) -> Rect {
    let x = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let y = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let right = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    Rect::new(x, y, right - x, bottom - y)
}
fn line_bounds(a: Point, b: Point) -> Rect {
    Rect::new(
        a.x.min(b.x),
        a.y.min(b.y),
        (a.x - b.x).abs(),
        (a.y - b.y).abs(),
    )
}
fn valid_rect(rect: Rect) -> bool {
    [
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        rect.right(),
        rect.bottom(),
    ]
    .iter()
    .all(|value| value.is_finite())
        && rect.width >= 0.0
        && rect.height >= 0.0
}
fn inverse_zoom(zoom: f64) -> Option<f64> {
    (zoom.is_finite() && zoom > 0.0)
        .then(|| 1.0 / zoom)
        .filter(|scale| scale.is_finite())
}
fn role_color(role: LayoutGuideRole) -> u32 {
    match role {
        LayoutGuideRole::Container => 0xff79a8ff,
        LayoutGuideRole::Content => 0xff82c9b8,
        LayoutGuideRole::Track => 0xff8cb4f4,
        LayoutGuideRole::Flow => 0xffb6a2f6,
        LayoutGuideRole::Spacing | LayoutGuideRole::Measurement => 0xffe7a1f5,
        LayoutGuideRole::Padding => 0xffe6bd7c,
        LayoutGuideRole::Relation => 0xff76d5ce,
        LayoutGuideRole::Placement => 0xff79a8ff,
        LayoutGuideRole::Rejected => 0xfff48b95,
    }
}
fn brush(color: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::from_uint32(color).into()
}
fn pen(color: u32, width: f64, dashed: bool) -> Rc<dyn IPen> {
    let pen = Pen::from_uint32(color, width);
    if dashed {
        pen.set_dash_style(Some(DashStyle::dash()));
    }
    pen.into()
}

#[cfg(test)]
#[path = "layout_helper_renderer_tests.rs"]
mod tests;

#[cfg(all(test, feature = "render-smoke"))]
#[path = "layout_helper_render_tests.rs"]
mod render_tests;
