//! The `ShouldPlaceToolTips` fixtures of tests/CoreTests/SeriesTests at the pin: the series, chart
//! size, pointer and `TooltipPosition` of each source test, with its assertions on where the
//! tooltip goes (`GetTooltipLocation` over the hover areas of the found points). The source reads
//! the location of its `SKDefaultTooltip`; the native placement takes the size of any tooltip,
//! here 80 by 40.

use crate::tooltips::{tooltip_location, PopUpPlacement, TooltipLocation, TooltipPosition};
use crate::*;
use ferroui_base::{Point, Size};

const TOOLTIP: Size = Size {
    width: 80.0,
    height: 40.0,
};
/// The tooltip of the scatter, financial and box fixtures, whose formatters write four lines: it is
/// taller than the room above their highest points, as the source's.
const TALL: Size = Size {
    width: 80.0,
    height: 100.0,
};
const CONTROL: f64 = 300.0;
const POSITIONS: [TooltipPosition; 6] = [
    TooltipPosition::Top,
    TooltipPosition::Bottom,
    TooltipPosition::Left,
    TooltipPosition::Right,
    TooltipPosition::Center,
    TooltipPosition::Auto,
];

fn hidden_axis() -> Axis {
    Axis {
        visible: false,
        ..Axis::default()
    }
}

/// A 300 by 300 Cartesian chart of one series with hidden axes, as the source fixtures: the
/// source's draw margin is then the whole control, where the native automatic margin keeps a
/// padding, so the fixtures set it.
fn cartesian(series: Series) -> (Scene, Series) {
    cartesian_with_y(series, hidden_axis())
}

fn cartesian_with_y(mut series: Series, y: Axis) -> (Scene, Series) {
    series.data_padding = Point::new(0.0, 0.0);
    let scene = CartesianChartEngine::measure(
        Size::new(CONTROL, CONTROL),
        std::slice::from_ref(&series),
        &[hidden_axis()],
        &[y],
        &CartesianOptions {
            draw_margin: Some(ferroui_base::Rect::new(0.0, 0.0, CONTROL, CONTROL)),
            ..CartesianOptions::default()
        },
    )
    .unwrap();
    (scene, series)
}

/// Where the tooltip of the points found under the pointer goes (`DrawToolTip` of a Cartesian
/// chart: its finding strategy, then `GetTooltipLocation`).
fn place_cartesian(
    chart: &(Scene, Series),
    pointer: (f64, f64),
    position: TooltipPosition,
) -> TooltipLocation {
    place_sized(chart, pointer, position, TOOLTIP)
}

fn place_sized(
    (scene, series): &(Scene, Series),
    pointer: (f64, f64),
    position: TooltipPosition,
    tooltip: Size,
) -> TooltipLocation {
    let found = scene.hit_test(
        Point::new(pointer.0, pointer.1),
        FindingStrategy::Automatic,
        0.0,
    );
    assert!(!found.is_empty(), "a point under {pointer:?}");
    let areas: Vec<_> = found
        .iter()
        .map(|hit| CartesianChartEngine::tooltip_hover_area(series, hit, position))
        .collect();
    tooltip_location(&areas, tooltip, Size::new(CONTROL, CONTROL), position)
}

fn close(actual: f64, expected: f64, tolerance: f64, what: &str) {
    assert!(
        (actual - expected).abs() < tolerance,
        "{what}: {actual} != {expected}"
    );
}

