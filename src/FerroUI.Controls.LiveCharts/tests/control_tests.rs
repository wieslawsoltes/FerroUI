//! Native host contracts: shared ownership, deep observation, markup and bindings.

use ferroui_base::data::BindingPriority;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::reactive::{IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Ref, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls_livecharts::*;
use ferroui_livecharts::kernel::{Coordinate, Error};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::{cell::Cell, rc::Rc};

fn line() -> Ref<LineSeries> {
    let series = LineSeries::new();
    series.set_numeric_values([3.0, 8.0, 5.0]);
    series
}

#[test]
fn coordinate_roundtrip_preserves_all_dimensions_nulls_and_asymmetric_errors() {
    let _scope = Dispatcher::unit_test_scope();
    for coordinate in [
        Coordinate::EMPTY,
        Coordinate::default(),
        Coordinate::financial(2.0, 12.0, 7.0, 9.0, 4.0),
        Coordinate::box_value(3.0, 15.0, 12.0, 6.0, 2.0, 8.0),
        Coordinate::new(1.0, 8.0).with_error(Error::new(2.0, 3.0, 4.0, 5.0)),
        Coordinate::new(1.0, 8.0).with_error(Error::new(0.0, 0.0, 0.0, 0.0)),
    ] {
        assert_eq!(
            ChartPoint::from_coordinate(coordinate).snapshot(),
            coordinate
        );
    }
    let point = ChartPoint::from_coordinate(
        Coordinate::new(1.0, 8.0).with_error(Error::new(2.0, 3.0, 4.0, 5.0)),
    );
    assert_eq!(point.error_y_minus(), 5.0);
    assert_eq!(point.error_y_plus(), 4.0);
}


#[test]
fn default_collections_are_per_instance_and_derived_series_keep_their_kind() {
    let _scope = Dispatcher::unit_test_scope();
    let a = CartesianChart::new();
    let b = CartesianChart::new();
    a.series().add(line().upcast());
    assert!(b.series().is_empty());
    let axis = Axis::new();
    axis.set_name("first only".into());
    a.x_axes().add(axis);
    assert!(b.x_axes().is_empty());
    assert_eq!(CandlesticksSeries::new().kind(), SeriesKind::Financial);
    assert_eq!(
        StackedStepAreaSeries::new().kind(),
        SeriesKind::StackedStepArea
    );
    assert_eq!(PolarLineSeries::new().kind(), SeriesKind::PolarLine);
    let c = LineSeries::new();
    let d = LineSeries::new();
    c.values().add(ChartPoint::with_xy(0.0, 1.0));
    assert!(d.values().is_empty());
}


#[test]
fn shared_series_mutation_invalidates_each_chart_and_scene_cache() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let a = CartesianChart::new();
    let b = CartesianChart::new();
    let series = line();
    let shared = ChartSeriesCollection::from_items([series.clone().upcast()]);
    a.set_series(shared.clone());
    b.set_series(shared);
    let size = Size::new(640.0, 360.0);
    let before = a.scene(size).unwrap();
    assert!(Rc::ptr_eq(&before, &a.scene(size).unwrap()));
    let revisions = (a.revision(), b.revision());
    series.values().get(1).set_y(30.0);
    assert!(a.revision() > revisions.0 && b.revision() > revisions.1);
    let after = a.scene(size).unwrap();
    assert!(!Rc::ptr_eq(&before, &after));
    assert!(after.hits.iter().any(|hit| hit.value == 30.0));
    assert_eq!(
        before
            .hits
            .iter()
            .find(|hit| hit.id.point == 1)
            .unwrap()
            .value,
        8.0
    );
}

#[test]
fn removed_and_replaced_sources_stop_notifying_and_drop_releases_owners() {
    let _scope = Dispatcher::unit_test_scope();
    let chart = CartesianChart::new();
    let series = line();
    let point = series.values().get(0);
    chart.series().add(series.clone().upcast());
    let old_points = series.values();
    series.set_values(ChartPoints::new());
    let revision = chart.revision();
    point.set_y(90.0);
    old_points.add(ChartPoint::with_xy(9.0, 10.0));
    assert_eq!(chart.revision(), revision);
    assert!(!old_points.has_collection_changed_subscribers());
    assert_eq!(point.property_changed_subscriber_count(), 0);
    chart.series().clear();
    let revision = chart.revision();
    series.set_name("removed".into());
    assert_eq!(chart.revision(), revision);
    chart.series().add(series.clone().upcast());
    let weak = chart.downgrade();
    drop(chart);
    assert!(weak.upgrade().is_none());
    series.set_name("still alive".into());
}

