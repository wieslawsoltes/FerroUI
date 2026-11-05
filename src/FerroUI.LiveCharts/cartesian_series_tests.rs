//! The `ShouldScale` fixtures of tests/CoreTests/SeriesTests at the pin: the series, values, chart
//! size and axis limits of each source test, with its assertions on the measured visuals. The
//! source reads the visual of each chart point after drawing an image; the native engine exposes
//! the same geometry in the retained scene, by point identity.

use crate::*;
use ferroui_base::{Rect, Size};

const TOLERANCE: f64 = 0.001;

fn measure(series: &[Series], x: Axis, y: Axis) -> CartesianMeasurement {
    CartesianChartEngine::measure_with_axes(
        Size::new(1000.0, 1000.0),
        series,
        &[x],
        &[y],
        &CartesianOptions::default(),
    )
    .unwrap()
}

fn limits(min: f64, max: f64) -> Axis {
    Axis {
        min_limit: Some(min),
        max_limit: Some(max),
        ..Axis::default()
    }
}

/// The rectangle visual of every point of a series, in point order.
fn rectangles(scene: &Scene, series: usize) -> Vec<Rect> {
    let mut found: Vec<_> = scene
        .primitives
        .iter()
        .filter_map(|p| match (&p.geometry, p.id) {
            (Geometry::Rectangle { rect, .. }, Some(id)) if id.series == series => {
                Some((id.point, *rect))
            }
            _ => None,
        })
        .collect();
    found.sort_by_key(|(point, _)| *point);
    found.dedup_by_key(|(point, _)| *point);
    found.into_iter().map(|(_, rect)| rect).collect()
}

/// The marker visual of every point of a series, in point order.
fn markers(scene: &Scene, series: usize) -> Vec<Rect> {
    let mut found: Vec<_> = scene
        .primitives
        .iter()
        .filter_map(|p| match (&p.geometry, p.id) {
            (Geometry::Ellipse(rect), Some(id)) if id.series == series => Some((id.point, *rect)),
            _ => None,
        })
        .collect();
    found.sort_by_key(|(point, _)| *point);
    found.into_iter().map(|(_, rect)| rect).collect()
}

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < TOLERANCE,
        "{what}: {actual} != {expected}"
    );
}

/// The steps between consecutive visuals are equal (the source compares each step with the
/// previous one).
fn assert_even_steps(positions: impl IntoIterator<Item = f64>, what: &str) {
    let positions: Vec<f64> = positions.into_iter().collect();
    for pair in positions.windows(3) {
        assert_close(pair[1] - pair[0], pair[2] - pair[1], what);
    }
}

const POWERS: [f64; 10] = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0];

// ColumnSeriesTest.ShouldScale
#[test]
fn source_column_should_scale() {
    let series = Series::from_values(SeriesKind::Column, POWERS);
    let result = measure(&[series], limits(-1.0, 10.0), limits(0.0, 512.0));
    let plot = result.scene.plot_bounds;
    let bars = rectangles(&result.scene, 0);
    assert_eq!(bars.len(), POWERS.len());
    let unit = bars[0];
    assert!(unit.width > 1.0 && unit.height > 1.0);
    for (bar, value) in bars.iter().zip(POWERS).skip(1) {
        assert_close(unit.height, bar.height / value, "height");
        assert_close(unit.width, bar.width, "width");
        assert_close((1.0 - value / 512.0) * plot.height, bar.y - plot.y, "y");
    }
    assert_even_steps(bars.iter().map(|b| b.x), "x");
}

// RowSeriesTest.ShouldScale
#[test]
fn source_row_should_scale() {
    let series = Series::from_values(SeriesKind::Row, POWERS);
    let result = measure(&[series], limits(0.0, 512.0), limits(-1.0, 10.0));
    let plot = result.scene.plot_bounds;
    let bars = rectangles(&result.scene, 0);
    assert_eq!(bars.len(), POWERS.len());
    let unit = bars[0];
    assert!(unit.width > 1.0 && unit.height > 1.0);
    for (bar, value) in bars.iter().zip(POWERS).skip(1) {
        assert_close(unit.width, bar.width / value, "width");
        assert_close(unit.height, bar.height, "height");
        assert_close(bar.x, plot.x, "x");
    }
    assert_even_steps(bars.iter().map(|b| b.y), "y");
}

