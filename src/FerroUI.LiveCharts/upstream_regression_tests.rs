//! Behavioral test adaptations from the pinned source. These explicitly test the
//! numerical contracts, not upstream paint-task identities or native host events.

use crate::*;
use ferroui_base::{media::Color, Rect, Size};

fn measure(series: &[Series], x: &[Axis], y: &[Axis]) -> CartesianMeasurement {
    CartesianChartEngine::measure_with_axes(
        Size::new(1000.0, 400.0),
        series,
        x,
        y,
        &CartesianOptions::default(),
    )
    .unwrap()
}

// tests/CoreTests/SeriesTests/RangeSeriesAutoBoundsTests.cs
#[test]
fn range_row_series_auto_bounds_include_lowest_low_endpoint() {
    let mut series = Series::new(SeriesKind::RangeRow);
    series.values = vec![
        Coordinate::range(0.0, 5.0, 0.0),
        Coordinate::range(1.0, 10.0, 6.0),
        Coordinate::range(2.0, 14.0, 8.0),
    ];
    let result = measure(&[series], &[], &[]);
    assert!(result.x_scales[0].min <= 0.0);
    assert!(result.x_scales[0].max >= 14.0);
}
#[test]
fn range_column_series_auto_bounds_include_reversed_low_endpoint() {
    let mut series = Series::new(SeriesKind::RangeColumn);
    series.values = vec![
        Coordinate::range(0.0, 100.0, 0.0),
        Coordinate::range(1.0, 150.0, 100.0),
        Coordinate::range(2.0, 130.0, 180.0),
    ];
    let result = measure(&[series], &[], &[]);
    assert!(result.y_scales[0].min <= 0.0);
    assert!(result.y_scales[0].max >= 180.0);
}
#[test]
fn range_line_series_auto_bounds_include_lowest_low_endpoint() {
    let mut series = Series::new(SeriesKind::RangeLine);
    series.values = vec![
        Coordinate::range(0.0, 100.0, 0.0),
        Coordinate::range(1.0, 150.0, 100.0),
        Coordinate::range(2.0, 130.0, 180.0),
    ];
    let result = measure(&[series], &[], &[]);
    assert!(result.y_scales[0].min <= 0.0);
    assert!(result.y_scales[0].max >= 180.0);
}

// tests/CoreTests/SeriesTests/ColumnMaxBarWidthTests.cs
#[test]
fn column_width_is_clamped_to_max_bar_width() {
    for cap in [50.0, 120.0] {
        let mut series = Series::from_values(SeriesKind::Column, [10.0; 3]);
        series.max_bar_width = cap;
        series.padding = 0.0;
        let result = measure(
            &[series],
            &[Axis {
                min_limit: Some(0.5),
                max_limit: Some(1.5),
                ..Axis::default()
            }],
            &[],
        );
        let hit = result.scene.hits.iter().find(|h| h.id.point == 1).unwrap();
        let HitShape::WithHoverArea { shape, bounds } = &hit.shape else {
            panic!("column has a separate category hover slot")
        };
        let HitShape::Rectangle(rect) = shape.as_ref() else {
            panic!("column exact geometry must be a rectangle")
        };
        assert!((rect.width - cap).abs() < 1.5);
        assert!(bounds.width > rect.width);
    }
}

