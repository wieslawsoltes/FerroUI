//! The placement of a tooltip from the hover areas of its points, as the pinned
//! `Extensions.GetTooltipLocation`, `TooltipPlacementContext` and the `SuggestTooltipPlacement`
//! of `RectangleHoverArea`, `SemicircleHoverArea` and `AnnularSectorHoverArea`: each found point
//! suggests where the tooltip goes, and the chart's `TooltipPosition` picks a side of the points
//! (or, for a pie, the outer end of the slice), kept inside the control.

use crate::cartesian_chart_engine::CartesianChartEngine;
use crate::pie_chart_engine::PieChartEngine;
use crate::scene::{HitRegion, HitShape};
use crate::series::{Series, SeriesKind};
use ferroui_base::{Point, Rect, Size};

/// Where a chart places its tooltip (`TooltipPosition`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipPosition {
    Hidden,
    #[default]
    Auto,
    Top,
    Bottom,
    Left,
    Right,
    Center,
}

/// The side of its points a tooltip was placed at (`PopUpPlacement`, the
/// `AutoToolTipsInfo.ToolTipPlacement` of the source).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopUpPlacement {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

/// What a found point tells the placement of the tooltip (the `HoverArea` of its
/// `ChartPoint.Context`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum TooltipHoverArea {
    /// `RectangleHoverArea`: the area, the point it suggests for the tooltip
    /// (`SuggestedTooltipLocation`, which `CenterXToolTip`, `StartYToolTip` and the others set)
    /// and whether its value is below the pivot of its series (`LessThanPivot`).
    Rectangle {
        rect: Rect,
        suggested: Point,
        less_than_pivot: bool,
    },
    /// `SemicircleHoverArea`, a slice of a pie: its centre, its start and end angles in
    /// degrees, and its outer radius.
    Semicircle {
        center: Point,
        start_angle: f64,
        end_angle: f64,
        radius: f64,
    },
    /// `AnnularSectorHoverArea`, an arc node of a Sankey chart: its centre, outer radius, start
    /// angle and sweep in degrees.
    AnnularSector {
        center: Point,
        outer_radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    },
}

impl TooltipHoverArea {
    /// A rectangle that suggests its own centre (`CenterXToolTip().CenterYToolTip()`), as the
    /// heat, scatter, polar, treemap and Sankey node areas.
    pub fn centered(rect: Rect) -> Self {
        Self::Rectangle {
            rect,
            suggested: Point::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5),
            less_than_pivot: false,
        }
    }

    /// `SuggestTooltipPlacement`.
    fn suggest(&self, ctx: &mut TooltipPlacementContext, tooltip: Size) {
        match *self {
            Self::Rectangle {
                rect,
                suggested,
                less_than_pivot,
            } => {
                ctx.are_all_less_than_pivot = less_than_pivot && ctx.are_all_less_than_pivot;
                let auto_y = if less_than_pivot { tooltip.height } else { 0.0 };
                // As the source, the edges of the area are compared with the suggested points
                // the context keeps.
                if rect.y < ctx.most_top {
                    ctx.most_top = suggested.y;
                }
                if rect.y + rect.height > ctx.most_bottom {
                    ctx.most_bottom = suggested.y;
                }
                if rect.x + rect.width > ctx.most_right {
                    ctx.most_right = suggested.x;
                }
                if rect.x < ctx.most_left {
                    ctx.most_left = suggested.x;
                }
                if rect.y < ctx.most_auto_top {
                    ctx.most_auto_top = suggested.y + auto_y;
                    ctx.auto_placement = ctx.auto();
                }
                if rect.y + rect.height > ctx.most_auto_bottom {
                    ctx.most_auto_bottom = suggested.y + auto_y;
                    ctx.auto_placement = ctx.auto();
                }
            }
            Self::Semicircle {
                center,
                start_angle,
                end_angle,
                radius,
            } => {
                let start = normalized(start_angle);
                // -0.01 keeps the end of a last slice at 360 from becoming 0 (`GetActualEndAngle`).
                let end = normalized(end_angle - 0.01);
                let r = radius * 0.95;
                // The tooltip goes to the slice that reaches the furthest from the centre.
                if r <= ctx.pie_most_r {
                    return;
                }
                ctx.pie_most_r = r;
                let angle = if end > start {
                    (start + end) * 0.5
                } else {
                    (start + end + 360.0) * 0.5
                };
                let (sin, cos) = angle.to_radians().sin_cos();
                ctx.pie = Point::new(center.x + cos * r, center.y + sin * r);
            }
            Self::AnnularSector {
                center,
                outer_radius,
                start_angle,
                sweep_angle,
            } => {
                let (sin, cos) = (start_angle + sweep_angle * 0.5).to_radians().sin_cos();
                let anchor = Point::new(center.x + cos * outer_radius, center.y + sin * outer_radius);
                ctx.are_all_less_than_pivot = false;
                ctx.most_top = ctx.most_top.min(anchor.y);
                ctx.most_bottom = ctx.most_bottom.max(anchor.y);
                ctx.most_right = ctx.most_right.max(anchor.x);
                ctx.most_left = ctx.most_left.min(anchor.x);
                if anchor.y < ctx.most_auto_top {
                    ctx.most_auto_top = anchor.y;
                    ctx.auto_placement = ctx.auto();
                }
                if anchor.y > ctx.most_auto_bottom {
                    ctx.most_auto_bottom = anchor.y;
                    ctx.auto_placement = ctx.auto();
                }
            }
        }
    }
}