// ScatterSeriesTest.ShouldScale
#[test]
fn source_scatter_should_scale() {
    let mut series = Series::new(SeriesKind::Scatter);
    series.values = (0..10)
        .map(|i| Coordinate::weighted(i as f64, POWERS[i], i as f64))
        .collect();
    series.min_geometry_size = 10.0;
    series.geometry_size = 100.0;
    let result = measure(&[series], limits(-1.0, 10.0), limits(0.0, 512.0));
    let plot = result.scene.plot_bounds;
    let visuals = markers(&result.scene, 0);
    assert_eq!(visuals.len(), POWERS.len());
    let unit = visuals[0];
    assert!(unit.width == 10.0 && unit.height == 10.0);
    for (i, (visual, value)) in visuals.iter().zip(POWERS).enumerate().skip(1) {
        let size = 10.0 + (100.0 - 10.0) * (i as f64 / 9.0);
        assert_close(visual.height, size, "height");
        assert_close(visual.width, size, "width");
        assert_close(
            (1.0 - value / 512.0) * plot.height,
            visual.y + visual.height * 0.5 - plot.y,
            "y",
        );
    }
    // The source compares the steps between the visual's left edges, which move with the
    // weighted size; the centres are evenly spaced.
    assert_even_steps(visuals.iter().map(|v| v.x + v.width * 0.5), "x");
}

// StepLineSeriesTest.ShouldScale
#[test]
fn source_step_line_should_scale() {
    let mut series = Series::from_values(SeriesKind::StepLine, POWERS);
    series.geometry_size = 10.0;
    let result = measure(&[series], limits(-1.0, 10.0), limits(0.0, 512.0));
    let plot = result.scene.plot_bounds;
    let visuals = markers(&result.scene, 0);
    assert_eq!(visuals.len(), POWERS.len());
    assert!(visuals[0].width == 10.0 && visuals[0].height == 10.0);
    for (visual, value) in visuals.iter().zip(POWERS) {
        assert_close(
            (1.0 - value / 512.0) * plot.height,
            visual.y + visual.height * 0.5 - plot.y,
            "y",
        );
    }
    assert_even_steps(visuals.iter().map(|v| v.x), "x");
    // The source also checks the end of each step segment; the native step line is one path
    // (the last series-wide primitive, drawn over the fill) whose segments end at the points.
    let path: Vec<_> = result
        .scene
        .primitives
        .iter()
        .rev()
        .find_map(|p| match (&p.geometry, p.id) {
            (Geometry::Path(commands), Some(id)) if id.series == 0 && id.point == usize::MAX => {
                Some(commands.clone())
            }
            _ => None,
        })
        .expect("the step line")
        .into_iter()
        .filter_map(|command| match command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => Some(point),
            _ => None,
        })
        .collect();
    for (visual, value) in visuals.iter().zip(POWERS) {
        let x = visual.x + visual.width * 0.5;
        let y = plot.y + (1.0 - value / 512.0) * plot.height;
        assert!(
            path.iter()
                .any(|p| (p.x - x).abs() < TOLERANCE && (p.y - y).abs() < TOLERANCE),
            "the step line passes through the point ({x}, {y})"
        );
    }
}

fn assert_stacked_bars(scene: &Scene, horizontal: bool) {
    let plot = scene.plot_bounds;
    let values = &POWERS[..9];
    let first = rectangles(scene, 0);
    let second = rectangles(scene, 1);
    assert_eq!(first.len(), values.len());
    assert_eq!(second.len(), values.len());
    let unit = first[0];
    assert!(unit.width > 1.0 && unit.height > 1.0);
    for (series, bars) in [(0, &first), (1, &second)] {
        for (i, (bar, value)) in bars.iter().zip(values).enumerate() {
            // The value below this one in its stack.
            let start = if series == 0 { 0.0 } else { values[i] };
            if horizontal {
                assert_close(unit.width, bar.width / value, "width");
                assert_close(unit.height, bar.height, "height");
                assert_close(bar.x - plot.x, start / 512.0 * plot.width, "x");
            } else {
                assert_close(unit.height, bar.height / value, "height");
                assert_close(unit.width, bar.width, "width");
                assert_close(
                    (1.0 - (value + start) / 512.0) * plot.height,
                    bar.y - plot.y,
                    "y",
                );
            }
        }
        if horizontal {
            assert_even_steps(bars.iter().map(|b| b.y), "y");
        } else {
            assert_even_steps(bars.iter().map(|b| b.x), "x");
        }
    }
}

