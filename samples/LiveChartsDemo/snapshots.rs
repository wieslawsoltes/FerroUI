//! The Cartesian, pie, polar, Sankey and treemap scenarios of LiveCharts2's `tests/SnapshotTests`
//! at the pin, restated with the native controls. Each scenario keeps the source's chart, series,
//! values, axes and size; `livecharts-snapshots` renders them through Vello CPU, and the render job of
//! `livecharts.yml` keeps the images beside the upstream reference images. The upstream suite
//! compares SkiaSharp images pixel by pixel, which a native renderer cannot reproduce, so the
//! images are a visual reference and the tests check that every scenario measures its data.
//!
//! Source settings without a native counterpart are left out and named in a comment: tooltip
//! label formatters, `TicksAtCenter`, `IsHoverable`, paints of geometries and separators. The
//! node and link colours of a Sankey series stand for its `NodeColorMapper`, and the labels a
//! `LabelMapper` builds are the names of the nodes. The hover scenarios move the pointer after the
//! first measure, so their renders show the native tooltip.

use ferroui_base::media::Color;
use ferroui_base::{Point, Ref, Size};
use ferroui_controls_livecharts::{
    Axis, BoxSeries, CandlesticksSeries, CartesianChart, Chart, ChartPoint, ChartSeries,
    ColumnSeries, LegendPosition, LineSeries, PieChart, PieSeries, PolarChart,
    PolarLabelsPosition, PolarLineSeries, RangeColumnSeries, RangeLineSeries, RangeRowSeries,
    RowSeries, SankeyChart, SankeyLayoutKind, SankeyLink, SankeyLinks, SankeyNode, SankeyNodes,
    SankeySeries, ScatterSeries, StackedAreaSeries, StackedColumnSeries, StackedRowSeries,
    StackedStepAreaSeries, StepLineSeries, TreemapChart, TreemapNode, TreemapNodes,
    TreemapSeries,
};
use ferroui_livecharts::kernel::Coordinate;
use ferroui_livecharts::{COTANGENT_ANGLE, TANGENT_ANGLE};

/// One scenario: the name of its upstream reference image (`<Class>_<Test>`), its size and a
/// builder. `then` runs after a first measure, for the source tests that change data after
/// drawing once.
pub struct Scenario {
    pub name: &'static str,
    pub size: Size,
    pub build: fn() -> Ref<Chart>,
    pub then: Option<fn(&Chart)>,
}

const SQUARE: Size = Size::new(600.0, 600.0);

fn scenario(name: &'static str, build: fn() -> Ref<Chart>) -> Scenario {
    Scenario {
        name,
        size: SQUARE,
        build,
        then: None,
    }
}

fn changed(name: &'static str, build: fn() -> Ref<Chart>, then: fn(&Chart)) -> Scenario {
    Scenario {
        name,
        size: SQUARE,
        build,
        then: Some(then),
    }
}

fn sized(name: &'static str, width: f64, height: f64, build: fn() -> Ref<Chart>) -> Scenario {
    Scenario {
        name,
        size: Size::new(width, height),
        build,
        then: None,
    }
}

