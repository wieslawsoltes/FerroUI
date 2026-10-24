//! Rounded editor background outlines, ported from AvaloniaEdit.
//! Copyright (c) 2014 AlphaSierraPapa for the SharpDevelop Team; MIT, see NOTICE.md.

use crate::background_geometry_rects::{
    BackgroundGeometryLine, BackgroundGeometryViewport, get_rects_for_segment,
    get_rects_for_selection,
};
use crate::{TextArea, VisualLineError};
use ferroui_base::media::{FillRule, StreamGeometry, SweepDirection};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{Point, Rect, Ref, Size};
use ferroui_controls::TopLevel;
use ferroui_edit_core::document::ISegment;
use ferroui_edit_core::editing::SelectionSegment;
use std::collections::VecDeque;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundGeometryError {
    InvalidRectangle,
    InvalidCornerRadius,
    InvalidBorderThickness,
    InvalidPixelSize,
    InvalidViewport,
    InvalidVisualRange,
    CoordinateOverflow,
}
impl fmt::Display for BackgroundGeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidRectangle => "A background rectangle must have finite coordinates and nonnegative dimensions",
            Self::InvalidCornerRadius => "The background corner radius must be finite and nonnegative",
            Self::InvalidBorderThickness => "The background border thickness must be finite and nonnegative",
            Self::InvalidPixelSize => "Pixel dimensions must be finite and positive",
            Self::InvalidViewport => "Background viewport dimensions must be finite and nonnegative, with a positive space width",
            Self::InvalidVisualRange => "The background visual range is invalid",
            Self::CoordinateOverflow => "A background coordinate exceeds the supported finite range",
        })
    }
}
impl std::error::Error for BackgroundGeometryError {}
impl From<BackgroundGeometryError> for VisualLineError {
    fn from(value: BackgroundGeometryError) -> Self {
        Self::Extension(value.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum PathCommand {
    Line(Point),
    Arc {
        end: Point,
        radius: f64,
        direction: SweepDirection,
    },
}
impl PathCommand {
    fn arc(end: Point, radius: f64, direction: SweepDirection) -> Self {
        if radius == 0.0 {
            Self::Line(end)
        } else {
            Self::Arc {
                end,
                radius,
                direction,
            }
        }
    }
    fn write(self, context: &mut dyn IGeometryContext) {
        match self {
            Self::Line(end) => context.line_to(end, true),
            Self::Arc {
                end,
                radius,
                direction,
            } => {
                context.arc_to(end, Size::new(radius, radius), 0.0, false, direction, true);
            }
        }
    }
}

struct Figure {
    start: Point,
    forward: Vec<PathCommand>,
    backward: VecDeque<PathCommand>,
}
struct PendingFigure {
    figure: Figure,
    last: Rect,
    radius: f64,
}

/// Joins consecutive vertically adjacent text rectangles into rounded outlines.
///
/// Add rectangles in visual row order. A vertical gap starts another figure;
/// `close_figure` explicitly separates unrelated segments. As upstream, this is
/// a text-row outline builder, not a general polygon union or rectangle sorter.
/// Native path resources are allocated only by `create_geometry`.
#[derive(Default)]
pub struct BackgroundGeometryBuilder {
    pub corner_radius: f64,
    pub align_to_whole_pixels: bool,
    pub border_thickness: f64,
    pub extend_to_full_width_at_line_end: bool,
    figures: Vec<Figure>,
    current: Option<PendingFigure>,
}
impl BackgroundGeometryBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_empty(&self) -> bool {
        self.figures.is_empty() && self.current.is_none()
    }
    /// Clears accumulated figures while retaining all formatting options.
    pub fn clear(&mut self) {
        self.figures.clear();
        self.current = None;
    }
    /// One physical pixel in view coordinates. As upstream, visual transforms
    /// are excluded; an unattached view uses the scale of one.
    pub fn pixel_size(text_view: &TextArea) -> Size {
        let scale =
            TopLevel::get_top_level(Some(text_view)).map_or(1.0, |top| top.render_scaling());
        Size::new(1.0 / scale, 1.0 / scale)
    }
    /// Adds an already aligned rectangle. Alignment options do not affect this
    /// overload; use `add_rectangle_aligned` for device-pixel alignment.
    pub fn add_rectangle(&mut self, rectangle: Rect) -> Result<(), BackgroundGeometryError> {
        self.validate_options()?;
        validate_rectangle(rectangle)?;
        self.add_rectangle_validated(rectangle)
    }
    pub fn add_rectangle_for_view(
        &mut self,
        text_view: &TextArea,
        rectangle: Rect,
    ) -> Result<(), BackgroundGeometryError> {
        self.add_rectangle_aligned(rectangle, Self::pixel_size(text_view))
    }
    /// Snaps the outer border edges using upstream's midpoint-away-from-zero
    /// rounding. `pixel_size` can specify different horizontal/vertical scales.
    pub fn add_rectangle_aligned(
        &mut self,
        rectangle: Rect,
        pixel_size: Size,
    ) -> Result<(), BackgroundGeometryError> {
        self.validate_options()?;
        validate_rectangle(rectangle)?;
        let rectangle = if self.align_to_whole_pixels {
            align_rectangle(rectangle, pixel_size, self.border_thickness)?
        } else {
            rectangle
        };
        self.add_rectangle_validated(rectangle)
    }
    pub fn add_segment<'a>(
        &mut self,
        lines: impl IntoIterator<Item = BackgroundGeometryLine<'a>>,
        segment: &dyn ISegment,
        viewport: BackgroundGeometryViewport,
        pixel_size: Size,
    ) -> Result<(), BackgroundGeometryError> {
        let rectangles = get_rects_for_segment(
            lines,
            segment,
            viewport,
            self.extend_to_full_width_at_line_end,
        )?;
        self.add_projected_rectangles(rectangles, pixel_size)
    }
    pub fn add_selection<'a>(
        &mut self,
        lines: impl IntoIterator<Item = BackgroundGeometryLine<'a>>,
        selection: SelectionSegment,
        viewport: BackgroundGeometryViewport,
        pixel_size: Size,
    ) -> Result<(), BackgroundGeometryError> {
        let rectangles = get_rects_for_selection(
            lines,
            selection,
            viewport,
            self.extend_to_full_width_at_line_end,
        )?;
        self.add_projected_rectangles(rectangles, pixel_size)
    }
    fn add_projected_rectangles(
        &mut self,
        rectangles: Vec<Rect>,
        pixel_size: Size,
    ) -> Result<(), BackgroundGeometryError> {
        self.validate_options()?;
        // Validate the complete projection before changing an existing outline.
        let rectangles = rectangles
            .into_iter()
            .map(|rectangle| {
                validate_rectangle(rectangle)?;
                let rectangle = if self.align_to_whole_pixels {
                    align_rectangle(rectangle, pixel_size, self.border_thickness)?
                } else {
                    rectangle
                };
                self.validate_corners(rectangle)?;
                Ok(rectangle)
            })
            .collect::<Result<Vec<_>, BackgroundGeometryError>>()?;
        for rectangle in rectangles {
            self.add_rectangle_validated(rectangle)?;
        }
        Ok(())
    }
    fn validate_options(&self) -> Result<(), BackgroundGeometryError> {
        if !self.corner_radius.is_finite() || self.corner_radius < 0.0 {
            return Err(BackgroundGeometryError::InvalidCornerRadius);
        }
        if !self.border_thickness.is_finite() || self.border_thickness < 0.0 {
            return Err(BackgroundGeometryError::InvalidBorderThickness);
        }
        Ok(())
    }
    fn validate_corners(&self, rectangle: Rect) -> Result<(), BackgroundGeometryError> {
        for edge in [
            rectangle.x,
            rectangle.y,
            rectangle.right(),
            rectangle.bottom(),
        ] {
            if !(edge + self.corner_radius).is_finite() || !(edge - self.corner_radius).is_finite()
            {
                return Err(BackgroundGeometryError::CoordinateOverflow);
            }
        }
        Ok(())
    }
    fn add_rectangle_validated(&mut self, rectangle: Rect) -> Result<(), BackgroundGeometryError> {
        self.validate_corners(rectangle)?;
        if let Some(current) = &self.current {
            self.validate_corners(current.last)?;
        }
        if self
            .current
            .as_ref()
            .is_some_and(|current| !close(rectangle.y, current.last.bottom()))
        {
            self.close_figure();
        }
        let radius = self.corner_radius;
        let (left, top, right, bottom) = (
            rectangle.x,
            rectangle.y,
            rectangle.right(),
            rectangle.bottom(),
        );
        let arc = |x, y, direction| PathCommand::arc(Point::new(x, y), radius, direction);
        let line = |x, y| PathCommand::Line(Point::new(x, y));
        use SweepDirection::{Clockwise, CounterClockwise};
        if let Some(current) = &mut self.current {
            let last = current.last;
            let forward = &mut current.figure.forward;
            if !close(last.right(), right) {
                let (cr, first, second) = if right < last.right() {
                    (-radius, Clockwise, CounterClockwise)
                } else {
                    (radius, CounterClockwise, Clockwise)
                };
                forward.push(arc(last.right() + cr, last.bottom(), first));
                forward.push(line(right - cr, top));
                forward.push(arc(right, top + radius, second));
            }
            forward.push(line(right, bottom - radius));
            let backward = &mut current.figure.backward;
            backward.push_front(line(last.x, last.y + radius));
            if !close(last.x, left) {
                let (cr, first, second) = if left < last.x {
                    (radius, CounterClockwise, Clockwise)
                } else {
                    (-radius, Clockwise, CounterClockwise)
                };
                backward.push_front(arc(last.x, last.bottom() - radius, first));
                backward.push_front(line(last.x - cr, last.bottom()));
                backward.push_front(arc(left + cr, last.bottom(), second));
            }
            current.last = rectangle;
            current.radius = radius;
        } else {
            let mut forward = Vec::new();
            if (left - right).abs() > radius {
                forward.push(arc(left + radius, top, Clockwise));
                forward.push(line(right - radius, top));
                forward.push(arc(right, top + radius, Clockwise));
            }
            forward.push(line(right, bottom - radius));
            self.current = Some(PendingFigure {
                figure: Figure {
                    start: Point::new(left, top + radius),
                    forward,
                    backward: VecDeque::new(),
                },
                last: rectangle,
                radius,
            });
        }
        Ok(())
    }
    /// Finishes the current outline so the next rectangle starts another one.
    pub fn close_figure(&mut self) {
        let Some(mut current) = self.current.take() else {
            return;
        };
        // Finish using the last successfully added row's settings. Changing
        // options later cannot invalidate an already prepared path.
        let radius = current.radius;
        let last = current.last;
        let backward = &mut current.figure.backward;
        backward.push_front(PathCommand::Line(Point::new(last.x, last.y + radius)));
        if (last.x - last.right()).abs() > radius {
            backward.push_front(PathCommand::arc(
                Point::new(last.x, last.bottom() - radius),
                radius,
                SweepDirection::Clockwise,
            ));
            backward.push_front(PathCommand::Line(Point::new(
                last.x + radius,
                last.bottom(),
            )));
            backward.push_front(PathCommand::arc(
                Point::new(last.right() - radius, last.bottom()),
                radius,
                SweepDirection::Clockwise,
            ));
        }
        self.figures.push(current.figure);
    }
    /// Writes filled, closed native path figures without disposing the caller's
    /// context. This also supports a native `PathGeometryContext` for inspection.
    pub fn write_to(&mut self, context: &mut dyn IGeometryContext) -> bool {
        self.close_figure();
        if self.figures.is_empty() {
            return false;
        }
        context.set_fill_rule(FillRule::EvenOdd);
        for figure in &self.figures {
            context.begin_figure(figure.start, true);
            for command in figure.forward.iter().chain(figure.backward.iter()) {
                command.write(context);
            }
            context.end_figure(true);
        }
        true
    }
    /// Creates an independent native geometry snapshot, or `None` when empty.
    /// Later additions to this builder do not mutate previously created paths.
    pub fn create_geometry(&mut self) -> Option<Ref<StreamGeometry>> {
        if self.is_empty() {
            return None;
        }
        let geometry = StreamGeometry::new();
        {
            let mut context = geometry.open();
            self.write_to(&mut context);
        }
        Some(geometry)
    }
}