/// An angle in degrees in [0, 360).
fn normalized(angle: f64) -> f64 {
    let angle = angle % 360.0;
    if angle < 0.0 {
        angle + 360.0
    } else {
        angle
    }
}

/// `TooltipPlacementContext`: the extremes of the points the found hover areas suggest.
struct TooltipPlacementContext {
    auto_placement: PopUpPlacement,
    are_all_less_than_pivot: bool,
    most_top: f64,
    most_bottom: f64,
    most_right: f64,
    most_left: f64,
    most_auto_top: f64,
    most_auto_bottom: f64,
    pie: Point,
    pie_most_r: f64,
}

impl TooltipPlacementContext {
    fn new() -> Self {
        Self {
            auto_placement: PopUpPlacement::Top,
            are_all_less_than_pivot: true,
            most_top: f64::MAX,
            most_bottom: f64::MIN,
            most_right: f64::MIN,
            most_left: f64::MAX,
            most_auto_top: f64::MAX,
            most_auto_bottom: f64::MIN,
            pie: Point::new(0.0, 0.0),
            pie_most_r: 0.0,
        }
    }

    fn auto(&self) -> PopUpPlacement {
        if self.are_all_less_than_pivot {
            PopUpPlacement::Bottom
        } else {
            PopUpPlacement::Top
        }
    }
}

/// Where a tooltip goes (`GetTooltipLocation`): its top left corner and the side of its points
/// it is at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipLocation {
    pub origin: Point,
    pub placement: PopUpPlacement,
}

/// The location of a tooltip of `tooltip` size over the hover areas of the found points of a
/// chart of `control` size (`Extensions.GetTooltipLocation`). A pie chart (hover areas that are
/// slices) places it above the outer end of the slice that reaches the furthest, or below when
/// that leaves the control; the other chart kinds place it at the side of the points that
/// `position` names (`Auto` above the highest point, or below the lowest when every point is below
/// its pivot), on the opposite side when it leaves the control. The tooltip is then kept inside
/// the control. No area places it at the top left corner.
pub fn tooltip_location(
    areas: &[TooltipHoverArea],
    tooltip: Size,
    control: Size,
    position: TooltipPosition,
) -> TooltipLocation {
    let pie = areas
        .iter()
        .any(|area| matches!(area, TooltipHoverArea::Semicircle { .. }));
    let mut location = if pie {
        pie_location(areas, tooltip)
    } else {
        cartesian_location(areas, tooltip, control, position)
    };
    let p = &mut location.origin;
    if p.y < 0.0 {
        p.y = 0.0;
    }
    if p.x < 0.0 {
        p.x = 0.0;
    }
    if p.y + tooltip.height > control.height {
        p.y = control.height - tooltip.height;
    }
    if p.x + tooltip.width > control.width {
        p.x = control.width - tooltip.width;
    }
    location
}