/// Every ported scenario, in the order of the source files.
pub fn scenarios() -> Vec<Scenario> {
    vec![
        scenario("ColumnSeriesTests_BasicColumn", column_basic),
        scenario("ColumnSeriesTests_LayeredColumns", column_layered),
        scenario("ColumnSeriesTests_SpacingColumns", column_spacing),
        scenario("ColumnSeriesTests_BackgroundColumns", column_background),
        scenario("RowSeriesTests_BasicRow", row_basic),
        scenario("RowSeriesTests_LayeredRows", row_layered),
        scenario("RowSeriesTests_SpacingRows", row_spacing),
        scenario("RowSeriesTests_BackgroundRows", row_background),
        scenario("LineSeriesTests_Basic", line_basic),
        scenario("LineSeriesTests_Area", line_area),
        scenario("LineSeriesTests_Straight", line_straight),
        scenario("LineSeriesTests_Curved", line_curved),
        changed("LineSeriesTests_VectorOperationsRemoveEnd", line_vector, |c| {
            edit(c, |v| v.remove_at(v.count() - 1))
        }),
        changed("LineSeriesTests_VectorOperationsRemoveStart", line_vector, |c| {
            edit(c, |v| v.remove_at(0))
        }),
        changed("LineSeriesTests_VectorOperationsRemoveMiddle", line_vector, |c| {
            edit(c, |v| v.remove_at(2))
        }),
        changed("LineSeriesTests_VectorOperationsAdd", line_vector, |c| {
            edit(c, |v| v.add(ChartPoint::with_xy(0.0, 6.0)))
        }),
        changed("LineSeriesTests_VectorOperationsInsertMiddle", line_vector, |c| {
            edit(c, |v| v.insert(2, ChartPoint::with_xy(0.0, 6.0)))
        }),
        changed("LineSeriesTests_VectorOperationsInsertStart", line_vector, |c| {
            edit(c, |v| v.insert(0, ChartPoint::with_xy(0.0, 6.0)))
        }),
        changed("LineSeriesTests_VectorOperationsInsertEnd", line_vector, |c| {
            edit(c, |v| v.insert(v.count(), ChartPoint::with_xy(0.0, 6.0)))
        }),
        changed("LineSeriesTests_VectorOperationsReplace", line_vector, |c| {
            edit(c, |v| v.set(2, ChartPoint::with_xy(0.0, 6.0)))
        }),
        changed("LineSeriesTests_VectorOperationsClear", line_vector, |c| {
            edit(c, |v| v.clear())
        }),
        changed("LineSeriesTests_Gaps", line_gaps, line_gaps_push),
        changed(
            "LineSeriesTests_Issue2132_InterpolatedInsertsWithGaps",
            line_interpolated,
            line_interpolated_inserts,
        ),
        scenario("StepLineSeriesTests_Basic", step_basic),
        scenario("StepLineSeriesTests_Area", step_area),
        scenario("StepLineSeriesTests_Styled", step_styled),
        scenario("StepLineSeriesTests_WithDataLabels", step_labels),
        scenario("StepLineSeriesTests_Gaps", step_gaps),
        scenario("ScatterSeriesTests_Basic", scatter_basic),
        scenario("ScatterSeriesTests_Bubbles", scatter_bubbles),
        scenario("StackedAreaTests_Basic", stacked_area_basic),
        scenario("StackedAreaTests_Step", stacked_area_step),
        scenario("StackedAreaTests_Issue2073_MixedSigns", stacked_area_mixed),
        scenario(
            "StackedAreaTests_Issue1923_LineOverStacked",
            stacked_area_line_over,
        ),
        scenario("StackedColumnsTests_Basic", stacked_columns_basic),
        scenario("StackedColumnsTests_StackGroup", stacked_columns_groups),
        scenario("StackedRowSeriesTests_Basic", stacked_rows_basic),
        scenario("StackedRowSeriesTests_StackGroup", stacked_rows_groups),
        scenario("StackedRowSeriesTests_Issue2152_MixedSigns", stacked_rows_mixed),
        scenario("BoxSeriesTests_BasicBox", box_basic),
        scenario("CandlesticksSeriesTests_Basic", candlesticks_basic),
        scenario("RangeSeriesTests_RangeColumn", range_column),
        scenario("RangeSeriesTests_RangeRow", range_row),
        scenario("RangeSeriesTests_RangeColumn_SwappedEndpoints", range_column_swapped),
        scenario("RangeSeriesTests_RangeLine", range_line),
        scenario("RangeSeriesTests_RangeLine_SwappedEndpoints", range_line_swapped),
        scenario("PieChartTests_Basic", pie_basic),
        scenario("PieChartTests_OuterRadius", pie_outer_radius),
        scenario("PieChartTests_InnerRadius", pie_inner_radius),
        scenario("PieChartTests_Pushout", pie_pushout),
        scenario("PieChartTests_Gauge", pie_gauge),
        scenario("PieChartTests_GaugeValueExceedsMaxValue", pie_gauge_over),
        scenario(
            "PieChartTests_GaugeValueExceedsRangeWithMinValue",
            pie_gauge_over_with_min,
        ),
        scenario("PieChartTests_GaugeMultiple", pie_gauge_multiple),
        Scenario {
            name: "PieChartTests_GaugeInTightBounds",
            size: Size::new(240.0, 100.0),
            build: pie_gauge_tight,
            then: None,
        },
        scenario("PolarChartTests_Basic", polar_basic),
        scenario("PolarChartTests_MultipleSeries", polar_multiple),
        scenario("PolarChartTests_FilledArea", polar_filled),
        scenario("PolarChartTests_WithInnerRadius", polar_inner_radius),
        scenario("PolarChartTests_OpenLine", polar_open_line),
        sized("SankeySeriesTests_TwoLevelFlow", 600., 400., sankey_two_level),
        sized("SankeySeriesTests_WithLabels", 700., 400., sankey_with_labels),
        sized("SankeySeriesTests_JobApplicationFunnel", 900., 600., sankey_funnel),
        sized("SankeySeriesTests_BipartiteArc", 700., 600., sankey_bipartite_arc),
        sized(
            "SankeySeriesTests_BipartiteArcWithLabels",
            700.,
            600.,
            sankey_bipartite_arc_labels,
        ),
        Scenario {
            name: "SankeySeriesTests_HoverTooltip_Vertical",
            size: Size::new(600., 400.),
            build: sankey_hover_vertical,
            then: Some(|c| c.move_pointer(Point::new(8., 80.))),
        },
        changed(
            "SankeySeriesTests_HoverTooltip_BipartiteArc",
            sankey_hover_arc,
            |c| c.move_pointer(Point::new(590., 300.)),
        ),
        sized("SankeySeriesTests_DefaultTheme_AppliesPaletteColor", 600., 400., sankey_theme),
        sized("SankeySeriesTests_ThreeColumnsWithBranching", 700., 400., sankey_three_columns),
        scenario("TreemapSeriesTests_FlatBasic", treemap_flat),
        scenario("TreemapSeriesTests_WithDataLabels", treemap_labels),
        changed("TreemapSeriesTests_HoverTooltip", treemap_population, |c| {
            c.move_pointer(Point::new(150., 150.))
        }),
        sized(
            "TreemapSeriesTests_TwoSeriesWithTitleAndLegend",
            900.,
            600.,
            treemap_two_series,
        ),
        scenario("TreemapSeriesTests_HierarchicalTwoLevel", treemap_hierarchical),
    ]
}

// Construction helpers.

fn cartesian(series: Vec<Ref<ChartSeries>>) -> Ref<Chart> {
    let chart = CartesianChart::new().upcast::<Chart>();
    for s in series {
        chart.series().add(s);
    }
    chart.set_animations_speed(0.0);
    chart
}

fn pie_chart(series: Vec<Ref<ChartSeries>>) -> Ref<Chart> {
    let chart = PieChart::new().upcast::<Chart>();
    for s in series {
        chart.series().add(s);
    }
    chart.set_animations_speed(0.0);
    chart
}

/// A polar chart: its X axes are the angle axes, its Y axes the radius axes.
fn polar_chart(series: Vec<Ref<ChartSeries>>) -> Ref<Chart> {
    let chart = PolarChart::new().upcast::<Chart>();
    for s in series {
        chart.series().add(s);
    }
    chart.set_animations_speed(0.0);
    chart
}

fn values<T: ferroui_base::ObjectType + ferroui_base::Upcast<ChartSeries>>(series: Ref<T>, values: &[f64]) -> Ref<ChartSeries> {
    let series = series.upcast::<ChartSeries>();
    series.set_numeric_values(values.iter().copied());
    series
}

/// Values with gaps: `None` is an empty point at its index, as a null of the source.
fn nullable<T: ferroui_base::ObjectType + ferroui_base::Upcast<ChartSeries>>(
    series: Ref<T>,
    values: &[Option<f64>],
) -> Ref<ChartSeries> {
    let series = series.upcast::<ChartSeries>();
    series.set_coordinates(values.iter().enumerate().map(|(i, v)| point(i as f64, *v)));
    series
}

fn point(x: f64, y: Option<f64>) -> Coordinate {
    match y {
        Some(y) => Coordinate::new(x, y),
        None => Coordinate {
            secondary_value: x,
            ..Coordinate::EMPTY
        },
    }
}

fn coordinates<T: ferroui_base::ObjectType + ferroui_base::Upcast<ChartSeries>>(
    series: Ref<T>,
    values: impl IntoIterator<Item = Coordinate>,
) -> Ref<ChartSeries> {
    let series = series.upcast::<ChartSeries>();
    series.set_coordinates(values);
    series
}

fn axis(configure: impl FnOnce(&Axis)) -> Ref<Axis> {
    let axis = Axis::new();
    configure(&axis);
    axis
}