/// The assertions every `ShouldPlaceToolTips` makes for its six positions about an anchor: the
/// tooltip above, below, at the left, at the right, centred on it, and above it with `Auto`. A
/// bar, box or candle has room around its anchor: `half` is the distance from the anchor to the
/// top and bottom of the area (and to its sides), which the source writes as `1 / 5d * 300 *
/// 0.5d`.
fn assert_six_positions(
    chart: &(Scene, Series),
    pointer: (f64, f64),
    anchor: (f64, f64),
    half: f64,
    tooltip: Size,
) {
    let (w, h) = (tooltip.width, tooltip.height);
    for position in POSITIONS {
        let at = place_sized(chart, pointer, position, tooltip).origin;
        let what = format!("{position:?}");
        let (x, y) = match position {
            TooltipPosition::Top | TooltipPosition::Auto => {
                (anchor.0 - w * 0.5, anchor.1 - h - half)
            }
            TooltipPosition::Bottom => (anchor.0 - w * 0.5, anchor.1 + half),
            TooltipPosition::Left => (anchor.0 - w - half, anchor.1 - h * 0.5),
            TooltipPosition::Right => (anchor.0 + half, anchor.1 - h * 0.5),
            _ => (anchor.0 - w * 0.5, anchor.1 - h * 0.5),
        };
        close(at.x, x, 0.1, &what);
        close(at.y, y, 0.1, &what);
    }
}

/// A hidden value axis of the bounds of the source's bars, which do not reach their pivot
/// (`DataFactory.GetCartesianBounds`); the native bounds of a bar include its base.
fn value_axis(min: f64, max: f64) -> Axis {
    Axis {
        min_limit: Some(min),
        max_limit: Some(max),
        ..hidden_axis()
    }
}

// ColumnSeriesTest.ShouldPlaceToolTips
#[test]
fn source_column_should_place_tool_tips() {
    let mut series = Series::from_values(SeriesKind::Column, [1.0, 2.0, 3.0, 4.0, 5.0]);
    series.name = "Series #1".into();
    let chart = cartesian_with_y(series.clone(), value_axis(1.0, 5.0));
    assert_six_positions(&chart, (150.0, 150.0), (150.0, 150.0), 0.0, TOOLTIP);
    let auto = place_cartesian(&chart, (150.0, 150.0), TooltipPosition::Auto);
    assert_eq!(auto.placement, PopUpPlacement::Top);
    // Below the pivot the tooltip goes under the end of the bar.
    let negative = cartesian_with_y(
        Series::from_values(SeriesKind::Column, [-1.0, -2.0, -3.0, -4.0, -5.0]),
        value_axis(-5.0, -1.0),
    );
    let auto = place_cartesian(&negative, (150.0, 150.0), TooltipPosition::Auto);
    close(auto.origin.x + TOOLTIP.width * 0.5, 150.0, 0.1, "x");
    close(auto.origin.y, 150.0, 0.1, "y");
    assert_eq!(auto.placement, PopUpPlacement::Bottom);
}

/// The line and step line fixtures: five points of no geometry size, the middle one under the
/// pointer, and the ends, where the tooltip turns to the side that keeps it in the control.
fn assert_line_places_tool_tips(kind: SeriesKind) {
    let mut series = Series::from_values(kind, [1.0, 2.0, 3.0, 4.0, 5.0]);
    series.geometry_size = 0.0;
    let chart = cartesian(series.clone());
    assert_six_positions(&chart, (150.0, 150.0), (150.0, 150.0), 0.0, TOOLTIP);
    let auto = place_cartesian(&chart, (150.0, 150.0), TooltipPosition::Auto);
    assert_eq!(auto.placement, PopUpPlacement::Top);
    let mut negative = series.clone();
    negative.values = Series::from_values(kind, [-1.0, -2.0, -3.0, -4.0, -5.0]).values;
    let negative = cartesian(negative);
    let auto = place_cartesian(&negative, (150.0, 150.0), TooltipPosition::Auto);
    close(auto.origin.x + TOOLTIP.width * 0.5, 150.0, 0.1, "x");
    close(auto.origin.y, 150.0, 0.1, "y");
    assert_eq!(auto.placement, PopUpPlacement::Bottom);
    let right = place_cartesian(&chart, (299.0, 150.0), TooltipPosition::Auto);
    close(right.origin.x, CONTROL - TOOLTIP.width, 2.0, "x at the right end");
    assert_eq!(right.placement, PopUpPlacement::Left);
    let left = place_cartesian(&chart, (1.0, 150.0), TooltipPosition::Auto);
    close(left.origin.x, 0.0, 2.0, "x at the left end");
    assert_eq!(left.placement, PopUpPlacement::Right);
}

