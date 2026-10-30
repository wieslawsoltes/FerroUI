//! Source bounds/tick contracts exercised through the public engine API.

use ferroui_base::{Point, Rect, Size};
use ferroui_livecharts::measure::AxisOrientation;
use ferroui_livecharts::{
    Axis, AxisScale, CartesianChartEngine, CartesianMeasurement, CartesianOptions, Coordinate,
    Geometry, Series, SeriesKind,
};

fn measure(series: &[Series]) -> CartesianMeasurement {
    CartesianChartEngine::measure_with_axes(
        Size::new(800.0, 400.0),
        series,
        &[],
        &[],
        &CartesianOptions::default(),
    )
    .unwrap()
}

// Direct port: SeriesBoundsPaddingTests.Line_PadsBothAxes_AndReservesGeometry.
#[test]
fn source_line_pads_both_axes_and_reserves_geometry() {
    let measured = measure(&[Series::from_values(SeriesKind::Line, [1.0, 2.0, 4.0, 8.0])]);
    let bounds = measured.series_bounds[0].unwrap();
    assert_eq!(bounds.x.padding_min, bounds.x.padding_max);
    assert_eq!(bounds.y.padding_min, bounds.y.padding_max);
    assert_eq!(
        bounds.x.requested_geometry_size,
        bounds.y.requested_geometry_size
    );
    assert!(bounds.x.padding_max > 0.0);
    assert!(bounds.y.padding_max > 0.0);
    assert!(bounds.x.requested_geometry_size > 0.0);
}

// Direct port: SeriesBoundsPaddingTests.Column_PadsValueAxisOnly_CategoryGapIsAnOffset.
#[test]
fn source_column_pads_value_axis_and_offsets_categories() {
    let measured = measure(&[Series::from_values(
        SeriesKind::Column,
        [1.0, 2.0, 4.0, 8.0],
    )]);
    let bounds = measured.series_bounds[0].unwrap();
    assert_eq!(bounds.x.padding_min, bounds.x.padding_max);
    assert_eq!(bounds.y.padding_min, bounds.y.padding_max);
    assert_eq!(
        bounds.x.requested_geometry_size,
        bounds.y.requested_geometry_size
    );
    assert_eq!(bounds.x.padding_max, 0.0);
    assert!(bounds.y.padding_max > 0.0);
    assert_eq!((bounds.x.min, bounds.x.max), (-0.5, 3.5));
    assert_eq!(measured.y_scales[0].min, 0.0);
}

// Direct port: SeriesBoundsPaddingTests.Scatter_PadsBothAxes.
#[test]
fn source_scatter_pads_both_axes() {
    let mut series = Series::new(SeriesKind::Scatter);
    series.values = vec![
        Coordinate::new(1.0, 1.0),
        Coordinate::new(2.0, 4.0),
        Coordinate::new(3.0, 9.0),
    ];
    let measured = measure(&[series]);
    let bounds = measured.series_bounds[0].unwrap();
    assert_eq!(bounds.x.padding_min, bounds.x.padding_max);
    assert_eq!(bounds.y.padding_min, bounds.y.padding_max);
    assert_eq!(
        bounds.x.requested_geometry_size,
        bounds.y.requested_geometry_size
    );
    assert!(bounds.x.padding_max > 0.0 && bounds.y.padding_max > 0.0);
}

#[test]
fn zero_data_padding_and_no_markers_preserve_data_endpoints() {
    let mut series = Series::from_values(SeriesKind::Line, [1.0, 2.0, 4.0, 8.0]);
    series.data_padding = Point::new(0.0, 0.0);
    series.geometry_size = 0.0;
    let measured = measure(&[series]);
    assert_eq!(
        (measured.x_scales[0].min, measured.x_scales[0].max),
        (0.0, 3.0)
    );
    assert_eq!(
        (measured.y_scales[0].min, measured.y_scales[0].max),
        (1.0, 8.0)
    );
    let bounds = measured.series_bounds[0].unwrap();
    assert_eq!(bounds.x.padding_max, 0.0);
    assert_eq!(bounds.y.requested_geometry_size, 0.0);
}

#[test]
fn row_padding_is_applied_to_the_horizontal_value_axis() {
    let measured = measure(&[Series::from_values(SeriesKind::Row, [1.0, 2.0, 4.0, 8.0])]);
    let bounds = measured.series_bounds[0].unwrap();
    assert!(bounds.x.padding_min > 0.0);
    assert_eq!(bounds.y.padding_min, 0.0);
    assert_eq!((bounds.y.min, bounds.y.max), (-0.5, 3.5));
}

#[test]
fn source_family_padding_defaults_are_distinct() {
    for (kind, expected) in [
        (SeriesKind::Line, Point::new(0.5, 1.0)),
        (SeriesKind::StepLine, Point::new(0.5, 1.0)),
        (SeriesKind::Column, Point::new(0.0, 1.0)),
        (SeriesKind::Row, Point::new(1.0, 0.0)),
        (SeriesKind::Scatter, Point::new(1.0, 1.0)),
        (SeriesKind::Heat, Point::new(0.0, 0.0)),
        (SeriesKind::Financial, Point::new(0.5, 0.5)),
        (SeriesKind::PolarLine, Point::new(1.0, 1.5)),
    ] {
        assert_eq!(Series::new(kind).data_padding, expected, "{kind:?}");
    }
}