fn labels(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn rgb(hex: u32) -> Color {
    Color::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// The source's numeric values map to their index; after an edit of the collection the native
/// points, which carry their X, get the index again.
fn edit(chart: &Chart, change: impl FnOnce(&ferroui_controls_livecharts::ChartPoints)) {
    let points = chart.series().get(0).values();
    change(&points);
    for (i, p) in points.to_vec().iter().enumerate() {
        p.set_x(i as f64);
    }
}

// ColumnSeriesTests and RowSeriesTests.

fn column_basic() -> Ref<Chart> {
    let chart = cartesian(vec![
        values(ColumnSeries::new(), &[1., 5., 6.]),
        values(ColumnSeries::new(), &[4., 2., 3.]),
    ]);
    // TicksAtCenter has no native counterpart.
    chart.x_axes().add(axis(|a| {
        a.set_labels(labels(&["A", "B", "C", "D", "E"]));
        a.set_min_step(1.);
        a.set_force_step(true);
    }));
    chart
}

fn layered<T: ferroui_base::ObjectType + ferroui_base::Upcast<ChartSeries>>(make: fn() -> Ref<T>) -> Ref<Chart> {
    let a = values(make(), &[6., 3., 5., 7., 3., 4., 6., 3.]);
    a.set_max_bar_width(999_999.);
    a.set_ignores_bar_position(true);
    let b = values(make(), &[2., 4., 8., 9., 5., 2., 4., 7.]);
    b.set_max_bar_width(30.);
    b.set_ignores_bar_position(true);
    cartesian(vec![a, b])
}

fn spacing<T: ferroui_base::ObjectType + ferroui_base::Upcast<ChartSeries>>(make: fn() -> Ref<T>) -> Ref<Chart> {
    let a = values(make(), &[6., 3., 5.]);
    a.set_padding(0.);
    a.set_max_bar_width(999_999.);
    cartesian(vec![a])
}

fn background<T: ferroui_base::ObjectType + ferroui_base::Upcast<ChartSeries>>(make: fn() -> Ref<T>) -> Ref<Chart> {
    // IsHoverable has no native counterpart.
    let back = values(make(), &[10., 10., 10.]);
    back.set_fill(Some(Color::from_argb(50, 180, 180, 180)));
    back.set_ignores_bar_position(true);
    let front = values(make(), &[6., 3., 5.]);
    front.set_ignores_bar_position(true);
    cartesian(vec![back, front])
}

fn column_layered() -> Ref<Chart> {
    layered(ColumnSeries::new)
}
fn column_spacing() -> Ref<Chart> {
    spacing(ColumnSeries::new)
}
fn column_background() -> Ref<Chart> {
    background(ColumnSeries::new)
}

fn row_basic() -> Ref<Chart> {
    let chart = cartesian(vec![
        values(RowSeries::new(), &[1., 5., 6.]),
        values(RowSeries::new(), &[4., 2., 3.]),
    ]);
    chart.x_axes().add(axis(|a| {
        a.set_labels(labels(&["A", "B", "C", "D", "E"]));
        a.set_min_step(1.);
        a.set_force_step(true);
    }));
    chart
}
fn row_layered() -> Ref<Chart> {
    layered(RowSeries::new)
}
fn row_spacing() -> Ref<Chart> {
    spacing(RowSeries::new)
}
fn row_background() -> Ref<Chart> {
    background(RowSeries::new)
}

// LineSeriesTests.

fn line_basic() -> Ref<Chart> {
    cartesian(vec![
        values(LineSeries::new(), &[1., 5., 7., 3.]),
        values(LineSeries::new(), &[4., 2., 8., 6.]),
        values(LineSeries::new(), &[2., 6., 4., 8.]),
    ])
}

fn line_area() -> Ref<Chart> {
    // GeometryFill and GeometryStroke null: no markers.
    let s = values(LineSeries::new(), &[1., 5., 7., 3.]);
    s.set_fill(Some(rgb(0x6495ED)));
    s.set_stroke(None);
    s.set_stroke_width(0.);
    s.set_geometry_size(0.);
    cartesian(vec![s])
}

fn line_smoothness(smoothness: f64) -> Ref<Chart> {
    let s = values(LineSeries::new(), &[0., 10., 0., 10., 0.]);
    s.set_line_smoothness(smoothness);
    cartesian(vec![s])
}
fn line_straight() -> Ref<Chart> {
    line_smoothness(0.)
}
fn line_curved() -> Ref<Chart> {
    line_smoothness(1.)
}

/// The source's `Fill = null`: no area under the line. A transparent fill removes the area of the
/// theme; the stroke keeps the palette colour of the series.
fn no_fill(series: Ref<ChartSeries>) -> Ref<ChartSeries> {
    series.set_fill(Some(Color::from_argb(0, 0, 0, 0)));
    series
}

fn line_vector() -> Ref<Chart> {
    cartesian(vec![no_fill(values(LineSeries::new(), &[1., 2., 3., 4., 5.]))])
}

fn line_gaps() -> Ref<Chart> {
    cartesian(vec![no_fill(nullable(LineSeries::new(), &[Some(1.), Some(1.)]))])
}

/// Four pushes of `[null, 1]`, each adding a point and removing the first, as the source.
fn line_gaps_push(chart: &Chart) {
    let to_add = [None, Some(1.)];
    for count in 0..4 {
        let points = chart.series().get(0).values();
        points.add(ChartPoint::from_coordinate(point(0., to_add[count % 2])));
        points.remove_at(0);
        for (i, p) in points.to_vec().iter().enumerate() {
            p.set_x(i as f64);
        }
        let _ = chart.scene(SQUARE);
    }
}

fn line_interpolated() -> Ref<Chart> {
    let s = coordinates(
        LineSeries::new(),
        [(0., 1.), (1., 2.), (2., 3.), (3., 4.)].map(|(x, y)| Coordinate::new(x, y)),
    );
    cartesian(vec![no_fill(s)])
}

/// Three passes of midpoint inserts with the source's Y sequence (values above 6 are nulls).
fn line_interpolated_inserts(chart: &Chart) {
    let sequence = [
        Some(6.),
        Some(1.),
        Some(1.),
        Some(5.),
        Some(1.),
        Some(2.),
        None,
        Some(5.),
        Some(1.),
        None,
        Some(2.),
        Some(2.),
        Some(5.),
        Some(3.),
        Some(3.),
        Some(2.),
        Some(5.),
        Some(0.),
        None,
        Some(5.),
        Some(3.),
        Some(1.),
    ];
    let mut cursor = 0;
    for _ in 0..3 {
        let points = chart.series().get(0).values();
        for i in (1..points.count()).rev() {
            let x = (points.get(i - 1).x() + points.get(i).x()) / 2.;
            points.insert(
                i,
                ChartPoint::from_coordinate(point(x, sequence[cursor % sequence.len()])),
            );
            cursor += 1;
        }
        let _ = chart.scene(SQUARE);
    }
}

// StepLineSeriesTests.

fn step_basic() -> Ref<Chart> {
    cartesian(vec![
        values(StepLineSeries::new(), &[1., 5., 7., 3.]),
        values(StepLineSeries::new(), &[4., 2., 8., 6.]),
        values(StepLineSeries::new(), &[2., 6., 4., 8.]),
    ])
}

fn step_area() -> Ref<Chart> {
    let area = |v: &[f64], fill: u32| {
        let s = values(StepLineSeries::new(), v);
        s.set_fill(Some(rgb(fill)));
        s.set_stroke(None);
        s.set_stroke_width(0.);
        s.set_geometry_size(0.);
        s
    };
    cartesian(vec![
        area(&[1., 5., 7., 3.], 0x6495ED),
        area(&[4., 2., 8., 6.], 0xF08080),
    ])
}

fn step_styled() -> Ref<Chart> {
    // The white geometry fill and its stroke paint are the native marker defaults.
    let s = values(StepLineSeries::new(), &[1., 5., 7., 3., 6.]);
    s.set_stroke(Some(rgb(0x0000FF)));
    s.set_stroke_width(4.);
    s.set_fill(Some(Color::from_argb(0x33, 0x64, 0x95, 0xED)));
    s.set_geometry_size(20.);
    cartesian(vec![s])
}

fn step_labels() -> Ref<Chart> {
    let s = values(StepLineSeries::new(), &[3., 7., 2., 5., 4.]);
    s.set_data_labels(true);
    s.set_data_labels_size(14.);
    s.set_data_labels_color(Some(rgb(0x000000)));
    cartesian(vec![s])
}

fn step_gaps() -> Ref<Chart> {
    cartesian(vec![nullable(
        StepLineSeries::new(),
        &[Some(1.), Some(5.), None, Some(3.), Some(7.), None, Some(2.)],
    )])
}

// ScatterSeriesTests.

fn scatter_basic() -> Ref<Chart> {
    cartesian(vec![coordinates(
        ScatterSeries::new(),
        [(2.2, 5.4), (3.6, 9.6), (9.9, 5.2), (8.1, 4.7), (5.3, 7.1)]
            .map(|(x, y)| Coordinate::new(x, y)),
    )])
}

fn scatter_bubbles() -> Ref<Chart> {
    let bubbles = |points: [(f64, f64, f64); 5]| {
        let s = coordinates(
            ScatterSeries::new(),
            points.map(|(x, y, w)| Coordinate::weighted(x, y, w)),
        );
        s.set_geometry_size(100.);
        s.set_min_geometry_size(5.);
        s
    };
    cartesian(vec![
        bubbles([
            (2.2, 5.4, 0.5),
            (3.6, 9.6, 0.8),
            (9.9, 5.2, 0.3),
            (8.1, 4.7, 0.7),
            (5.3, 7.1, 0.6),
        ]),
        bubbles([
            (1.2, 4.4, 0.4),
            (2.6, 8.6, 0.9),
            (8.9, 4.2, 0.2),
            (7.1, 3.7, 0.6),
            (4.3, 6.1, 0.5),
        ]),
        bubbles([
            (0.2, 3.4, 0.3),
            (1.6, 7.6, 0.7),
            (7.9, 3.2, 0.1),
            (6.1, 2.7, 0.5),
            (3.3, 5.1, 0.4),
        ]),
    ])
}

// StackedAreaTests.

const STACK_1: [f64; 7] = [3., 2., 3., 5., 3., 4., 6.];
const STACK_2: [f64; 7] = [6., 5., 6., 3., 8., 5., 2.];
const STACK_3: [f64; 7] = [4., 8., 2., 8., 9., 5., 3.];

fn stacked_area_basic() -> Ref<Chart> {
    cartesian(vec![
        values(StackedAreaSeries::new(), &STACK_1),
        values(StackedAreaSeries::new(), &STACK_2),
        values(StackedAreaSeries::new(), &STACK_3),
    ])
}

fn stacked_area_step() -> Ref<Chart> {
    cartesian(vec![
        values(StackedStepAreaSeries::new(), &STACK_1),
        values(StackedStepAreaSeries::new(), &STACK_2),
        values(StackedStepAreaSeries::new(), &STACK_3),
    ])
}

fn stacked_area_mixed() -> Ref<Chart> {
    cartesian(vec![
        values(
            StackedAreaSeries::new(),
            &[-52.89, -50.35, -42.11, 100., -34.56, -28.33],
        ),
        values(
            StackedAreaSeries::new(),
            &[1404.4, 1404.4, 1327.89, 1383.78, 1365.07, 1235.7],
        ),
    ])
}

fn stacked_area_line_over() -> Ref<Chart> {
    let line = no_fill(values(LineSeries::new(), &[8., 6., 9., 7., 10., 6., 8.]));
    line.set_stroke(Some(rgb(0xFF0000)));
    line.set_stroke_width(6.);
    line.set_geometry_size(0.);
    cartesian(vec![
        values(StackedAreaSeries::new(), &STACK_1),
        values(StackedAreaSeries::new(), &STACK_2),
        values(StackedAreaSeries::new(), &STACK_3),
        line,
    ])
}

// StackedColumnsTests and StackedRowSeriesTests.

const MIXED_1: [f64; 7] = [3., 5., -3., 2., 5., -4., -2.];
const MIXED_2: [f64; 7] = [4., 2., -3., 2., 3., 4., -2.];
const MIXED_3: [f64; 7] = [-2., 6., 6., 5., 4., 3., -2.];

fn stacked_columns_basic() -> Ref<Chart> {
    // The tooltip label formatter has no native counterpart.
    let labelled = |v: &[f64]| {
        let s = values(StackedColumnSeries::new(), v);
        s.set_data_labels(true);
        s
    };
    cartesian(vec![labelled(&MIXED_1), labelled(&MIXED_2), labelled(&MIXED_3)])
}

fn grouped<T: ferroui_base::ObjectType + ferroui_base::Upcast<ChartSeries>>(make: fn() -> Ref<T>, data_labels: bool) -> Vec<Ref<ChartSeries>> {
    [
        ([3., 5., 3.], 0),
        ([4., 2., 3.], 0),
        ([4., 6., 6.], 1),
        ([2., 5., 4.], 1),
    ]
    .iter()
    .map(|(v, group)| {
        let s = values(make(), v);
        s.set_stack_group(*group);
        s.set_data_labels(data_labels);
        s
    })
    .collect()
}

fn stacked_columns_groups() -> Ref<Chart> {
    let chart = cartesian(grouped(StackedColumnSeries::new, false));
    chart.x_axes().add(axis(|a| {
        a.set_labels_rotation(-15.);
        a.set_labels(labels(&["Category 1", "Category 2", "Category 3"]));
        a.set_min_step(1.);
        a.set_force_step(true);
    }));
    chart
}

fn stacked_rows_basic() -> Ref<Chart> {
    cartesian(vec![
        values(StackedRowSeries::new(), &STACK_1),
        values(StackedRowSeries::new(), &STACK_2),
        values(StackedRowSeries::new(), &STACK_3),
    ])
}

fn stacked_rows_groups() -> Ref<Chart> {
    cartesian(grouped(StackedRowSeries::new, true))
}

fn stacked_rows_mixed() -> Ref<Chart> {
    cartesian(vec![
        values(StackedRowSeries::new(), &MIXED_1),
        values(StackedRowSeries::new(), &MIXED_2),
        values(StackedRowSeries::new(), &MIXED_3),
    ])
}

// BoxSeriesTests and CandlesticksSeriesTests.

fn box_basic() -> Ref<Chart> {
    // BoxValue(max, third quartile, first quartile, min, median).
    let boxes = |name: &str, v: [[f64; 5]; 3]| {
        let s = coordinates(
            BoxSeries::new(),
            v.iter()
                .enumerate()
                .map(|(i, b)| Coordinate::box_value(i as f64, b[0], b[1], b[2], b[3], b[4])),
        );
        s.set_name(name.into());
        s
    };
    let chart = cartesian(vec![
        boxes(
            "Year 2023",
            [
                [100., 80., 60., 20., 70.],
                [90., 70., 50., 30., 60.],
                [80., 60., 40., 10., 50.],
            ],
        ),
        boxes(
            "Year 2024",
            [
                [90., 70., 50., 30., 60.],
                [80., 60., 40., 10., 50.],
                [70., 50., 30., 20., 40.],
            ],
        ),
        boxes(
            "Year 2025",
            [
                [80., 60., 40., 10., 50.],
                [70., 50., 30., 20., 40.],
                [60., 40., 20., 10., 30.],
            ],
        ),
    ]);
    // The separator and tick paints and SeparatorsAtCenter have no native counterpart.
    chart.x_axes().add(axis(|a| {
        a.set_labels(labels(&["Apperitizers", "Mains", "Desserts"]));
        a.set_labels_rotation(0.);
    }));
    chart
}

fn candlesticks_basic() -> Ref<Chart> {
    // FinancialPointI: high, open, close, low, at the index.
    cartesian(vec![coordinates(
        CandlesticksSeries::new(),
        [
            (523., 500., 450., 400.),
            (500., 450., 425., 400.),
            (490., 425., 400., 380.),
            (420., 400., 420., 380.),
            (520., 420., 490., 400.),
            (580., 490., 560., 440.),
        ]
        .iter()
        .enumerate()
        .map(|(i, (h, o, c, l))| Coordinate::financial(i as f64, *h, *o, *c, *l)),
    )])
}

// RangeSeriesTests: RangeValue(low, high).

const RANGES: [(f64, f64); 7] = [
    (10., 35.),
    (20., 25.),
    (5., 45.),
    (15., 40.),
    (8., 22.),
    (28., 50.),
    (12., 18.),
];

fn ranges<T: ferroui_base::ObjectType + ferroui_base::Upcast<ChartSeries>>(series: Ref<T>, v: &[(f64, f64)]) -> Ref<ChartSeries> {
    coordinates(
        series,
        v.iter()
            .enumerate()
            .map(|(i, (low, high))| Coordinate::range(i as f64, *high, *low)),
    )
}

fn limits(min: f64, max: f64) -> Ref<Axis> {
    axis(|a| {
        a.set_min_limit(Some(min));
        a.set_max_limit(Some(max));
    })
}

fn range_column() -> Ref<Chart> {
    let chart = cartesian(vec![ranges(RangeColumnSeries::new(), &RANGES)]);
    chart.y_axes().add(limits(0., 60.));
    chart
}

fn range_row() -> Ref<Chart> {
    let chart = cartesian(vec![ranges(RangeRowSeries::new(), &RANGES)]);
    chart.x_axes().add(limits(0., 60.));
    chart
}

fn range_column_swapped() -> Ref<Chart> {
    let chart = cartesian(vec![ranges(
        RangeColumnSeries::new(),
        &[(35., 10.), (25., 20.), (45., 5.)],
    )]);
    chart.y_axes().add(limits(0., 60.));
    chart
}

fn range_line() -> Ref<Chart> {
    let chart = cartesian(vec![ranges(RangeLineSeries::new(), &RANGES)]);
    chart.y_axes().add(limits(0., 60.));
    chart
}

fn range_line_swapped() -> Ref<Chart> {
    let chart = cartesian(vec![ranges(
        RangeLineSeries::new(),
        &[(35., 10.), (20., 25.), (45., 5.), (15., 40.)],
    )]);
    chart.y_axes().add(limits(0., 60.));
    chart
}

// PieChartTests. `AsPieSeries` makes one series per value; the solid gauge generator makes one
// gauge series per item, each holding its value at its own ring index and empty points
// elsewhere, and a fill series of zeros behind them.

fn as_pie_series(data: &[f64], style: impl Fn(f64, &ChartSeries)) -> Vec<Ref<ChartSeries>> {
    data.iter()
        .map(|v| {
            let s = values(PieSeries::new(), &[*v]);
            style(*v, &s);
            s
        })
        .collect()
}

fn pie_basic() -> Ref<Chart> {
    pie_chart(as_pie_series(&[2., 4., 1., 4., 3.], |_, _| {}))
}

fn pie_outer_radius() -> Ref<Chart> {
    let outer = std::cell::Cell::new(0.);
    pie_chart(as_pie_series(&[6., 5., 4., 3.], |_, s| {
        s.set_outer_radius_offset(outer.get());
        outer.set(outer.get() + 50.);
    }))
}

fn pie_inner_radius() -> Ref<Chart> {
    pie_chart(as_pie_series(&[2., 4., 1., 4., 3.], |_, s| {
        s.set_max_radial_column_width(60.)
    }))
}

fn pie_pushout() -> Ref<Chart> {
    pie_chart(as_pie_series(&[6., 5., 4., 3., 2.], |v, s| {
        if v == 6. {
            s.set_push_out(30.)
        }
    }))
}

/// `GaugeGenerator.BuildSolidGauge`: the items, then the fill series (`GaugeItem.Background`),
/// styled by the background item when there is one.
fn solid_gauge(items: &[(f64, &dyn Fn(&ChartSeries))], background: &dyn Fn(&ChartSeries)) -> Vec<Ref<ChartSeries>> {
    let count = items.len();
    let mut series: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, (value, style))| {
            let s = nullable(
                PieSeries::new(),
                &(0..count)
                    .map(|ring| (ring == i).then_some(*value))
                    .collect::<Vec<_>>(),
            );
            s.set_is_gauge(true);
            s.set_hover_push_out(0.);
            gauge_theme(&s);
            style(&s);
            s
        })
        .collect();
    let fill = values(PieSeries::new(), &vec![0.; count]);
    fill.set_is_gauge(true);
    fill.set_is_fill_series(true);
    fill.set_fill(Some(Color::from_argb(10, 30, 30, 30)));
    background(&fill);
    series.push(fill);
    series
}