pub(crate) fn validate_rectangle(rectangle: Rect) -> Result<(), BackgroundGeometryError> {
    if ![
        rectangle.x,
        rectangle.y,
        rectangle.width,
        rectangle.height,
        rectangle.right(),
        rectangle.bottom(),
    ]
    .into_iter()
    .all(f64::is_finite)
        || rectangle.width < 0.0
        || rectangle.height < 0.0
    {
        return Err(BackgroundGeometryError::InvalidRectangle);
    }
    Ok(())
}
fn close(a: f64, b: f64) -> bool {
    // AvaloniaEdit's pixel-oriented extension uses a fixed 0.01 DIP tolerance.
    MathUtilities::are_close_eps(a, b, 0.01)
}
fn align_rectangle(
    rectangle: Rect,
    pixel: Size,
    border: f64,
) -> Result<Rect, BackgroundGeometryError> {
    if !pixel.width.is_finite()
        || pixel.width <= 0.0
        || !pixel.height.is_finite()
        || pixel.height <= 0.0
    {
        return Err(BackgroundGeometryError::InvalidPixelSize);
    }
    let half = border * 0.5;
    let round = |value: f64, size: f64| (value / size).round() * size;
    let left = round(rectangle.x - half, pixel.width) + half;
    let top = round(rectangle.y - half, pixel.height) + half;
    let right = round(rectangle.right() + half, pixel.width) - half;
    let bottom = round(rectangle.bottom() + half, pixel.height) - half;
    // Subpixel rectangles can round to an inverted centerline when a border is
    // thicker than the covered area. Preserve a finite zero-width/height path.
    let aligned = Rect::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0));
    validate_rectangle(aligned)?;
    Ok(aligned)
}
