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
    let polar = PolarLineSeries::new();
    assert_eq!(column.data_padding(), ferroui_base::Point::new(0.0, 1.0));
    assert_eq!(polar.data_padding(), ferroui_base::Point::new(1.0, 1.5));
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
fn runtime_xaml_loads_advanced_hierarchies_and_links() {
    let _app = UnitTestApplication::start(TestServices::new());
    register_types();
    let tree = FerroRuntimeXamlLoader::parse(
        r##"<lc:TreemapChart xmlns:lc="https://ferroui.dev/livecharts">
          <lc:TreemapChart.Roots>
            <lc:TreemapNode Name="Root">
              <lc:TreemapNode.Children>
                <lc:TreemapNodes><lc:TreemapNode Name="Leaf" Value="12" /></lc:TreemapNodes>
              </lc:TreemapNode.Children>
            </lc:TreemapNode>
          </lc:TreemapChart.Roots>
        </lc:TreemapChart>"##,
        Some(&ASSEMBLY),
    )
    .unwrap_or_else(|error| panic!("{}; {:?}", error.message(), error.inner_exception()));
    let tree = from_markup_value::<Ref<TreemapChart>>(&Some(tree)).unwrap();
    let root = tree.treemap_roots().get(0);
    assert_eq!(root.children().unwrap().get(0).value(), 12.0);
    let sankey = FerroRuntimeXamlLoader::parse(
        r##"<lc:SankeyChart xmlns:lc="https://ferroui.dev/livecharts">
          <lc:SankeyChart.Nodes>
            <lc:SankeyNode Id="1" Name="Input" /><lc:SankeyNode Id="2" Name="Output" />
          </lc:SankeyChart.Nodes>
          <lc:SankeyChart.Links>
            <lc:SankeyLink SourceId="1" TargetId="2" Weight="5" />
          </lc:SankeyChart.Links>
        </lc:SankeyChart>"##,
        Some(&ASSEMBLY),
    )
    .unwrap_or_else(|error| panic!("{}; {:?}", error.message(), error.inner_exception()));
    let sankey = from_markup_value::<Ref<SankeyChart>>(&Some(sankey)).unwrap();
    assert_eq!(sankey.sankey_nodes().count(), 2);
    assert_eq!(sankey.sankey_links().get(0).snapshot().weight, 5.0);
}