fn cartesian_location(
    areas: &[TooltipHoverArea],
    tooltip: Size,
    control: Size,
    position: TooltipPosition,
) -> TooltipLocation {
    let mut ctx = TooltipPlacementContext::new();
    for area in areas {
        area.suggest(&mut ctx, tooltip);
    }
    if areas.is_empty() {
        return TooltipLocation {
            origin: Point::new(0.0, 0.0),
            placement: PopUpPlacement::Top,
        };
    }
    let average_x = (ctx.most_right + ctx.most_left) * 0.5 - tooltip.width * 0.5;
    let average_y = (ctx.most_top + ctx.most_bottom) * 0.5 - tooltip.height * 0.5;
    let (mut x, mut y, mut placement) = (
        average_x,
        ctx.most_auto_top - tooltip.height,
        ctx.auto_placement,
    );
    match position {
        TooltipPosition::Top => {
            y = ctx.most_top - tooltip.height;
            placement = PopUpPlacement::Top;
        }
        TooltipPosition::Bottom => {
            y = ctx.most_bottom;
            placement = PopUpPlacement::Bottom;
        }
        TooltipPosition::Left => {
            x = ctx.most_left - tooltip.width;
            y = average_y;
            placement = PopUpPlacement::Left;
        }
        TooltipPosition::Right => {
            x = ctx.most_right;
            y = average_y;
            placement = PopUpPlacement::Right;
        }
        TooltipPosition::Center => {
            y = average_y;
            placement = PopUpPlacement::Top;
        }
        TooltipPosition::Hidden | TooltipPosition::Auto => {}
    }
    // Out of the left edge: at the right of the points; out of the right edge: at their left.
    if x < 0.0 {
        return TooltipLocation {
            origin: Point::new(ctx.most_right, average_y),
            placement: PopUpPlacement::Right,
        };
    }
    if x + tooltip.width > control.width {
        return TooltipLocation {
            origin: Point::new(ctx.most_left - tooltip.width, average_y),
            placement: PopUpPlacement::Left,
        };
    }
    if y < 0.0 {
        y += tooltip.height;
        placement = PopUpPlacement::Bottom;
    }
    if y + tooltip.height > control.height {
        y -= tooltip.height;
        placement = PopUpPlacement::Top;
    }
    TooltipLocation {
        origin: Point::new(x, y),
        placement,
    }
}

fn pie_location(areas: &[TooltipHoverArea], tooltip: Size) -> TooltipLocation {
    let mut ctx = TooltipPlacementContext::new();
    for area in areas {
        area.suggest(&mut ctx, tooltip);
    }
    let x = ctx.pie.x - tooltip.width * 0.5;
    let y = ctx.pie.y - tooltip.height;
    if y < 0.0 {
        return TooltipLocation {
            origin: Point::new(x, ctx.pie.y),
            placement: PopUpPlacement::Bottom,
        };
    }
    TooltipLocation {
        origin: Point::new(x, y),
        placement: PopUpPlacement::Top,
    }
}

impl TooltipHoverArea {
    /// The hover area of a point of a polar, Sankey or treemap chart: its rectangle suggests its
    /// centre (the polar points, Sankey nodes and treemap tiles of the source), and an arc node
    /// of a Sankey chart is an annular sector (`AnnularSectorHoverArea`).
    pub fn of_region(region: &HitRegion) -> Self {
        match &region.shape {
            HitShape::AnnularSector {
                center,
                outer_radius,
                start_angle,
                sweep_angle,
                ..
            } => Self::AnnularSector {
                center: *center,
                outer_radius: *outer_radius,
                start_angle: *start_angle,
                sweep_angle: *sweep_angle,
            },
            shape => Self::centered(shape.hover_bounds()),
        }
    }
}

