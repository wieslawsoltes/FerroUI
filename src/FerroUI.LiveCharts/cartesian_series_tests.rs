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

/// The high-to-low line and the body of every point of a financial or box series, in point
/// order: (top, bottom, body, body style).
fn candles(scene: &Scene, series: usize) -> Vec<(f64, f64, Rect, PaintStyle)> {
    let mut lines = std::collections::BTreeMap::new();
    let mut bodies = std::collections::BTreeMap::new();
    for p in &scene.primitives {
        let Some(id) = p.id.filter(|id| id.series == series) else {
            continue;
        };
        match &p.geometry {
            Geometry::Line { start, end } if start.x == end.x => {
                lines
                    .entry(id.point)
                    .or_insert((start.y.min(end.y), start.y.max(end.y)));
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