// tests/CoreTests/SeriesTests/LineSeriesTest.cs: ShouldScale (marker and end-point assertions).
#[test]
fn line_series_scale_has_even_categories_and_exponential_values() {
    let mut series = Series::from_values(
        SeriesKind::Line,
        [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0],
    );
    series.geometry_size = 10.0;
    let result = measure(
        &[series],
        &[Axis {
            min_limit: Some(-1.0),
            max_limit: Some(10.0),
            ..Axis::default()
        }],
        &[Axis {
            min_limit: Some(0.0),
            max_limit: Some(512.0),
            ..Axis::default()
        }],
    );
    let points = &result.scene.hits;
    let dx = points[1].center.x - points[0].center.x;
    for pair in points.windows(2) {
        assert!((pair[1].center.x - pair[0].center.x - dx).abs() < 0.001);
    }
    for point in points {
        let expected = result.scene.plot_bounds.y
            + (1.0 - point.value / 512.0) * result.scene.plot_bounds.height;
        assert!((point.center.y - expected).abs() < 0.001);
    }
    let markers: Vec<_> = result
        .scene
        .primitives
        .iter()
        .filter_map(|p| {
            if let Geometry::Ellipse(rect) = p.geometry {
                Some(rect)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(markers.len(), 10);
    assert!(markers.iter().all(|r| r.width == 10.0 && r.height == 10.0));
}

// tests/CoreTests/SeriesTests/LineSeriesTest.cs: MarkerColorMatchesUserStroke.
#[test]
fn line_marker_color_matches_user_stroke() {
    let colors = [
        Color::from_rgb(0, 0, 255),
        Color::from_rgb(0, 128, 0),
        Color::from_rgb(255, 0, 0),
    ];
    let series: Vec<_> = colors
        .iter()
        .map(|color| {
            let mut s = Series::from_values(SeriesKind::Line, [1.0, 2.0, 3.0]);
            s.stroke = Some(*color);
            s
        })
        .collect();
    let result = measure(&series, &[], &[]);
    for primitive in result
        .scene
        .primitives
        .iter()
        .filter(|p| matches!(p.geometry, Geometry::Ellipse(_)))
    {
        assert_eq!(
            primitive.style.stroke,
            Some(colors[primitive.id.unwrap().series])
        );
    }
    for legend in &result.scene.legend {
        assert_eq!(legend.color, colors[legend.series]);
    }
}

// tests/CoreTests/SeriesTests/Issue1970Tests.cs: numerical scene adaptation.
#[test]
fn reassigned_shorter_line_values_remove_stale_points_and_shapes() {
    prune_scene(SeriesKind::Line);
}
#[test]
fn reassigned_shorter_scatter_values_remove_stale_points_and_shapes() {
    prune_scene(SeriesKind::Scatter);
}
fn prune_scene(kind: SeriesKind) {
    let mut series = Series::from_values(kind, (1..=20).map(f64::from));
    assert_eq!(measure(&[series.clone()], &[], &[]).scene.hits.len(), 20);
    series.values = (1..=5)
        .map(|i| Coordinate::new((i - 1) as f64, i as f64))
        .collect();
    let actual = measure(&[series], &[], &[]).scene;
    let expected = measure(
        &[Series::from_values(kind, (1..=5).map(f64::from))],
        &[],
        &[],
    )
    .scene;
    assert_eq!(actual.hits.len(), 5);
    assert_eq!(actual.primitives.len(), expected.primitives.len());
    assert_eq!(actual, expected);
}

// tests/CoreTests/ChartTests/CollapsedDrawMarginTests.cs: retained scene adaptation.
#[test]
fn collapsed_plot_uses_real_zero_area_clip_and_recovers() {
    let series = Series::from_values(SeriesKind::Line, [1.0, 2.0, 3.0, 4.0, 5.0]);
    let collapsed = CartesianChartEngine::measure(
        Size::new(300.0, 300.0),
        &[series.clone()],
        &[],
        &[],
        &CartesianOptions {
            draw_margin: Some(Rect::new(200.0, 200.0, 0.0, 0.0)),
            ..CartesianOptions::default()
        },
    )
    .unwrap();
    assert_eq!(collapsed.plot_bounds.width, 0.0);
    assert!(collapsed
        .primitives
        .iter()
        .filter(|p| p.id.is_some())
        .all(|p| p.clip == Some(collapsed.plot_bounds)));
    assert!(collapsed
        .hit_test(
            ferroui_base::Point::new(200.0, 200.0),
            FindingStrategy::CompareAll,
            100.0
        )
        .is_empty());
    let restored = measure(&[series], &[], &[]).scene;
    assert!(restored.plot_bounds.width > 0.0 && restored.plot_bounds.height > 0.0);
}

#[test]
fn logarithmic_column_baseline_and_small_positive_data_remain_visible() {
    let result = measure(
        &[Series::from_values(SeriesKind::Column, [0.01, 0.1, 1.0])],
        &[],
        &[Axis {
            logarithmic_base: Some(10.0),
            ..Axis::default()
        }],
    );
    assert_eq!(result.scene.hits.len(), 3);
    assert!(result.y_scales[0].min <= 0.01);
    assert!(result
        .scene
        .hits
        .iter()
        .all(|h| h.center.x.is_finite() && h.center.y.is_finite()));
}