#[test]
fn reentrant_change_and_update_handlers_can_replace_data() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = CartesianChart::new();
    let series = line();
    chart.series().add(series.clone().upcast());
    let changed_once = Rc::new(Cell::new(false));
    let guard = changed_once.clone();
    let weak = series.downgrade();
    let changes = series.changed(move || {
        if !guard.replace(true) {
            weak.upgrade().unwrap().set_numeric_values([20.0, 40.0]);
        }
    });
    series.values().get(0).set_y(7.0);
    assert_eq!(series.values().count(), 2);
    assert_eq!(series.values().get(1).y(), 40.0);
    changes.dispose();

    let updated_once = Rc::new(Cell::new(false));
    let guard = updated_once.clone();
    let weak = series.downgrade();
    let updates = chart.update_finished(move || {
        if !guard.replace(true) {
            weak.upgrade().unwrap().values().get(0).set_y(60.0);
        }
    });
    let first = chart.scene(Size::new(400.0, 240.0)).unwrap();
    let second = chart.scene(Size::new(400.0, 240.0)).unwrap();
    assert!(!Rc::ptr_eq(&first, &second));
    assert!(second.hits.iter().any(|hit| hit.value == 60.0));
    updates.dispose();
}

#[test]
fn native_binding_changes_point_and_invalidates_chart() {
    let _scope = Dispatcher::unit_test_scope();
    let chart = CartesianChart::new();
    let series = line();
    chart.series().add(series.clone().upcast());
    let point = series.values().get(0);
    let source = Rc::new(LightweightSubject::<f64>::new());
    let subscription = point.bind(
        ChartPoint::y_property(),
        source.clone(),
        BindingPriority::LocalValue,
    );
    let revision = chart.revision();
    source.on_next(42.0);
    assert_eq!(point.y(), 42.0);
    assert!(chart.revision() > revision);
    subscription.dispose();
    let revision = chart.revision();
    source.on_next(99.0);
    assert_eq!(chart.revision(), revision);
}


#[test]
fn source_family_padding_defaults_remain_overridable_by_native_style_priority() {
    let _scope = Dispatcher::unit_test_scope();
    let column = ColumnSeries::new();
    assert_eq!(column.data_padding(), ferroui_base::Point::new(0.0, 1.0));
    let styled = ferroui_base::Point::new(3.0, 4.0);
    let subscription = column
        .set_value_with_priority(
            ChartSeries::data_padding_property(),
            styled,
            BindingPriority::Style,
        )
        .unwrap();
    assert_eq!(column.snapshot().data_padding, styled);
    subscription.dispose();
    assert_eq!(
        column.snapshot().data_padding,
        ferroui_base::Point::new(0.0, 1.0)
    );
}


#[test]
fn native_gradient_stop_changes_invalidate_and_replaced_brushes_unsubscribe() {
    use ferroui_base::media::{Colors, GradientStop, IBrush, LinearGradientBrush};
    let _scope = Dispatcher::unit_test_scope();
    let chart = CartesianChart::new();
    let series = line();
    let brush = LinearGradientBrush::new();
    let stop = GradientStop::with_color_and_offset(Colors::RED, 0.0);
    brush.gradient_stops().add(stop.clone());
    brush
        .gradient_stops()
        .add(GradientStop::with_color_and_offset(Colors::BLUE, 1.0));
    let interface: Rc<dyn IBrush> = brush.clone().into();
    series.set_fill_brush(Some(interface));
    chart.series().add(series.clone().upcast());
    let revision = chart.revision();
    stop.set_color(Colors::GREEN);
    assert!(chart.revision() > revision);
    let revision = chart.revision();
    brush
        .gradient_stops()
        .add(GradientStop::with_color_and_offset(Colors::WHITE, 0.5));
    assert!(chart.revision() > revision);
    series.set_fill_brush(None);
    let revision = chart.revision();
    stop.set_color(Colors::BLACK);
    assert_eq!(chart.revision(), revision);
}