// StackedColumnSeriesTest.ShouldScale
#[test]
fn source_stacked_column_should_scale() {
    let series = [
        Series::from_values(SeriesKind::StackedColumn, POWERS[..9].iter().copied()),
        Series::from_values(SeriesKind::StackedColumn, POWERS[..9].iter().copied()),
    ];
    let result = measure(&series, limits(-1.0, 10.0), limits(0.0, 512.0));
    assert_stacked_bars(&result.scene, false);
}

// StackedRowSeriesTest.ShouldScaleProperly. The source asserts a negative width for the first
// visual ("for a strange reason, widths are negative"), an artefact of its row geometry; the
// native rectangles are normalized, and the remaining assertions are the source's.
#[test]
fn source_stacked_row_should_scale_properly() {
    let series = [
        Series::from_values(SeriesKind::StackedRow, POWERS[..9].iter().copied()),
        Series::from_values(SeriesKind::StackedRow, POWERS[..9].iter().copied()),
    ];
    let result = measure(&series, limits(0.0, 512.0), limits(-1.0, 10.0));
    assert_stacked_bars(&result.scene, true);
}

/// The extent of the vertical lines (the wick of a candle, both whiskers of a box) and the body
/// of every point of a financial or box series, in point order: (top, bottom, body, body
/// style).
fn candles(scene: &Scene, series: usize) -> Vec<(f64, f64, Rect, PaintStyle)> {
    let mut lines = std::collections::BTreeMap::new();
    let mut bodies = std::collections::BTreeMap::new();
    for p in &scene.primitives {
        let Some(id) = p.id.filter(|id| id.series == series) else {
            continue;
        };
        match &p.geometry {
            Geometry::Line { start, end } if start.x == end.x => {
                let (top, bottom) = lines
                    .entry(id.point)
                    .or_insert((f64::INFINITY, f64::NEG_INFINITY));
                *top = top.min(start.y).min(end.y);
                *bottom = bottom.max(start.y).max(end.y);
            }
            Geometry::Rectangle { rect, .. } => {
                bodies.entry(id.point).or_insert((*rect, p.style.clone()));
            }
            _ => {}
        }
    }
    lines
        .into_iter()
        .map(|(point, (top, bottom))| {
            let (body, style) = bodies.remove(&point).expect("a body for every point");
            (top, bottom, body, style)
        })
        .collect()
}

fn assert_candles_scale(scene: &Scene, ranges: &[(f64, f64)]) {
    let plot = scene.plot_bounds;
    let visuals = candles(scene, 0);
    assert_eq!(visuals.len(), ranges.len());
    let (unit_top, unit_bottom, unit_body, _) = &visuals[0];
    let unit_height = unit_bottom - unit_top;
    assert!(unit_body.width > 1.0 && unit_height > 1.0);
    for ((top, bottom, body, _), (high, low)) in visuals.iter().zip(ranges).skip(1) {
        assert_close(unit_height, (bottom - top) / (high - low), "height");
        assert_close(unit_body.width, body.width, "width");
        assert_close((1.0 - high / 512.0) * plot.height, top - plot.y, "y");
    }
    assert_even_steps(visuals.iter().map(|(_, _, body, _)| body.x), "x");
}

// FinancialSeriesTest.ShouldScale. The source's days with a one-day unit width are the
// categories 0, 1 and 2 with a unit width of one.
#[test]
fn source_financial_should_scale() {
    let mut series = Series::new(SeriesKind::Financial);
    series.values = vec![
        Coordinate::financial(0.0, 1.0, 0.75, 0.25, 0.0),
        Coordinate::financial(1.0, 64.0, 32.0, 64.0, 32.0),
        Coordinate::financial(2.0, 512.0, 511.75, 511.25, 511.0),
    ];
    let x = Axis {
        unit_width: 1.0,
        ..Axis::default()
    };
    let result = measure(&[series], x, limits(0.0, 512.0));
    assert_candles_scale(&result.scene, &[(1.0, 0.0), (64.0, 32.0), (512.0, 511.0)]);
    // The source checks that each candle is in the paint of its direction (down when the open
    // is above the close).
    let styles: Vec<_> = candles(&result.scene, 0)
        .into_iter()
        .map(|(_, _, _, style)| style.fill)
        .collect();
    assert_eq!(
        styles[0], styles[2],
        "both down candles share the down paint"
    );
    assert_ne!(styles[0], styles[1], "the up candle has the up paint");
}