/// The gauge rules of the source's default (light) theme (`HasRuleForGaugeSeries`; the fill
/// series rule is applied above), which the native controls do not apply by themselves yet.
fn gauge_theme(s: &ChartSeries) {
    s.set_stroke(None);
    s.set_data_labels(true);
    s.set_polar_labels_position(PolarLabelsPosition::ChartCenter);
    s.set_data_labels_color(Some(Color::from_rgb(70, 70, 70)));
    s.set_corner_radius(8.);
}

fn gauge(value: f64, min: f64, max: f64) -> Ref<Chart> {
    let style = |s: &ChartSeries| {
        s.set_max_radial_column_width(50.);
        s.set_data_labels(true);
        s.set_data_labels_size(50.);
    };
    let chart = pie_chart(solid_gauge(&[(value, &style)], &|_| {}));
    chart.set_initial_rotation(-90.);
    chart.set_min_value(min);
    chart.set_max_value(Some(max));
    chart
}

fn pie_gauge() -> Ref<Chart> {
    gauge(30., 0., 100.)
}
fn pie_gauge_over() -> Ref<Chart> {
    gauge(150., 0., 100.)
}
fn pie_gauge_over_with_min() -> Ref<Chart> {
    gauge(150., 30., 100.)
}

fn pie_gauge_multiple() -> Ref<Chart> {
    // The data label formatter ("<value> <name>") has no native counterpart; labels show values.
    let named = |name: &'static str| {
        move |s: &ChartSeries| {
            s.set_name(name.into());
            s.set_data_labels(true);
            s.set_polar_labels_position(PolarLabelsPosition::Start);
            s.set_inner_radius(20.);
            s.set_relative_outer_radius(8.);
            s.set_relative_inner_radius(8.);
        }
    };
    let (a, b, c) = (named("Vanessa"), named("Charles"), named("Ana"));
    let chart = pie_chart(solid_gauge(
        &[(30., &a), (50., &b), (70., &c)],
        &|s| s.set_inner_radius(20.),
    ));
    chart.set_initial_rotation(45.);
    chart.set_max_angle(270.);
    chart.set_min_value(0.);
    chart.set_max_value(Some(100.));
    chart
}