// LineSeriesTest.ShouldPlaceToolTips
#[test]
fn source_line_should_place_tool_tips() {
    assert_line_places_tool_tips(SeriesKind::Line);
}

// StepLineSeriesTest.ShouldPlaceToolTipsCorrectly
#[test]
fn source_step_line_should_place_tool_tips() {
    assert_line_places_tool_tips(SeriesKind::StepLine);
}

// ScatterSeriesTest.ShouldPlaceToolTips
#[test]
fn source_scatter_should_place_tool_tips() {
    // The 24 px geometry of the source's ScatterSeries, under the pointer near the corners.
    let mut series = Series::from_values(SeriesKind::Scatter, [1.0, 2.0, 3.0, 4.0, 5.0]);
    series.geometry_size = 24.0;
    let chart = cartesian(series);
    assert_six_positions(&chart, (150.0, 150.0), (150.0, 150.0), 0.0, TALL);
    let auto = place_sized(&chart, (150.0, 150.0), TooltipPosition::Auto, TALL);
    assert_eq!(auto.placement, PopUpPlacement::Top);
    // Above the control it goes below the point.
    let (x, y) = (CONTROL * 3.0 / 4.0, CONTROL / 4.0);
    let auto = place_sized(&chart, (x, y), TooltipPosition::Auto, TALL);
    close(auto.origin.x, x - TALL.width * 0.5, 0.1, "x");
    close(auto.origin.y, y, 0.1, "y");
    assert_eq!(auto.placement, PopUpPlacement::Bottom);
    let corner = place_sized(&chart, (295.0, 5.0), TooltipPosition::Auto, TALL);
    close(corner.origin.x, CONTROL - TALL.width, 0.1, "x");
    assert_eq!(corner.placement, PopUpPlacement::Left);
    let corner = place_sized(&chart, (5.0, 295.0), TooltipPosition::Auto, TALL);
    close(corner.origin.x, 0.0, 0.1, "x");
    assert_eq!(corner.placement, PopUpPlacement::Right);
}

// HeatSeriesTest.ShouldPlaceToolTips
#[test]
fn source_heat_should_place_tool_tips() {
    use ferroui_base::media::Color;
    let weights = [
        [150.0, 123.0, 310.0, 225.0, 473.0],
        [432.0, 312.0, 135.0, 78.0, 124.0],
        [543.0, 134.0, 524.0, 315.0, 145.0],
        [90.0, 123.0, 70.0, 123.0, 432.0],
        [90.0, 123.0, 70.0, 123.0, 432.0],
    ];
    let mut series = Series::new(SeriesKind::Heat);
    for (x, row) in weights.iter().enumerate() {
        for (y, weight) in row.iter().enumerate() {
            series
                .values
                .push(Coordinate::weighted(x as f64, y as f64, *weight));
        }
    }
    series.heat_map = vec![
        Color::from_rgb(255, 241, 118),
        Color::from_rgb(47, 79, 79),
        Color::from_rgb(0, 0, 255),
    ];
    let chart = cartesian(series);
    assert_six_positions(&chart, (150.0, 150.0), (150.0, 150.0), 0.0, TOOLTIP);
    let auto = place_cartesian(&chart, (150.0, 150.0), TooltipPosition::Auto);
    assert_eq!(auto.placement, PopUpPlacement::Top);
    let cell = CONTROL / 5.0;
    // The top cell: no room above it, so below its centre.
    let auto = place_cartesian(&chart, (150.0, 10.0), TooltipPosition::Auto);
    close(auto.origin.x, 150.0 - TOOLTIP.width * 0.5, 0.1, "x");
    close(auto.origin.y, cell * 0.5, 0.1, "y");
    assert_eq!(auto.placement, PopUpPlacement::Bottom);
    let corner = place_cartesian(&chart, (295.0, 5.0), TooltipPosition::Auto);
    close(corner.origin.x, CONTROL - cell * 0.5 - TOOLTIP.width, 0.1, "x");
    close(corner.origin.y, -(TOOLTIP.height * 0.5 - cell * 0.5), 0.1, "y");
    assert_eq!(corner.placement, PopUpPlacement::Left);
    let corner = place_cartesian(&chart, (5.0, 295.0), TooltipPosition::Auto);
    close(corner.origin.x, cell * 0.5, 0.1, "x");
    close(
        corner.origin.y,
        CONTROL - TOOLTIP.height * 0.5 - cell * 0.5,
        0.1,
        "y",
    );
    assert_eq!(corner.placement, PopUpPlacement::Right);
}