#[test]
fn automation_uses_title_and_respects_native_explicit_name() {
    use ferroui_controls::automation::AutomationProperties;
    let _scope = Dispatcher::unit_test_scope();
    let chart = CartesianChart::new();
    chart.set_title("Weekly traffic".into());
    let series = line();
    series.set_name("Visits".into());
    chart.series().add(series.upcast());
    let peer = ChartAutomationPeer::new(&chart);
    assert_eq!(peer.get_name(), "Weekly traffic");
    assert!(peer.get_help_text().contains("Visits: 3, 8, 5"));
    AutomationProperties::set_name(&chart, Some("Accessible chart".into()));
    assert_eq!(peer.get_name(), "Accessible chart");
}

#[test]
fn runtime_xaml_loads_native_chart_series_points_and_axes() {
    let _app = UnitTestApplication::start(TestServices::new());
    register_types();
    let loaded = FerroRuntimeXamlLoader::parse(
        r##"<lc:CartesianChart xmlns="https://github.com/ferroui"
            xmlns:lc="https://ferroui.dev/livecharts" Title="Native markup" ZoomMode="Both">
          <lc:CartesianChart.XAxes>
            <lc:Axis Name="Time" MinLimit="0" MaxLimit="10" />
          </lc:CartesianChart.XAxes>
          <lc:LineSeries Name="Temperature" Stroke="#2563eb">
            <lc:ChartPoint X="0" Y="4" />
            <lc:ChartPoint X="1" Y="9" />
          </lc:LineSeries>
        </lc:CartesianChart>"##,
        Some(&ASSEMBLY),
    )
    .unwrap_or_else(|error| panic!("{}; {:?}", error.message(), error.inner_exception()));
    let chart = from_markup_value::<Ref<CartesianChart>>(&Some(loaded)).unwrap();
    assert_eq!(chart.title(), "Native markup");
    assert_eq!(chart.zoom_mode(), ZoomAndPanMode::BOTH);
    assert_eq!(chart.series().count(), 1);
    let series = chart.series().get(0);
    assert_eq!(series.values().count(), 2);
    assert_eq!(series.values().get(1).y(), 9.0);
    assert_eq!(chart.x_axes().count(), 1);
    assert_eq!(chart.x_axes().get(0).name(), "Time");
    let revision = chart.revision();
    series.values().get(0).set_y(12.0);
    assert!(chart.revision() > revision);
}

#[test]
fn polar_chart_takes_the_source_defaults_and_loads_from_markup() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    let polar = PolarChart::new();
    assert_eq!(polar.chart_kind(), ChartKind::Polar);
    // LiveChartsSettings.PolarInitialRotation: zero points up. The pie chart keeps 0.
    assert_eq!(polar.initial_rotation(), -90.0);
    assert_eq!(PieChart::new().initial_rotation(), 0.0);
    assert_eq!(
        PolarLineSeries::new().data_padding(),
        ferroui_base::Point::new(1.0, 1.5)
    );
    let loaded = FerroRuntimeXamlLoader::parse(
        r##"<lc:PolarChart xmlns:lc="https://ferroui.dev/livecharts" InnerRadius="50">
          <lc:PolarChart.YAxes>
            <lc:Axis MaxLimit="20" LabelsAngle="-60" />
          </lc:PolarChart.YAxes>
          <lc:PolarLineSeries Name="Signal" IsClosed="False">
            <lc:ChartPoint X="0" Y="10" />
            <lc:ChartPoint X="1" Y="8" />
            <lc:ChartPoint X="2" Y="12" />
          </lc:PolarLineSeries>
        </lc:PolarChart>"##,
        Some(&ASSEMBLY),
    )
    .unwrap_or_else(|error| panic!("{}; {:?}", error.message(), error.inner_exception()));
    let chart = from_markup_value::<Ref<PolarChart>>(&Some(loaded)).unwrap();
    assert_eq!(chart.inner_radius(), 50.0);
    assert_eq!(chart.y_axes().get(0).labels_angle(), -60.0);
    let series = chart.series().get(0);
    assert_eq!(series.kind(), SeriesKind::PolarLine);
    assert!(!series.polar_closed());
    chart.set_animations_speed(0.0);
    let scene = chart.scene(Size::new(600.0, 600.0)).unwrap();
    assert_eq!(scene.hits.len(), 3);
    let scale = chart.polar_scale(0, 0).unwrap().unwrap();
    assert!((scale.radius.max - 20.0).abs() < 1e-9);
}