fn pie_gauge_tight() -> Ref<Chart> {
    let value = |s: &ChartSeries| {
        s.set_inner_radius(38.);
        s.set_relative_inner_radius(-4.);
        s.set_max_radial_column_width(5.);
    };
    let chart = pie_chart(solid_gauge(&[(100., &value)], &|s| {
        s.set_inner_radius(40.);
        s.set_relative_inner_radius(4.);
        s.set_outer_radius_offset(-20.);
        s.set_max_radial_column_width(5.);
    }));
    chart.set_initial_rotation(270.);
    chart.set_max_angle(360.);
    chart.set_min_value(0.);
    chart.set_max_value(Some(100.));
    chart
}

// PolarChartTests.

fn polar_basic() -> Ref<Chart> {
    let s = values(
        PolarLineSeries::new(),
        &[
            15., 14., 13., 12., 11., 10., 9., 8., 7., 6., 5., 4., 3., 2., 1.,
        ],
    );
    s.set_data_labels(true);
    s.set_geometry_size(15.);
    s.set_data_labels_size(8.);
    s.set_polar_labels_position(PolarLabelsPosition::Middle);
    s.set_data_labels_rotation(COTANGENT_ANGLE);
    s.set_polar_closed(true);
    let chart = polar_chart(vec![s]);
    chart.y_axes().add(axis(|a| {
        a.set_labels_angle(-60.);
        a.set_max_limit(Some(30.));
    }));
    chart.x_axes().add(axis(|a| a.set_labels_rotation(TANGENT_ANGLE)));
    chart
}