/// The financial and box fixtures: five categories of a fifth of the control, each from its high
/// (or maximum) at `i + 1` to its low (or minimum) at `i`, so the area of the middle one spans
/// 120 to 180 around the pointer.
fn assert_category_places_tool_tips(series: Series) {
    let chart = cartesian(series);
    let half = CONTROL / 5.0 * 0.5;
    assert_six_positions(&chart, (150.0, 150.0), (150.0, 150.0), half, TALL);
    let auto = place_sized(&chart, (150.0, 150.0), TooltipPosition::Auto, TALL);
    assert_eq!(auto.placement, PopUpPlacement::Top);
    // The highest category has no room above it: below its top.
    let auto = place_sized(
        &chart,
        (CONTROL * 4.0 / 5.0 - 10.0, CONTROL / 5.0 + 10.0),
        TooltipPosition::Auto,
        TALL,
    );
    close(
        auto.origin.x + TALL.width * 0.5,
        CONTROL / 5.0 + 150.0,
        0.1,
        "x",
    );
    close(auto.origin.y, CONTROL / 5.0, 0.1, "y");
    assert_eq!(auto.placement, PopUpPlacement::Bottom);
    let right = place_sized(&chart, (299.0, 150.0), TooltipPosition::Auto, TALL);
    close(right.origin.x, CONTROL - half - TALL.width, 0.0001, "x");
    assert_eq!(right.placement, PopUpPlacement::Left);
    let left = place_sized(&chart, (1.0, 150.0), TooltipPosition::Auto, TALL);
    close(left.origin.x, half, 0.0001, "x");
    assert_eq!(left.placement, PopUpPlacement::Right);
}

// FinancialSeriesTest.ShouldPlaceToolTips
#[test]
fn source_financial_should_place_tool_tips() {
    let mut series = Series::new(SeriesKind::Financial);
    series.name = "Aaaa".into();
    series.max_bar_width = 1000.0;
    series.values = (0..5)
        .map(|i| {
            let i = i as f64;
            Coordinate::financial(i, i + 1.0, i + 0.75, i + 0.25, i)
        })
        .collect();
    assert_category_places_tool_tips(series);
}

// BoxSeriesTests.ShouldPlaceToolTips
#[test]
fn source_box_should_place_tool_tips() {
    let mut series = Series::new(SeriesKind::Box);
    series.name = "A".into();
    series.max_bar_width = 1000.0;
    series.values = (0..5)
        .map(|i| {
            let i = i as f64;
            Coordinate::box_value(i, i + 1.0, i + 0.75, i + 0.25, i, i + 0.5)
        })
        .collect();
    assert_category_places_tool_tips(series);
}