/// The A, B to X, Y graph of the Sankey hit tests: the nodes and the series that holds them.
fn sankey_fixture() -> (Ref<SankeySeries>, Vec<Ref<SankeyNode>>) {
    let nodes: Vec<_> = ["A", "B", "X", "Y"]
        .into_iter()
        .map(SankeyNode::with_name)
        .collect();
    let series = SankeySeries::new();
    series.set_nodes(SankeyNodes::from_items(nodes.iter().cloned()));
    series.set_links(SankeyLinks::from_items([
        SankeyLink::with_nodes(&nodes[0], &nodes[2], 4.0),
        SankeyLink::with_nodes(&nodes[0], &nodes[3], 2.0),
        SankeyLink::with_nodes(&nodes[1], &nodes[3], 6.0),
    ]));
    (series, nodes)
}

#[test]
fn sankey_chart_takes_the_source_defaults_and_observes_its_nodes_and_links() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    let chart = SankeyChart::new();
    assert_eq!(chart.chart_kind(), ChartKind::Sankey);
    chart.set_animations_speed(0.0);
    // The defaults of CoreSankeySeries.
    let defaults = SankeySeries::new();
    assert_eq!(defaults.kind(), SeriesKind::Sankey);
    assert_eq!(defaults.node_width(), 12.0);
    assert_eq!(defaults.node_padding(), 8.0);
    assert_eq!(defaults.layout_iterations(), 32);
    assert_eq!(defaults.arc_span_degrees(), 150.0);
    assert_eq!(defaults.node_label_position(), SankeyLabelPosition::Auto);
    assert_eq!(defaults.layout(), SankeyLayoutKind::Vertical);
    // Each series owns its collections.
    assert!(!defaults.nodes().ptr_eq(&SankeySeries::new().nodes()));
    let (series, nodes) = sankey_fixture();
    series.set_node_width(24.0);
    chart.series().add(series.clone().upcast());
    let size = Size::new(1000.0, 1000.0);
    let scene = chart.scene(size).unwrap();
    assert_eq!(scene.hits.len(), 4);
    let a = scene.hits.iter().find(|h| h.id.point == 0).unwrap();
    assert_eq!(a.label, "A: 6");
    assert_eq!(chart.find_points(a.center).len(), 1);
    // A node, a link and the collections are observed.
    let revision = chart.revision();
    nodes[0].set_name("Alpha".into());
    assert!(chart.revision() > revision);
    let revision = chart.revision();
    series.links().get(0).set_weight(10.0);
    assert!(chart.revision() > revision);
    let late = SankeyNode::with_name("Z");
    series.nodes().add(late.clone());
    series
        .links()
        .add(SankeyLink::with_nodes(&nodes[1], &late, 1.0));
    let revision = chart.revision();
    late.set_name("Zeta".into());
    assert!(chart.revision() > revision);
    let scene = chart.scene(size).unwrap();
    assert_eq!(scene.hits.len(), 5);
    assert!(scene.hits.iter().any(|h| h.label == "Alpha: 12"));
    // Replaced collections stop notifying.
    let old = series.nodes();
    series.set_nodes(SankeyNodes::new());
    let revision = chart.revision();
    old.add(SankeyNode::with_name("detached"));
    nodes[0].set_name("detached".into());
    assert_eq!(chart.revision(), revision);
    assert!(chart.scene(size).unwrap().hits.is_empty());
    // The bipartite arc layout, and a series of another kind, which the chart rejects.
    let (arc, _) = sankey_fixture();
    arc.set_layout(SankeyLayoutKind::BipartiteArc);
    chart.series().set(0, arc.upcast());
    assert!(matches!(
        chart.scene(size).unwrap().hits[0].shape,
        ferroui_livecharts::HitShape::AnnularSector { .. }
    ));
    chart.series().add(line().upcast());
    assert!(chart.scene(size).is_err());
}