const POLAR_1: [f64; 6] = [10., 8., 12., 6., 14., 4.];
const POLAR_2: [f64; 6] = [6., 14., 4., 10., 8., 12.];
const POLAR_3: [f64; 6] = [12., 6., 10., 14., 4., 8.];

fn radius_to_20(chart: Ref<Chart>) -> Ref<Chart> {
    chart.y_axes().add(axis(|a| a.set_max_limit(Some(20.))));
    chart
}

fn polar_multiple() -> Ref<Chart> {
    radius_to_20(polar_chart(vec![
        values(PolarLineSeries::new(), &POLAR_1),
        values(PolarLineSeries::new(), &POLAR_2),
        values(PolarLineSeries::new(), &POLAR_3),
    ]))
}

fn polar_filled() -> Ref<Chart> {
    let filled = |v: &[f64], fill: Color| {
        let s = values(PolarLineSeries::new(), v);
        s.set_fill(Some(fill));
        s.set_geometry_size(0.);
        s
    };
    radius_to_20(polar_chart(vec![
        filled(&POLAR_1, Color::from_argb(0x33, 0x64, 0x95, 0xED)),
        filled(&POLAR_2, Color::from_argb(0x33, 0xF0, 0x80, 0x80)),
    ]))
}

fn polar_inner_radius() -> Ref<Chart> {
    let chart = radius_to_20(polar_chart(vec![values(PolarLineSeries::new(), &POLAR_1)]));
    chart.set_inner_radius(50.);
    chart
}