// PieSeriesTest.ShouldPlaceToolTips: four slices of one; the tooltip of the slice under the
// pointer goes out from the centre into its quadrant.
#[test]
fn source_pie_should_place_tool_tips() {
    let series: Vec<_> = (0..4)
        .map(|_| Series::from_values(SeriesKind::Pie, [1.0]))
        .collect();
    let scene = PieChartEngine::measure(
        Size::new(CONTROL, CONTROL),
        &series,
        &PieOptions::default(),
    )
    .unwrap();
    let place = |pointer: (f64, f64)| {
        let found = scene.hit_test(
            Point::new(pointer.0, pointer.1),
            FindingStrategy::Automatic,
            0.0,
        );
        assert!(!found.is_empty(), "a slice under {pointer:?}");
        let areas: Vec<_> = found
            .iter()
            .map(|hit| PieChartEngine::tooltip_hover_area(hit))
            .collect();
        tooltip_location(
            &areas,
            TOOLTIP,
            Size::new(CONTROL, CONTROL),
            TooltipPosition::Auto,
        )
        .origin
    };
    let bottom = |p: Point| p.y + TOOLTIP.height;
    let p = place((160.0, 160.0));
    assert!(p.x - 150.0 > 0.0 && bottom(p) - 150.0 > 0.0, "{p:?}");
    let p = place((140.0, 160.0));
    assert!(p.x - 150.0 < 0.0 && bottom(p) - 150.0 > 0.0, "{p:?}");
    let p = place((140.0, 140.0));
    assert!(p.x - 150.0 < 0.0 && bottom(p) - 150.0 < 0.0, "{p:?}");
    let p = place((160.0, 140.0));
    assert!(p.x - 150.0 > 0.0 && bottom(p) - 150.0 < 0.0, "{p:?}");
}

// PolarLineSeriesTest.ShouldPlaceToolTipsCorrectly: the point of the lowest radius is at the
// centre, and the tooltip goes to each side of it.
#[test]
fn source_polar_line_should_place_tool_tips() {
    let mut series = Series::from_values(SeriesKind::PolarLine, [1.0, 2.0, 3.0, 4.0, 5.0]);
    series.geometry_size = 10.0;
    series.data_padding = Point::new(0.0, 0.0);
    let radius = Axis {
        min_limit: Some(1.0),
        max_limit: Some(5.0),
        ..Axis::default()
    };
    let scene = PolarChartEngine::measure(
        Size::new(CONTROL, CONTROL),
        &[series],
        &hidden_axis(),
        &radius,
        &PolarOptions::default(),
    )
    .unwrap();
    let found = scene.hit_test(Point::new(150.0, 150.0), FindingStrategy::Automatic, 0.0);
    assert_eq!(found.len(), 1);
    let areas: Vec<_> = found
        .iter()
        .map(|hit| TooltipHoverArea::of_region(hit))
        .collect();
    let (w, h) = (TOOLTIP.width, TOOLTIP.height);
    for position in &POSITIONS[..5] {
        let at = tooltip_location(&areas, TOOLTIP, Size::new(CONTROL, CONTROL), *position).origin;
        let what = format!("{position:?}");
        let (x, y) = match position {
            TooltipPosition::Top => (150.0 - w * 0.5, 150.0 - h),
            TooltipPosition::Bottom => (150.0 - w * 0.5, 150.0),
            TooltipPosition::Left => (150.0 - w, 150.0 - h * 0.5),
            TooltipPosition::Right => (150.0, 150.0 - h * 0.5),
            _ => (150.0 - w * 0.5, 150.0 - h * 0.5),
        };
        close(at.x, x, 0.1, &what);
        close(at.y, y, 0.1, &what);
    }
}