#[test]
fn treemap_chart_loads_nested_nodes_from_markup() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    assert_eq!(TreemapSeries::new().padding(), 2.0);
    assert_eq!(TreemapSeries::new().kind(), SeriesKind::Treemap);
    let loaded = FerroRuntimeXamlLoader::parse(
        r##"<lc:TreemapChart xmlns:lc="https://ferroui.dev/livecharts">
          <lc:TreemapSeries Name="Population" Padding="4" Fill="#FF608ADA" Stroke="#FFFFFFFF">
            <lc:TreemapSeries.Nodes>
              <lc:TreemapNode Name="Asia">
                <lc:TreemapNode.Children>
                  <lc:TreemapNodes>
                    <lc:TreemapNode Name="China" Value="1400" />
                    <lc:TreemapNode Name="India" Value="1300" />
                    <lc:TreemapNode Name="Japan" Value="125" />
                    <lc:TreemapNode Name="Indonesia" Value="270" />
                  </lc:TreemapNodes>
                </lc:TreemapNode.Children>
              </lc:TreemapNode>
              <lc:TreemapNode Name="Europe">
                <lc:TreemapNode.Children>
                  <lc:TreemapNodes>
                    <lc:TreemapNode Name="Germany" Value="83" />
                    <lc:TreemapNode Name="France" Value="67" />
                    <lc:TreemapNode Name="Italy" Value="60" />
                    <lc:TreemapNode Name="Spain" Value="46" />
                  </lc:TreemapNodes>
                </lc:TreemapNode.Children>
              </lc:TreemapNode>
            </lc:TreemapSeries.Nodes>
          </lc:TreemapSeries>
        </lc:TreemapChart>"##,
        Some(&ASSEMBLY),
    )
    .unwrap_or_else(|error| panic!("{}; {:?}", error.message(), error.inner_exception()));
    let chart = from_markup_value::<Ref<TreemapChart>>(&Some(loaded)).unwrap();
    assert_eq!(chart.chart_kind(), ChartKind::Treemap);
    chart.set_animations_speed(0.0);
    let series = chart.series().get(0).cast::<TreemapSeries>().unwrap();
    assert_eq!(series.padding(), 4.0);
    let asia = series.nodes().get(0);
    assert_eq!(asia.children().map(|c| c.count()), Some(4));
    assert!(asia.children().unwrap().get(0).children().is_none());
    let scene = chart.scene(Size::new(600.0, 600.0)).unwrap();
    // SnapshotTests/TreemapSeriesTests.HoverTooltip: China is the top-left leaf.
    let hits = chart.find_points(ferroui_base::Point::new(150.0, 150.0));
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].label, "China: 1400");
    assert_eq!(scene.hits.len(), 8);
    // A leaf value and a new child are observed through the tree.
    let revision = chart.revision();
    asia.children().unwrap().get(0).set_numeric_value(10.0);
    assert!(chart.revision() > revision);
    let revision = chart.revision();
    let europe = series.nodes().get(1);
    europe
        .children()
        .unwrap()
        .add(TreemapNode::with_value(10.0, "Portugal"));
    assert!(chart.revision() > revision);
    assert_eq!(chart.scene(Size::new(600.0, 600.0)).unwrap().hits.len(), 9);
}

/// Weak handles of the given objects: each reports whether its object is still alive.
fn alive<T: ferroui_base::ObjectType>(objects: &[Ref<T>]) -> Vec<Box<dyn Fn() -> bool>> {
    objects
        .iter()
        .map(|object| {
            let weak = object.downgrade();
            Box::new(move || weak.upgrade().is_some()) as Box<dyn Fn() -> bool>
        })
        .collect()
}

/// Port of _MemoryTests.SankeySeries_Releases_Vertical and SankeySeries_Releases_BipartiteArc:
/// after a draw, an emptied series and a second draw, no node or link survives.
#[test]
fn sankey_series_release_their_nodes_and_links() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    const COUNT: usize = 20;
    for layout in [SankeyLayoutKind::Vertical, SankeyLayoutKind::BipartiteArc] {
        let chart = SankeyChart::new();
        chart.set_animations_speed(0.0);
        let series = SankeySeries::new();
        series.set_layout(layout);
        chart.series().add(series.clone().upcast());
        let size = Size::new(300.0, 200.0);
        let survivors = {
            let nodes: Vec<_> = (0..COUNT)
                .map(|i| SankeyNode::with_name(format!("N{i}")))
                .collect();
            let links: Vec<_> = (0..COUNT / 2)
                .map(|i| SankeyLink::with_nodes(&nodes[i], &nodes[COUNT / 2 + i], (i + 1) as f64))
                .collect();
            let mut survivors = alive(&nodes);
            survivors.extend(alive(&links));
            series.set_nodes(SankeyNodes::from_items(nodes));
            series.set_links(SankeyLinks::from_items(links));
            assert_eq!(chart.scene(size).unwrap().hits.len(), COUNT);
            series.set_nodes(SankeyNodes::new());
            series.set_links(SankeyLinks::new());
            assert!(chart.scene(size).unwrap().hits.is_empty());
            survivors
        };
        assert!(survivors.iter().all(|alive| !alive()), "{layout:?}");
    }
}