fn polar_open_line() -> Ref<Chart> {
    // GeometryFill and GeometryStroke have no native counterpart; the markers take the series
    // paint.
    let s = values(PolarLineSeries::new(), &POLAR_1);
    s.set_polar_closed(false);
    s.set_geometry_size(12.);
    radius_to_20(polar_chart(vec![s]))
}

// SankeySeriesTests.

const SANKEY_BLUE: Color = Color::from_rgb(96, 138, 218);
const SANKEY_LINK: Color = Color::from_argb(90, 96, 138, 218);
const BLACK: Color = Color::from_rgb(0, 0, 0);
const WHITE: Color = Color::from_rgb(255, 255, 255);

fn sankey_chart(series: Ref<SankeySeries>) -> Ref<Chart> {
    let chart = SankeyChart::new().upcast::<Chart>();
    chart.series().add(series.upcast());
    chart.set_animations_speed(0.0);
    chart
}

/// A Sankey series of the named nodes, linked by their indexes, with the source's fill and link
/// fill (`None` leaves them to the theme).
fn sankey(
    names: &[&str],
    links: &[(usize, usize, f64)],
    fill: Option<Color>,
    link_fill: Option<Color>,
) -> Ref<SankeySeries> {
    let nodes: Vec<_> = names.iter().map(|name| SankeyNode::with_name(*name)).collect();
    let series = SankeySeries::new();
    series.set_links(SankeyLinks::from_items(links.iter().map(
        |&(source, target, weight)| SankeyLink::with_nodes(&nodes[source], &nodes[target], weight),
    )));
    series.set_nodes(SankeyNodes::from_items(nodes));
    series.set_fill(fill);
    series.set_link_fill(link_fill);
    series
}

/// The node colours of a series, in place of the source's `NodeColorMapper`.
fn colour_nodes(series: &SankeySeries, colours: &[Color]) {
    for (node, colour) in series.nodes().to_vec().iter().zip(colours) {
        node.set_color(Some(*colour));
    }
}

fn sankey_labels(series: &SankeySeries, colour: Color, size: f64) {
    series.set_data_labels(true);
    series.set_data_labels_color(Some(colour));
    series.set_data_labels_size(size);
}

const TWO_LEVEL: [(usize, usize, f64); 5] =
    [(0, 3, 8.), (0, 4, 4.), (1, 3, 6.), (1, 4, 2.), (2, 4, 10.)];

fn sankey_two_level() -> Ref<Chart> {
    let s = sankey(
        &["A", "B", "C", "X", "Y"],
        &TWO_LEVEL,
        Some(SANKEY_BLUE),
        Some(SANKEY_LINK),
    );
    s.set_node_width(16.);
    sankey_chart(s)
}

fn sankey_with_labels() -> Ref<Chart> {
    let s = sankey(
        &["Alice", "Bob", "Carol", "Trips", "Other"],
        &TWO_LEVEL,
        Some(SANKEY_BLUE),
        Some(SANKEY_LINK),
    );
    s.set_node_width(16.);
    sankey_labels(&s, BLACK, 16.);
    sankey_chart(s)
}

fn sankey_funnel() -> Ref<Chart> {
    // The source's LabelMapper writes the node value before the name; the names say it here.
    let s = sankey(
        &[
            "9 Applications",
            "4 1st Interviews",
            "3 Rejected",
            "2 No Reply",
            "2 2nd Interviews",
            "1 Dropped by Myself",
            "1 No Offer Received",
            "2 Offers",
            "1 Accepted",
            "1 Declined",
        ],
        &[
            (0, 1, 4.),
            (0, 2, 3.),
            (0, 3, 2.),
            (1, 4, 2.),
            (1, 5, 1.),
            (1, 6, 1.),
            (4, 7, 2.),
            (7, 8, 1.),
            (7, 9, 1.),
        ],
        Some(SANKEY_BLUE),
        Some(WHITE),
    );
    let orange = Color::from_rgb(220, 130, 50);
    colour_nodes(
        &s,
        &[
            SANKEY_BLUE,
            SANKEY_BLUE,
            Color::from_rgb(210, 80, 80),
            Color::from_rgb(180, 180, 180),
            SANKEY_BLUE,
            orange,
            orange,
            SANKEY_BLUE,
            Color::from_rgb(64, 175, 110),
            orange,
        ],
    );
    s.set_node_width(18.);
    s.set_node_padding(14.);
    s.set_node_corner_radius(3.);
    sankey_labels(&s, BLACK, 13.);
    sankey_chart(s)
}

fn sankey_bipartite_arc() -> Ref<Chart> {
    let s = sankey(
        &[
            "Furniture",
            "Technology",
            "Office Supplies",
            "Chairs",
            "Phones",
            "Storage",
            "Tables",
            "Paper",
        ],
        &[
            (0, 3, 12.),
            (0, 6, 6.),
            (1, 4, 14.),
            (1, 5, 8.),
            (2, 5, 4.),
            (2, 7, 9.),
            (2, 3, 2.),
        ],
        Some(SANKEY_BLUE),
        Some(SANKEY_LINK),
    );
    colour_nodes(
        &s,
        &[
            SANKEY_BLUE,
            Color::from_rgb(52, 78, 144),
            Color::from_rgb(150, 188, 232),
            SANKEY_BLUE,
            Color::from_rgb(52, 78, 144),
            Color::from_rgb(120, 158, 220),
            Color::from_rgb(170, 60, 80),
            Color::from_rgb(232, 168, 130),
        ],
    );
    s.set_layout(SankeyLayoutKind::BipartiteArc);
    s.set_arc_span_degrees(150.);
    s.set_node_width(20.);
    s.set_node_padding(8.);
    sankey_chart(s)
}

fn sankey_bipartite_arc_labels() -> Ref<Chart> {
    let s = sankey(
        &["Alpha", "Beta", "Gamma", "Xenon", "Yttrium", "Zinc"],
        &[(0, 3, 8.), (0, 4, 4.), (1, 3, 6.), (1, 5, 5.), (2, 4, 10.), (2, 5, 3.)],
        Some(SANKEY_BLUE),
        Some(SANKEY_LINK),
    );
    s.set_layout(SankeyLayoutKind::BipartiteArc);
    s.set_node_width(14.);
    sankey_labels(&s, BLACK, 14.);
    sankey_chart(s)
}