// BoxSeriesTests.ShouldScale
#[test]
fn source_box_should_scale() {
    let mut series = Series::new(SeriesKind::Box);
    series.values = vec![
        Coordinate::box_value(0.0, 1.0, 0.75, 0.25, 0.0, 0.5),
        Coordinate::box_value(1.0, 64.0, 32.0, 64.0, 32.0, 40.0),
        Coordinate::box_value(2.0, 512.0, 511.75, 511.25, 511.0, 511.5),
    ];
    let result = measure(&[series], Axis::default(), limits(0.0, 512.0));
    assert_candles_scale(&result.scene, &[(1.0, 0.0), (64.0, 32.0), (512.0, 511.0)]);
}

// HeatSeriesTest.ShouldScale: every cell has the size of the unit cell, and the colour of each
// weight between the coldest (transparent black) and the hottest (white) of the heat map is
// the byte of its fraction of 10 in every channel.
#[test]
fn source_heat_should_scale() {
    let mut series = Series::new(SeriesKind::Heat);
    for x in [0.0, 1.0] {
        for (y, weight) in [0., 1., 2., 3., 4., 5., 5., 5., 5., 5., 5.]
            .into_iter()
            .zip(0..=10)
        {
            series
                .values
                .push(Coordinate::weighted(x, y, weight as f64));
        }
    }
    series.heat_map = vec![
        ferroui_base::media::Color::from_argb(0, 0, 0, 0),
        ferroui_base::media::Color::from_argb(255, 255, 255, 255),
    ];
    let result = measure(&[series.clone()], Axis::default(), Axis::default());
    let cells: Vec<_> = result
        .scene
        .primitives
        .iter()
        .filter_map(|p| match (&p.geometry, p.id) {
            (Geometry::Rectangle { rect, .. }, Some(id)) => Some((id.point, *rect, p.style.fill)),
            _ => None,
        })
        .collect();
    assert_eq!(cells.len(), series.values.len());
    let unit = cells
        .iter()
        .find(|(point, ..)| series.values[*point].primary_value == 1.0)
        .unwrap()
        .1;
    assert!(unit.width > 1.0 && unit.height > 1.0);
    for (point, rect, fill) in &cells {
        assert_close(rect.width, unit.width, "width");
        assert_close(rect.height, unit.height, "height");
        let weight = series.values[*point].tertiary_value;
        let p = (255.0 * weight / 10.0) as u8;
        assert_eq!(
            *fill,
            Some(ferroui_base::media::Color::from_argb(p, p, p, p)),
            "weight {weight}"
        );
    }
}

// CoreHeatSeries: the cells are inset by PointPadding (4 by default) within the hover area of
// the whole cell, and ColorStops, MinValue and MaxValue place the colours.
#[test]
fn heat_point_padding_color_stops_and_weight_bounds() {
    use ferroui_base::media::Color;
    let mut series = Series::new(SeriesKind::Heat);
    series.values = vec![
        Coordinate::weighted(0.0, 0.0, 0.0),
        Coordinate::weighted(1.0, 0.0, 5.0),
        Coordinate::weighted(2.0, 0.0, 10.0),
    ];
    let (red, green, blue) = (
        Color::from_rgb(255, 0, 0),
        Color::from_rgb(0, 255, 0),
        Color::from_rgb(0, 0, 255),
    );
    series.heat_map = vec![red, green, blue];
    let fills = |series: &Series| {
        let result = measure(&[series.clone()], Axis::default(), Axis::default());
        let rects = rectangles(&result.scene, 0);
        let HitShape::Rectangle(cell) = result.scene.hits[0].shape else {
            panic!("a heat cell hovers its whole cell")
        };
        let fills: Vec<_> = result
            .scene
            .primitives
            .iter()
            .filter(|p| matches!(p.geometry, Geometry::Rectangle { .. }) && p.id.is_some())
            .map(|p| p.style.fill.unwrap())
            .collect();
        (rects, cell, fills)
    };
    let (rects, cell, colors) = fills(&series);
    assert_close(rects[0].x - cell.x, 4.0, "left padding");
    assert_close(cell.width - rects[0].width, 8.0, "horizontal padding");
    assert_close(cell.height - rects[0].height, 8.0, "vertical padding");
    assert_eq!(colors, vec![red, green, blue]);
    // Stops at 0, 0.9 and 1: the middle weight is five ninths of the way to green.
    series.color_stops = Some(vec![0.0, 0.9, 1.0]);
    series.point_padding = ferroui_base::Thickness::uniform(0.0);
    let (rects, cell, colors) = fills(&series);
    assert_close(rects[0].width, cell.width, "no padding");
    assert_eq!(colors[1], Color::from_rgb(113, 141, 0));
    // A maximum of 20 puts the hottest weight in the middle.
    series.color_stops = None;
    series.max_value = Some(20.0);
    let (_, _, colors) = fills(&series);
    assert_eq!(colors[2], green);
    // A heat map of one colour is an error, as the source throws.
    series.heat_map = vec![red];
    assert!(CartesianChartEngine::measure(
        Size::new(100.0, 100.0),
        &[series],
        &[],
        &[],
        &CartesianOptions::default()
    )
    .is_err());
}