/// Port of _MemoryTests.TreemapSeries_Releases and TreemapSeries_Releases_Hierarchical.
#[test]
fn treemap_series_release_their_nodes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    const COUNT: usize = 20;
    for hierarchical in [false, true] {
        let chart = TreemapChart::new();
        chart.set_animations_speed(0.0);
        let series = TreemapSeries::new();
        chart.series().add(series.clone().upcast());
        let size = Size::new(300.0, 200.0);
        let survivors = {
            let mut nodes: Vec<_> = (0..COUNT)
                .map(|i| TreemapNode::with_value((i + 1) as f64, format!("N{i}")))
                .collect();
            let roots = if hierarchical {
                let root =
                    TreemapNode::with_children("Root", TreemapNodes::from_items(nodes.clone()));
                nodes.push(root.clone());
                vec![root]
            } else {
                nodes.clone()
            };
            let survivors = alive(&nodes);
            series.set_nodes(TreemapNodes::from_items(roots));
            assert_eq!(chart.scene(size).unwrap().hits.len(), COUNT);
            series.set_nodes(TreemapNodes::new());
            assert!(chart.scene(size).unwrap().hits.is_empty());
            survivors
        };
        assert!(survivors.iter().all(|alive| !alive()), "{hierarchical}");
    }
}

fn heat_lands(series: &HeatLandSeries, lands: &[(&str, f64)]) {
    series.set_lands(HeatLands::from_items(
        lands
            .iter()
            .map(|&(name, value)| HeatLand::with_value(name, value)),
    ));
}

#[test]
fn geo_map_takes_the_source_defaults_and_loads_from_markup() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    use ferroui_base::media::Color;
    // The defaults of SourceGenMapChart and HeatLandSeries.
    let map = GeoMap::new();
    assert_eq!(map.chart_kind(), ChartKind::GeoMap);
    assert_eq!(map.map_projection(), MapProjection::Default);
    assert_eq!(map.stroke(), Some(Color::from_rgb(255, 255, 255)));
    assert_eq!(map.fill(), Some(Color::from_rgb(240, 240, 240)));
    assert_eq!(map.interaction_mode(), MapInteractionMode::NONE);
    assert_eq!((map.min_zoom_level(), map.max_zoom_level()), (1.0, 100.0));
    assert_eq!(map.zooming_speed(), 0.2);
    assert!(map.active_map().is_none());
    assert_eq!((map.zoom_level(), map.pan_offset()), (1.0, ferroui_base::Point::new(0.0, 0.0)));
    let series = HeatLandSeries::new();
    assert_eq!(series.kind(), SeriesKind::HeatLand);
    assert_eq!(
        series.heat_map(),
        vec![Color::from_rgb(179, 229, 252), Color::from_rgb(2, 136, 209)]
    );
    assert!(!series.lands().ptr_eq(&HeatLandSeries::new().lands()));
    assert!(!map.markers().ptr_eq(&GeoMap::new().markers()));
    let loaded = FerroRuntimeXamlLoader::parse(
        r##"<lc:GeoMap xmlns:lc="https://ferroui.dev/livecharts" MapProjection="Mercator"
              InteractionMode="Both">
          <lc:HeatLandSeries Name="Population">
            <lc:HeatLand Name="bra" Value="13" />
            <lc:HeatLand Name="usa" Value="15" />
          </lc:HeatLandSeries>
          <lc:GeoMap.Markers>
            <lc:GeoMarker Longitude="-74" Latitude="40.71" Size="10" />
          </lc:GeoMap.Markers>
        </lc:GeoMap>"##,
        Some(&ASSEMBLY),
    )
    .unwrap_or_else(|error| panic!("{}; {:?}", error.message(), error.inner_exception()));
    let chart = from_markup_value::<Ref<GeoMap>>(&Some(loaded)).unwrap();
    chart.set_animations_speed(0.0);
    assert_eq!(chart.map_projection(), MapProjection::Mercator);
    assert_eq!(chart.interaction_mode(), MapInteractionMode::BOTH);
    let series = chart.series().get(0).cast::<HeatLandSeries>().unwrap();
    assert_eq!(series.lands().count(), 2);
    assert_eq!(chart.markers().get(0).size(), 10.0);
    let scene = chart.scene(Size::new(800.0, 600.0)).unwrap();
    assert!(scene.hits.len() > 100);
    // GeoMap_GetPointsAt_ReturnsHitLand through the chart: the land under Brasilia, its
    // values and the default tooltip text.
    let brasilia = chart.project(-47.92, -15.79).unwrap().unwrap();
    let land = chart.find_land_at(brasilia).unwrap();
    assert_eq!(land.short_name, "bra");
    assert_eq!(land.values.len(), 1);
    assert_eq!(land.values[0].value, 13.0);
    let hits = chart.find_points(brasilia);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].label, "Brazil\nPopulation: 13.00");
    assert_eq!(chart.map_land(hits[0].id), Some(land));
    let (longitude, latitude) = chart.unproject(brasilia).unwrap().unwrap();
    assert!((longitude + 47.92).abs() < 1e-6 && (latitude + 15.79).abs() < 1e-6);
}