fn sankey_hover_vertical() -> Ref<Chart> {
    let s = sankey(
        &["A", "B", "C", "X", "Y"],
        &[(0, 3, 8.), (0, 4, 4.), (1, 3, 6.), (2, 4, 10.)],
        Some(SANKEY_BLUE),
        Some(SANKEY_LINK),
    );
    s.set_node_width(16.);
    sankey_chart(s)
}

fn sankey_hover_arc() -> Ref<Chart> {
    let s = sankey(
        &["Furniture", "Technology", "Chairs", "Phones", "Storage"],
        &[(0, 2, 12.), (1, 3, 14.), (1, 4, 8.)],
        Some(SANKEY_BLUE),
        Some(SANKEY_LINK),
    );
    s.set_layout(SankeyLayoutKind::BipartiteArc);
    s.set_node_width(20.);
    sankey_chart(s)
}

fn sankey_theme() -> Ref<Chart> {
    // No fill and no link fill: the theme's palette colour and its LinkFill at alpha 90.
    let s = sankey(
        &["A", "B", "X", "Y"],
        &[(0, 2, 5.), (0, 3, 3.), (1, 3, 7.)],
        None,
        None,
    );
    s.set_node_width(16.);
    sankey_chart(s)
}

fn sankey_three_columns() -> Ref<Chart> {
    let s = sankey(
        &["A", "B", "C", "M", "N", "P", "Q"],
        &[
            (0, 3, 10.),
            (1, 3, 5.),
            (1, 4, 3.),
            (2, 4, 12.),
            (3, 5, 8.),
            (3, 6, 7.),
            (4, 5, 5.),
            (4, 6, 10.),
        ],
        Some(Color::from_rgb(232, 105, 105)),
        Some(Color::from_argb(80, 232, 105, 105)),
    );
    s.set_node_width(14.);
    s.set_node_padding(12.);
    sankey_chart(s)
}

// TreemapSeriesTests.

fn treemap_chart(series: Vec<Ref<TreemapSeries>>) -> Ref<Chart> {
    let chart = TreemapChart::new().upcast::<Chart>();
    for s in series {
        chart.series().add(s.upcast());
    }
    chart.set_animations_speed(0.0);
    chart
}

/// A treemap series with the source's fill and white stroke of 2.
fn treemap(nodes: Vec<Ref<TreemapNode>>, padding: f64, fill: Color) -> Ref<TreemapSeries> {
    let series = TreemapSeries::new();
    series.set_nodes(TreemapNodes::from_items(nodes));
    series.set_padding(padding);
    series.set_fill(Some(fill));
    series.set_stroke(Some(WHITE));
    series.set_stroke_width(2.);
    series
}

fn leaves(values: &[(f64, &str)]) -> Vec<Ref<TreemapNode>> {
    values
        .iter()
        .map(|&(value, name)| TreemapNode::with_value(value, name))
        .collect()
}

fn branch(name: &str, values: &[(f64, &str)]) -> Ref<TreemapNode> {
    TreemapNode::with_children(name, TreemapNodes::from_items(leaves(values)))
}

fn treemap_flat() -> Ref<Chart> {
    // The default padding of 2.
    treemap_chart(vec![treemap(
        leaves(&[(40., "A"), (25., "B"), (15., "C"), (10., "D"), (7., "E"), (3., "F")]),
        2.,
        SANKEY_BLUE,
    )])
}

fn treemap_labels() -> Ref<Chart> {
    let s = treemap(
        leaves(&[
            (40., "Asia"),
            (25., "Europe"),
            (15., "Americas"),
            (10., "Africa"),
            (7., "Oceania"),
            (3., "Antarctica"),
        ]),
        4.,
        SANKEY_BLUE,
    );
    s.set_data_labels(true);
    s.set_data_labels_color(Some(WHITE));
    s.set_data_labels_size(18.);
    treemap_chart(vec![s])
}

const ASIA: [(f64, &str); 4] = [
    (1400., "China"),
    (1300., "India"),
    (125., "Japan"),
    (270., "Indonesia"),
];
const EUROPE: [(f64, &str); 4] = [
    (83., "Germany"),
    (67., "France"),
    (60., "Italy"),
    (46., "Spain"),
];

fn treemap_population() -> Ref<Chart> {
    treemap_chart(vec![treemap(
        vec![branch("Asia", &ASIA), branch("Europe", &EUROPE)],
        4.,
        SANKEY_BLUE,
    )])
}

fn treemap_two_series() -> Ref<Chart> {
    let year = |name: &str, values: &[(f64, &str)], fill: Color| {
        let s = treemap(leaves(values), 4., fill);
        s.set_name(name.into());
        s.set_data_labels(true);
        s.set_data_labels_color(Some(WHITE));
        s.set_data_labels_size(16.);
        s
    };
    let chart = treemap_chart(vec![
        year(
            "2023",
            &[(60., "Americas"), (50., "EMEA"), (28., "APAC"), (15., "Other")],
            SANKEY_BLUE,
        ),
        year(
            "2024",
            &[(40., "Americas"), (28., "EMEA"), (16., "APAC"), (8., "Other")],
            Color::from_rgb(232, 105, 105),
        ),
    ]);
    // The source title is a 22 px label with a padding of 12 in black.
    chart.set_title("Sales by Region".into());
    chart.set_title_size(22.);
    chart.set_legend_position(LegendPosition::Right);
    chart
}

fn treemap_hierarchical() -> Ref<Chart> {
    let s = treemap(
        vec![
            branch("Asia", &ASIA),
            branch("Europe", &EUROPE),
            branch("Americas", &[(330., "USA"), (220., "Brazil"), (130., "Mexico")]),
        ],
        4.,
        Color::from_argb(160, 96, 138, 218),
    );
    s.set_corner_radius(4.);
    treemap_chart(vec![s])
}

/// Builds a scenario and measures it at its size, applying its change after a first measure.
pub fn measure(scenario: &Scenario) -> Result<Ref<Chart>, ferroui_livecharts::ChartError> {
    let chart = (scenario.build)();
    chart.scene(scenario.size)?;
    if let Some(then) = scenario.then {
        then(&chart);
        chart.scene(scenario.size)?;
    }
    Ok(chart)
}