impl CartesianChartEngine {
    /// The hover area of a point of a Cartesian series (the `RectangleHoverArea` its series
    /// measures): a bar suggests the middle of its category at the end of the drawn bar away
    /// from the pivot (`BarSeries`); a line, step line or stacked area point the middle of its
    /// category above its marker, or below it under the pivot (`CoreLineSeries`); a range line
    /// the top of its band; a box or financial point the side of its category from the high to
    /// the low that `position` names (`ConfigureHoverAnchor`); a scatter point and a heat cell
    /// their centre. A point below the pivot of a bar or line series is `less_than_pivot`.
    pub fn tooltip_hover_area(
        series: &Series,
        region: &HitRegion,
        position: TooltipPosition,
    ) -> TooltipHoverArea {
        let rect = region.shape.hover_bounds();
        let middle_x = rect.x + rect.width * 0.5;
        let middle_y = rect.y + rect.height * 0.5;
        let below = region.value < series.pivot;
        let rectangle = |suggested: Point, less_than_pivot: bool| TooltipHoverArea::Rectangle {
            rect,
            suggested,
            less_than_pivot,
        };
        match series.kind {
            SeriesKind::Column
            | SeriesKind::Row
            | SeriesKind::StackedColumn
            | SeriesKind::StackedRow
            | SeriesKind::RangeColumn
            | SeriesKind::RangeRow => {
                // The anchor is on the drawn bar, which a row draws thinner than its strip.
                let bar = match &region.shape {
                    HitShape::WithHoverArea { shape, .. } => shape.hover_bounds(),
                    shape => shape.hover_bounds(),
                };
                let y = if below { bar.y + bar.height } else { bar.y };
                rectangle(Point::new(middle_x, y), below)
            }
            SeriesKind::Line
            | SeriesKind::StepLine
            | SeriesKind::Area
            | SeriesKind::StackedArea
            | SeriesKind::StackedStepArea => {
                // The area of the source is as tall as the geometry, which the native hover area
                // keeps at least 1 px tall.
                let half = series.geometry_size.max(0.0) * 0.5;
                let rect = Rect::new(rect.x, region.center.y - half, rect.width, half * 2.0);
                let y = if below { rect.y + rect.height } else { rect.y };
                TooltipHoverArea::Rectangle {
                    rect,
                    suggested: Point::new(middle_x, y),
                    less_than_pivot: below,
                }
            }
            SeriesKind::RangeLine => rectangle(Point::new(middle_x, rect.y), false),
            SeriesKind::Box | SeriesKind::Financial | SeriesKind::Ohlc => {
                let suggested = match position {
                    TooltipPosition::Auto | TooltipPosition::Top => Point::new(middle_x, rect.y),
                    TooltipPosition::Bottom => Point::new(middle_x, rect.y + rect.height),
                    TooltipPosition::Left => Point::new(rect.x, middle_y),
                    TooltipPosition::Right => Point::new(rect.x + rect.width, middle_y),
                    TooltipPosition::Center => Point::new(middle_x, middle_y),
                    // A hidden tooltip leaves the suggestion at its default, the origin.
                    TooltipPosition::Hidden => Point::new(0.0, 0.0),
                };
                rectangle(suggested, false)
            }
            _ => TooltipHoverArea::centered(rect),
        }
    }
}

impl PieChartEngine {
    /// The hover area of a slice (`SemicircleHoverArea` of `CorePieSeries`): its centre, angles
    /// and outer radius.
    pub fn tooltip_hover_area(region: &HitRegion) -> TooltipHoverArea {
        match &region.shape {
            HitShape::AnnularSector {
                center,
                outer_radius,
                start_angle,
                sweep_angle,
                ..
            } => TooltipHoverArea::Semicircle {
                center: *center,
                start_angle: *start_angle,
                end_angle: start_angle + sweep_angle,
                radius: *outer_radius,
            },
            shape => TooltipHoverArea::centered(shape.hover_bounds()),
        }
    }
}

/// The location of the tooltip of a land of a map (`SKDefaultGeoTooltip.Show`): centred above the
/// centre of the land (`LandCenter`), or below it with `TooltipPosition::Bottom`; with `Auto` (or
/// `Hidden`) a tooltip that leaves the top of the control goes below the centre, and one that
/// leaves the bottom above it. It is kept inside the control horizontally and at the bottom.
pub fn geo_tooltip_location(
    land_center: Point,
    tooltip: Size,
    control: Size,
    position: TooltipPosition,
) -> TooltipLocation {
    let mut placement = if position == TooltipPosition::Bottom {
        PopUpPlacement::Bottom
    } else {
        PopUpPlacement::Top
    };
    let mut x = land_center.x - tooltip.width * 0.5;
    let mut y = match placement {
        PopUpPlacement::Bottom => land_center.y,
        _ => land_center.y - tooltip.height,
    };
    if matches!(position, TooltipPosition::Auto | TooltipPosition::Hidden) {
        if placement == PopUpPlacement::Top && y < 0.0 {
            placement = PopUpPlacement::Bottom;
            y = land_center.y;
        } else if placement == PopUpPlacement::Bottom && y + tooltip.height > control.height {
            placement = PopUpPlacement::Top;
            y = land_center.y - tooltip.height;
        }
    }
    if x < 0.0 {
        x = 0.0;
    }
    if x + tooltip.width > control.width {
        x = control.width - tooltip.width;
    }
    if y + tooltip.height > control.height {
        y = control.height - tooltip.height;
    }
    TooltipLocation {
        origin: Point::new(x, y),
        placement,
    }
}