/// GeoMapTests.GeoMap_SwappingSeries_PreservesFillOnSharedLands (issue 962) on the chart, and
/// the observation of the lands, the series and the markers.
#[test]
fn geo_map_observes_its_lands_series_and_markers() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = GeoMap::new();
    chart.set_animations_speed(0.0);
    let first = HeatLandSeries::new();
    heat_lands(&first, &[("fra", 10.0), ("usa", 5.0)]);
    chart.series().add(first.clone().upcast());
    let size = Size::new(400.0, 400.0);
    let fill_of = |name: &str| {
        let scene = chart.scene(size).unwrap();
        let land = scene
            .hits
            .iter()
            .find(|hit| hit.label.to_lowercase().starts_with(name))
            .unwrap_or_else(|| panic!("{name}"))
            .id;
        scene
            .primitives
            .iter()
            .find(|p| p.id == Some(land))
            .and_then(|p| p.style.fill)
    };
    let high = Some(ferroui_base::media::Color::from_rgb(2, 136, 209));
    let low = Some(ferroui_base::media::Color::from_rgb(179, 229, 252));
    assert_eq!(fill_of("france"), high);
    assert_eq!(fill_of("united states"), low);
    // A land value, a new land and a replaced series are observed.
    let revision = chart.revision();
    first.lands().get(0).set_numeric_value(1.0);
    assert!(chart.revision() > revision);
    assert_eq!(fill_of("france"), low);
    let revision = chart.revision();
    first.lands().add(HeatLand::with_value("deu", 3.0));
    assert!(chart.revision() > revision);
    let second = HeatLandSeries::new();
    heat_lands(&second, &[("fra", 1.0), ("bra", 99.0)]);
    chart.series().set(0, second.upcast());
    assert_eq!(fill_of("france"), low);
    assert_eq!(
        fill_of("united states"),
        Some(ferroui_base::media::Color::from_rgb(240, 240, 240))
    );
    // The old series no longer notifies.
    let revision = chart.revision();
    first.lands().get(0).set_numeric_value(50.0);
    assert_eq!(chart.revision(), revision);
    // Markers, and a marker's coordinate.
    let marker = GeoMarker::at(2.35, 48.85);
    chart.markers().add(marker.clone());
    let revision = chart.revision();
    marker.set_longitude(13.4);
    assert!(chart.revision() > revision);
    // ChartTests.MapShouldHandleNullParams: a map without series draws the map.
    chart.series().clear();
    assert!(!chart.scene(size).unwrap().hits.is_empty());
    // A series of another kind is rejected.
    chart.series().add(line().upcast());
    assert!(chart.scene(size).is_err());
}

/// GeoMapTests.GeoMap_TitleSet_ReducesDrawMarginAndAddsVisualToCanvas and
/// GeoMap_HeatLegendRight_ReducesDrawMarginWidth: a title moves the map down and a legend on the
/// right narrows it.
#[test]
fn geo_map_title_and_legend_take_room_from_the_map() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = GeoMap::new();
    chart.set_animations_speed(0.0);
    let series = HeatLandSeries::new();
    heat_lands(&series, &[("rus", 1.0), ("usa", 9.0)]);
    chart.series().add(series.upcast());
    let size = Size::new(600.0, 600.0);
    let bare = chart.scene(size).unwrap().plot_bounds;
    chart.set_title("Hello".into());
    let titled = chart.scene(size).unwrap();
    assert!(titled.plot_bounds.y > bare.y);
    assert!(titled.plot_bounds.height < 600.0);
    assert!(titled.primitives.iter().any(|p| matches!(
        &p.geometry,
        ferroui_livecharts::Geometry::Text { text, .. } if text == "Hello"
    )));
    chart.set_title(String::new());
    chart.set_legend_position(LegendPosition::Right);
    let size = Size::new(800.0, 600.0);
    let scene = chart.scene(size).unwrap();
    assert!(scene.plot_bounds.x + scene.plot_bounds.width < 800.0);
    assert_eq!(scene.legend.len(), 1);
}