/// The lines of each point of a series, in point order.
fn lines(scene: &Scene, series: usize) -> Vec<(usize, ferroui_base::Point, ferroui_base::Point, PaintStyle)> {
    scene
        .primitives
        .iter()
        .filter_map(|p| match (&p.geometry, p.id) {
            (Geometry::Line { start, end }, Some(id)) if id.series == series => {
                Some((id.point, *start, *end, p.style.clone()))
            }
            _ => None,
        })
        .collect()
}

// OhlcSeries with OhlcGeometry: the spine from the high to the low, the open to the left and the
// close to the right, in the bar clamped to MaxBarWidth (25), stroked by the up or down paint.
#[test]
fn ohlc_bars_draw_the_spine_and_the_open_and_close_ticks() {
    let mut series = Series::new(SeriesKind::Ohlc);
    series.values = vec![
        Coordinate::financial(0.0, 523.0, 500.0, 450.0, 400.0),
        Coordinate::financial(1.0, 420.0, 400.0, 420.0, 380.0),
    ];
    assert_eq!((series.max_bar_width, series.stroke_width), (25.0, 3.0));
    let result = measure(&[series], Axis::default(), limits(0.0, 600.0));
    let plot = result.scene.plot_bounds;
    let y = |value: f64| plot.y + (1.0 - value / 600.0) * plot.height;
    let bars = lines(&result.scene, 0);
    assert_eq!(bars.len(), 6);
    for (i, (high, open, close, low)) in [(523.0, 500.0, 450.0, 400.0), (420.0, 400.0, 420.0, 380.0)]
        .into_iter()
        .enumerate()
    {
        let [(_, spine_start, spine_end, style), (_, open_start, open_end, _), (_, close_start, close_end, _)] =
            [bars[i * 3].clone(), bars[i * 3 + 1].clone(), bars[i * 3 + 2].clone()];
        assert_close(spine_start.y, y(high), "high");
        assert_close(spine_end.y, y(low), "low");
        assert_close(open_start.y, y(open), "open");
        assert_close(close_start.y, y(close), "close");
        assert_close(open_end.x, spine_start.x, "open tick ends at the spine");
        assert_close(close_start.x, spine_start.x, "close tick starts at the spine");
        assert_close(close_end.x - open_start.x, 25.0, "bar width");
        assert_close(open_end.y, open_start.y, "flat open tick");
        assert_close(close_end.y, close_start.y, "flat close tick");
        assert_eq!(style.fill, None);
        assert_eq!(style.stroke_width, 3.0);
        assert_eq!(
            style.stroke,
            Some(if close > open {
                crate::series::FINANCIAL_UP
            } else {
                crate::series::FINANCIAL_DOWN
            })
        );
    }
    assert_eq!(result.scene.hits.len(), 2);
}