/// `SuggestTooltipPlacement` of the hover areas and the edges of `GetTooltipLocation`: the
/// rectangle compares its edges with the suggested points, a point below its pivot pushes an
/// `Auto` tooltip below it, the arc of a Sankey node anchors at the middle of its outer edge,
/// and a tooltip is kept inside the control.
#[test]
fn hover_areas_suggest_the_placement_of_the_source() {
    let control = Size::new(400.0, 300.0);
    let below = TooltipHoverArea::Rectangle {
        rect: ferroui_base::Rect::new(100.0, 100.0, 50.0, 50.0),
        suggested: Point::new(125.0, 150.0),
        less_than_pivot: true,
    };
    let auto = tooltip_location(&[below], TOOLTIP, control, TooltipPosition::Auto);
    assert_eq!(auto.origin, Point::new(85.0, 150.0));
    assert_eq!(auto.placement, PopUpPlacement::Bottom);
    // One point above the pivot is enough to place it above.
    let above = TooltipHoverArea::Rectangle {
        rect: ferroui_base::Rect::new(200.0, 80.0, 50.0, 50.0),
        suggested: Point::new(225.0, 80.0),
        less_than_pivot: false,
    };
    let auto = tooltip_location(&[below, above], TOOLTIP, control, TooltipPosition::Auto);
    assert_eq!(auto.placement, PopUpPlacement::Top);
    assert_eq!(auto.origin, Point::new(175.0 - 40.0, 80.0 - 40.0));
    let arc = TooltipHoverArea::AnnularSector {
        center: Point::new(200.0, 150.0),
        outer_radius: 100.0,
        start_angle: 0.0,
        sweep_angle: 180.0,
    };
    // The middle of the outer edge, at the bottom of the centre.
    let top = tooltip_location(&[arc], TOOLTIP, control, TooltipPosition::Top);
    close(top.origin.x, 160.0, 1e-9, "x");
    close(top.origin.y, 210.0, 1e-9, "y");
    // Below the bottom edge it goes above the anchor.
    let short = Size::new(400.0, 280.0);
    let bottom = tooltip_location(&[arc], TOOLTIP, short, TooltipPosition::Bottom);
    close(bottom.origin.y, 210.0, 1e-9, "y");
    assert_eq!(bottom.placement, PopUpPlacement::Top);
    // A slice near the left edge: the tooltip is kept inside the control.
    let slice = TooltipHoverArea::Semicircle {
        center: Point::new(20.0, 150.0),
        start_angle: 0.0,
        end_angle: 90.0,
        radius: 10.0,
    };
    let pie = tooltip_location(&[slice], TOOLTIP, control, TooltipPosition::Auto);
    assert_eq!(pie.origin.x, 0.0);
    let r = 9.5 * std::f64::consts::FRAC_1_SQRT_2;
    close(pie.origin.y, 150.0 + r - 40.0, 1e-3, "y");
    assert_eq!(pie.placement, PopUpPlacement::Top);
    // No area: the top left corner.
    let none = tooltip_location(&[], TOOLTIP, control, TooltipPosition::Auto);
    assert_eq!(none.origin, Point::new(0.0, 0.0));
}

// SKChartingVisualsTests.SKDefaultGeoTooltip_ShowThenHideRunsTheFullLifecycle and
// SKDefaultGeoTooltip_BottomPlacementIsExercisedByFixedTooltipPosition: the tooltip of a land of
// a 400 by 400 map above its centre, below it with `Bottom`, and below it with `Auto` when above
// leaves the control.
#[test]
fn source_geo_tooltip_is_placed_about_the_centre_of_the_land() {
    use crate::tooltips::geo_tooltip_location;
    let control = Size::new(400.0, 400.0);
    let top = geo_tooltip_location(
        Point::new(200.0, 200.0),
        TOOLTIP,
        control,
        TooltipPosition::Auto,
    );
    assert_eq!(top.origin, Point::new(160.0, 160.0));
    assert_eq!(top.placement, PopUpPlacement::Top);
    let bottom = geo_tooltip_location(
        Point::new(100.0, 50.0),
        TOOLTIP,
        control,
        TooltipPosition::Bottom,
    );
    assert_eq!(bottom.origin, Point::new(60.0, 50.0));
    assert_eq!(bottom.placement, PopUpPlacement::Bottom);
    let flipped = geo_tooltip_location(Point::new(20.0, 10.0), TOOLTIP, control, TooltipPosition::Auto);
    assert_eq!(flipped.origin, Point::new(0.0, 10.0));
    assert_eq!(flipped.placement, PopUpPlacement::Bottom);
    // An explicit top stays above, outside the control.
    let explicit = geo_tooltip_location(Point::new(200.0, 10.0), TOOLTIP, control, TooltipPosition::Top);
    assert_eq!(explicit.origin, Point::new(160.0, -30.0));
    // At the bottom edge it is kept inside.
    let low = geo_tooltip_location(Point::new(200.0, 390.0), TOOLTIP, control, TooltipPosition::Bottom);
    assert_eq!(low.origin, Point::new(160.0, 360.0));
}