/// ChartInteractiveApiTests.GeoMap_PanAndZoomAndResetViewportBeforeRender and
/// GeoMap_ViewToAndRotateToBeforeRender, and GeoMapTests.GeoMap_ProjectUnproject_RoundTrips
/// and the orthographic cases through the chart.
#[test]
fn geo_map_viewport_and_rotation_before_render() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = GeoMap::new();
    chart.set_animations_speed(0.0);
    chart.set_map_projection(MapProjection::Mercator);
    let series = HeatLandSeries::new();
    heat_lands(&series, &[("bra", 1.0)]);
    chart.series().add(series.upcast());
    let size = Size::new(400.0, 400.0);
    chart.pan_map(ferroui_base::Point::new(10.0, 20.0));
    assert_eq!(chart.pan_offset(), ferroui_base::Point::new(10.0, 20.0));
    let pivot = ferroui_base::Point::new(100.0, 100.0);
    chart.zoom_map(pivot, ZoomDirection::ZoomIn);
    assert!((chart.zoom_level() - 1.25).abs() < 1e-12);
    chart.zoom_map(pivot, ZoomDirection::ZoomOut);
    assert!((chart.zoom_level() - 1.0).abs() < 1e-12);
    chart.reset_viewport();
    assert_eq!(chart.map_viewport(), MapViewport::default());
    assert!(!chart.scene(size).unwrap().hits.is_empty());
    // A zoomed map keeps the coordinate under the pivot in place.
    chart.scene(size).unwrap();
    let under = chart.unproject(pivot).unwrap().unwrap();
    chart.zoom_map(pivot, ZoomDirection::ZoomIn);
    let moved = chart.project(under.0, under.1).unwrap().unwrap();
    assert!((moved.x - pivot.x).abs() < 1e-6 && (moved.y - pivot.y).abs() < 1e-6);
    chart.reset_zoom();
    assert_eq!(chart.zoom_level(), 1.0);
    // A globe turns the short way round, at once outside a visual tree.
    chart.set_map_projection(MapProjection::Orthographic);
    chart.set_center_longitude(170.0);
    chart.rotate_to(-170.0, 20.0, 0.0);
    assert_eq!((chart.center_longitude(), chart.center_latitude()), (190.0, 20.0));
    chart.rotate_to(10.0, 20.0, 800.0);
    assert_eq!(chart.center_longitude(), 10.0);
    assert!(chart.project(180.0 + 10.0, 0.0).unwrap().is_none());
    assert!(chart.unproject(ferroui_base::Point::new(0.0, 0.0)).unwrap().is_none());
    assert!(chart.project(10.0, 20.0).unwrap().is_some());
}

/// The active map: a custom GeoJSON map in place of the world map, and its lands keyed by their
/// short names (MapsTests.CustomMap_LoadsAndRendersMexicoStates with an inline map).
#[test]
fn geo_map_draws_a_custom_map() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let map = Maps::map_from_geojson(
        r#"{"type":"FeatureCollection","features":[
            {"type":"Feature","properties":{"name":"North","shortName":"n"},
             "geometry":{"type":"Polygon","coordinates":[[[0,10],[10,10],[10,20],[0,20],[0,10]]]}},
            {"type":"Feature","properties":{"name":"South","shortName":"s"},
             "geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]]]}}]}"#,
    )
    .unwrap();
    let chart = GeoMap::new();
    chart.set_animations_speed(0.0);
    chart.set_active_map(Some(std::rc::Rc::new(map)));
    chart.set_min_latitude(Some(-5.0));
    chart.set_max_latitude(Some(25.0));
    chart.set_min_longitude(Some(-5.0));
    chart.set_max_longitude(Some(15.0));
    let series = HeatLandSeries::new();
    heat_lands(&series, &[("n", 92.0), ("s", 23.0)]);
    chart.series().add(series.upcast());
    let scene = chart.scene(Size::new(600.0, 600.0)).unwrap();
    assert_eq!(scene.hits.len(), 2);
    let north = chart.project(5.0, 15.0).unwrap().unwrap();
    assert_eq!(chart.find_land_at(north).unwrap().short_name, "n");
    assert!(chart.find_land_at(chart.project(-3.0, 23.0).unwrap().unwrap()).is_none());
}