#[test]
fn native_measured_label_width_and_density_change_tick_spacing() {
    let axis = Axis {
        min_separators: 0,
        labels_density: 0.0,
        ..Axis::default()
    };
    let scale = AxisScale::new(&axis, 0.0, 100.0, 0.0, 400.0).unwrap();
    let size = Size::new(400.0, 200.0);
    let labels = Size::new(100.0, 20.0);
    let compact = scale.ticks_with_metrics(&axis, size, AxisOrientation::X, labels);
    let spacious = scale.ticks_with_metrics(
        &Axis {
            labels_density: 3.0,
            ..axis
        },
        size,
        AxisOrientation::X,
        labels,
    );
    assert_eq!(compact, vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
    assert_eq!(spacious, vec![0.0, 50.0, 100.0]);
}

#[test]
fn minimum_separator_count_yields_to_a_forced_step() {
    let axis = Axis {
        min_separators: 10,
        ..Axis::default()
    };
    let scale = AxisScale::new(&axis, 0.0, 100.0, 0.0, 100.0).unwrap();
    let ticks = scale.ticks_with_metrics(
        &axis,
        Size::new(100.0, 100.0),
        AxisOrientation::X,
        Size::new(100.0, 20.0),
    );
    assert_eq!(ticks.len(), 11);
    let forced = scale.ticks_with_metrics(
        &Axis {
            force_step: true,
            min_step: 25.0,
            ..axis
        },
        Size::new(100.0, 100.0),
        AxisOrientation::X,
        Size::new(100.0, 20.0),
    );
    assert_eq!(forced, vec![0.0, 25.0, 50.0, 75.0, 100.0]);
}

#[test]
fn explicit_separators_drive_native_grid_geometry() {
    let x = Axis {
        min_limit: Some(0.0),
        max_limit: Some(10.0),
        custom_separators: vec![-1.0, 2.0, 7.0, 15.0],
        ..Axis::default()
    };
    let y = Axis {
        visible: false,
        ..Axis::default()
    };
    let measured = CartesianChartEngine::measure_with_axes(
        Size::new(400.0, 300.0),
        &[],
        &[x],
        &[y],
        &CartesianOptions {
            draw_margin: Some(Rect::new(0.0, 0.0, 400.0, 300.0)),
            ..CartesianOptions::default()
        },
    )
    .unwrap();
    let positions: Vec<_> = measured
        .scene
        .primitives
        .iter()
        .filter_map(|p| match p.geometry {
            Geometry::Line { start, end } if start.x == end.x => Some(start.x),
            _ => None,
        })
        .collect();
    assert_eq!(positions, vec![80.0, 280.0]);
}

#[test]
fn separators_stop_when_the_requested_step_cannot_advance_f64() {
    let axis = Axis {
        min_limit: Some(1e100),
        max_limit: Some(1e100 + 1e90),
        force_step: true,
        min_step: 1.0,
        ..Axis::default()
    };
    let scale = AxisScale::new(&axis, 0.0, 1.0, 0.0, 1000.0).unwrap();
    let ticks = scale.ticks(&axis, 1000.0);
    assert_eq!(ticks.len(), 1);
    assert!(ticks[0].is_finite());
}

#[test]
fn padding_requests_do_not_accumulate_when_series_share_an_axis() {
    let series = Series::from_values(SeriesKind::Line, [1.0, 2.0, 4.0, 8.0]);
    let once = measure(std::slice::from_ref(&series));
    let twice = measure(&[series.clone(), series]);
    assert_eq!(once.x_scales, twice.x_scales);
    assert_eq!(once.y_scales, twice.y_scales);
}

#[test]
fn explicitly_pinned_axis_limits_override_padding_requests() {
    let mut series = Series::from_values(SeriesKind::Line, [1.0, 2.0, 4.0, 8.0]);
    series.data_padding = Point::new(1000.0, 1000.0);
    let x = Axis {
        min_limit: Some(0.0),
        max_limit: Some(3.0),
        ..Axis::default()
    };
    let y = Axis {
        min_limit: Some(1.0),
        max_limit: Some(8.0),
        ..Axis::default()
    };
    let measured = CartesianChartEngine::measure_with_axes(
        Size::new(800.0, 400.0),
        &[series],
        &[x],
        &[y],
        &CartesianOptions::default(),
    )
    .unwrap();
    assert_eq!(
        (measured.x_scales[0].min, measured.x_scales[0].max),
        (0.0, 3.0)
    );
    assert_eq!(
        (measured.y_scales[0].min, measured.y_scales[0].max),
        (1.0, 8.0)
    );
}