// ShowError and ErrorPaint of the bar, line and scatter series: no error bars unless the series
// shows them; a vertical line from the bottom to the top error and a horizontal line from the
// left to the right error through the point, the centre of the bar's end for a bar (beside the
// other series of its category) and the swapped axes for a row; the bounds include the errors.
#[test]
fn error_bars_follow_show_error_through_the_point() {
    use crate::kernel::Error;
    let error_series = |kind: SeriesKind| {
        let mut series = Series::new(kind);
        series.values = vec![
            Coordinate::new(0.0, 50.0).with_error(Error::new(0.2, 0.3, 15.0, 4.0)),
            Coordinate::new(1.0, 30.0).with_error(Error::symmetric(0.1, 6.0)),
        ];
        series
    };
    let hidden = measure(
        &[error_series(SeriesKind::Column)],
        Axis::default(),
        limits(0.0, 100.0),
    );
    assert!(lines(&hidden.scene, 0).is_empty());
    for kind in [SeriesKind::Column, SeriesKind::Line, SeriesKind::Scatter] {
        let mut series = error_series(kind);
        series.show_error = true;
        let mut other = Series::new(SeriesKind::Column);
        other.values = vec![Coordinate::new(0.0, 10.0), Coordinate::new(1.0, 10.0)];
        let result = measure(
            &[series, other],
            limits(-1.0, 2.0),
            limits(0.0, 100.0),
        );
        let plot = result.scene.plot_bounds;
        let y = |value: f64| plot.y + (1.0 - value / 100.0) * plot.height;
        let x = |value: f64| plot.x + (value + 1.0) / 3.0 * plot.width;
        let errors = lines(&result.scene, 0);
        assert_eq!(errors.len(), 4, "{kind:?}");
        let (_, bottom, top, style) = &errors[0];
        assert_close(bottom.y, y(46.0), "bottom error");
        assert_close(top.y, y(65.0), "top error");
        assert_eq!(style.stroke, Some(crate::series::ERROR_PAINT));
        assert_eq!(style.stroke_width, 1.0);
        let (_, left, right, _) = &errors[1];
        assert_close(left.y, y(50.0), "through the value");
        assert_close(right.x - left.x, x(0.5) - x(0.0), "left and right errors");
        let center = result.scene.hits[0].center.x;
        assert_close(bottom.x, center, "through the point or the centre of the bar");
        if kind == SeriesKind::Column {
            assert!(center < x(0.0), "the bar stands beside the other column");
        }
    }
    let mut row = error_series(SeriesKind::Row);
    row.show_error = true;
    row.error_paint = Some(ferroui_base::media::Color::from_rgb(1, 2, 3));
    let result = measure(&[row], limits(0.0, 100.0), limits(-1.0, 2.0));
    let plot = result.scene.plot_bounds;
    let x = |value: f64| plot.x + value / 100.0 * plot.width;
    let row_errors = lines(&result.scene, 0);
    let (_, low, high, style) = &row_errors[0];
    assert_close(low.x, x(46.0), "a row's bottom error is to the left");
    assert_close(high.x, x(65.0), "a row's top error is to the right");
    assert_close(low.y, high.y, "a row's value error is horizontal");
    assert_eq!(style.stroke, Some(ferroui_base::media::Color::from_rgb(1, 2, 3)));
    // Without limits the value axis reaches the top error of the first point.
    let free = measure(
        &[error_series(SeriesKind::Scatter)],
        Axis::default(),
        Axis::default(),
    );
    assert!(free.y_scales[0].max >= 65.0 && free.y_scales[0].min <= 24.0);
}

// RangeRowDateTimeAxisTests.RangeRow_WithDateTimeXAxis_HoverAreaStaysInPixelRange: the strip of
// a row is measured on its category axis, so a date axis of a two-day unit on the value side
// keeps the hover areas and the tooltip anchors in the pixels of the chart.
#[test]
fn source_range_row_with_date_time_x_axis_hover_area_stays_in_pixel_range() {
    use ferroui_base::animation::SignedDuration;
    use ferroui_base::utilities::DateTime;
    let start = DateTime::new(2026, 6, 1);
    let day = |days: f64| start.add_days(days).ticks() as f64;
    let mut series = Series::new(SeriesKind::RangeRow);
    series.values = vec![
        Coordinate::range(0.0, day(5.0), day(0.0)),
        Coordinate::range(1.0, day(8.0), day(3.0)),
        Coordinate::range(2.0, day(12.0), day(5.0)),
    ];
    let x = Axis::date_time(SignedDuration::from_days(2.0), |date| {
        date.to_string_with("MMM dd")
    });
    let y = Axis {
        labels: vec!["A".into(), "B".into(), "C".into()],
        ..Axis::default()
    };
    let result = CartesianChartEngine::measure_with_axes(
        Size::new(400.0, 400.0),
        &[series],
        &[x],
        &[y],
        &CartesianOptions::default(),
    )
    .unwrap();
    assert_eq!(result.scene.hits.len(), 3);
    for hit in &result.scene.hits {
        let area = hit.shape.hover_bounds();
        assert!(area.height < 1000.0, "hover height {}", area.height);
        assert!(area.y.abs() < 1000.0, "hover y {}", area.y);
        assert!(hit.center.x > -200.0 && hit.center.x < 600.0);
        assert!(hit.center.y > -200.0 && hit.center.y < 600.0);
    }
}

/// The chart of `SeriesHitTests`: 1000 by 1000 with hidden axes.
fn hidden_axes_scene(series: Series) -> Scene {
    let hidden = || Axis {
        visible: false,
        ..Axis::default()
    };
    measure(&[series], hidden(), hidden()).scene
}

/// The OHLC series of `SeriesHitTests`: FinancialPointI(high, open, close, low) at the index.
fn ohlc_hit_series(values: &[(f64, f64, f64, f64)]) -> Series {
    let mut series = Series::new(SeriesKind::Ohlc);
    series.values = values
        .iter()
        .enumerate()
        .map(|(i, (h, o, c, l))| Coordinate::financial(i as f64, *h, *o, *c, *l))
        .collect();
    series
}

// SeriesHitTests.Ohlc_CompareOnlyX_HitsInBarColumn, Ohlc_OutsideChart_Misses and
// Ohlc_CompareOnlyX_GapAroundVisualStillHits: the financial hover of the I-bars spans the
// category column, not the clamped bar.
#[test]
fn source_ohlc_hit_tests() {
    let scene = hidden_axes_scene(ohlc_hit_series(&[
        (10.0, 8.0, 5.0, 3.0),
        (20.0, 18.0, 15.0, 13.0),
        (30.0, 28.0, 25.0, 23.0),
    ]));
    let center = scene.hits[1].shape.hover_center();
    assert_eq!(
        scene
            .hit_test(center, FindingStrategy::CompareOnlyX, 0.0)
            .len(),
        1
    );
    let scene = hidden_axes_scene(ohlc_hit_series(&[
        (10.0, 8.0, 5.0, 3.0),
        (20.0, 18.0, 15.0, 13.0),
    ]));
    assert!(scene
        .hit_test(
            ferroui_base::Point::new(0.0, 0.0),
            FindingStrategy::CompareOnlyX,
            0.0
        )
        .is_empty());
    let HitShape::WithHoverArea { shape, .. } = &scene.hits[0].shape else {
        panic!("a financial hover area")
    };
    let HitShape::Rectangle(bar) = **shape else {
        panic!("the bar")
    };
    let probe = ferroui_base::Point::new(bar.x + bar.width + 5.0, bar.y + 1.0);
    assert!(!scene.hits[0].contains(probe), "the probe is beside the bar");
    assert_eq!(
        scene
            .hit_test(probe, FindingStrategy::CompareOnlyX, 0.0)
            .len(),
        1
    );
}

// SeriesHitTests.Heat_CompareAll_HitsCellCenter and Heat_OutsideAnyCell_Misses: the hover area
// of a cell is the whole cell.
#[test]
fn source_heat_hit_tests() {
    let mut series = Series::new(SeriesKind::Heat);
    series.values = (0..9)
        .map(|i| Coordinate::weighted((i / 3) as f64, (i % 3) as f64, i as f64))
        .collect();
    let scene = hidden_axes_scene(series);
    let center = scene.hits[4].shape.hover_center();
    let hits = scene.hit_test(center, FindingStrategy::CompareAll, 0.0);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id.point, 4);
    let mut single = Series::new(SeriesKind::Heat);
    single.values = vec![Coordinate::weighted(0.0, 0.0, 0.0)];
    assert!(hidden_axes_scene(single)
        .hit_test(
            ferroui_base::Point::new(-100.0, -100.0),
            FindingStrategy::CompareAll,
            0.0
        )
        .is_empty());
}
