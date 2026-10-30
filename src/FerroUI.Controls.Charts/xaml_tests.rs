//! Public Charts metadata and XAML scenarios adapted to native FerroUI metadata.

use crate::*;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::data::BindingPriority;
use ferroui_base::metadata::{from_markup_value, MarkupAssembly, MarkupType, MarkupTyped};
use ferroui_base::{BoxedValue, FerroObject, PropertyValue, Ref, Size, TypeInfo};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, ItemsSource, Window};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::any::TypeId;
use std::rc::Rc;

fn boxed<T: PropertyValue>(value: T) -> Option<BoxedValue> {
    Some(Rc::new(value))
}

#[test]
fn xaml_metadata_unknown_types_return_null() {
    register_types();
    assert!(TypeInfo::find("FerroUI.Controls.Charts", "UnknownChart").is_none());
    assert!(MarkupType::find("FerroUI.Controls.Charts", "UnknownChart").is_none());
}

#[test]
fn xaml_metadata_samples_activates_and_items_source_round_trips() {
    register_types();
    let samples = Samples::TYPE
        .create_instance()
        .unwrap()
        .cast::<Samples>()
        .unwrap();
    let source = boxed(ItemsSource::from_values([1.0, 2.0, 3.0]));
    let property = Samples::TYPE.find_property("ItemsSource").unwrap();
    samples.set_value_untyped(property, &source, BindingPriority::LocalValue);
    assert_eq!(samples.items_source(), source);
    samples.clear_value_untyped(property);
    assert!(samples.items_source().is_none());
    assert_eq!(Samples::TYPE.content_property(), Some("ItemsSource"));
}

#[test]
fn xaml_metadata_chart_activates_and_adds_data_axes_and_series() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    let chart = Chart::TYPE
        .create_instance()
        .unwrap()
        .cast::<Chart>()
        .unwrap();
    let metadata = Chart::TYPE.markup().unwrap();
    let data =
        (metadata.find_property("Data").unwrap().get.unwrap())(&[boxed(chart.clone())]).unwrap();
    let axes =
        (metadata.find_property("Axes").unwrap().get.unwrap())(&[boxed(chart.clone())]).unwrap();
    let series =
        (metadata.find_property("Series").unwrap().get.unwrap())(&[boxed(chart.clone())]).unwrap();
    let source = Samples::from_values([2.0, 3.0]);
    let axis = LinearAxis::new();
    let line = LineSeries::new();
    line.set_y_values(Some(source.clone()));
    (<ChartDataCollection as MarkupTyped>::MARKUP
        .find_methods("Add")
        .next()
        .unwrap()
        .invoke)(&[data, boxed(source)])
    .unwrap();
    (<ChartAxisCollection as MarkupTyped>::MARKUP
        .find_methods("Add")
        .next()
        .unwrap()
        .invoke)(&[axes, boxed(axis)])
    .unwrap();
    (<ChartSeriesCollection as MarkupTyped>::MARKUP
        .find_methods("Add")
        .next()
        .unwrap()
        .invoke)(&[series, boxed(line)])
    .unwrap();
    assert_eq!(
        (
            chart.data().count(),
            chart.axes().count(),
            chart.series().count()
        ),
        (1, 1, 1)
    );
}

#[test]
fn xaml_metadata_chart_legend_dependency_properties_round_trip() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    let chart = Chart::new();
    let legend = Chart::TYPE.find_property("ShowLegend").unwrap();
    let title = Chart::TYPE.find_property("LegendTitle").unwrap();
    assert_eq!(
        chart.get_value_untyped(legend).downcast_ref::<bool>(),
        Some(&false)
    );
    assert_eq!(
        chart
            .get_value_untyped(title)
            .downcast_ref::<String>()
            .unwrap(),
        ""
    );
    chart.set_value_untyped(legend, &true, BindingPriority::LocalValue);
    chart.set_value_untyped(title, &"Revenue".to_owned(), BindingPriority::LocalValue);
    assert!(chart.show_legend());
    assert_eq!(chart.legend_title(), "Revenue");
}

#[test]
fn xaml_metadata_cartesian_series_members_are_on_base_type() {
    register_types();
    assert!(CartesianSeries::TYPE.default_constructor().is_none());
    assert!(CartesianSeries::TYPE.create_instance().is_none());
    for name in [
        "Title",
        "IsVisible",
        "XValues",
        "YValues",
        "Stroke",
        "StrokeThickness",
        "StrokeDashStyle",
        "ShowDataLabels",
        "ShowDataMarkers",
        "MarkerShape",
        "DataLabelBrush",
        "DataMarkerBrush",
    ] {
        assert!(
            CartesianSeries::TYPE.find_property(name).is_some(),
            "{name}"
        );
    }
    // Native direct properties supply notification and validation for the two
    // plain properties of the WinRT surface; the difference is documented.
    for name in ["XAxis", "YAxis"] {
        assert!(CartesianSeries::TYPE
            .find_property(name)
            .unwrap()
            .is_direct());
    }
    for name in ["DataLabelOverrides", "DataMarkerOverrides"] {
        assert!(CartesianSeries::TYPE.find_property(name).is_none());
        assert!(CartesianSeries::TYPE
            .markup()
            .is_none_or(|m| m.find_property(name).is_none()));
    }
}

#[test]
fn xaml_metadata_line_series_inherited_members_round_trip() {
    register_types();
    let line = LineSeries::TYPE
        .create_instance()
        .unwrap()
        .cast::<LineSeries>()
        .unwrap();
    assert_eq!(LineSeries::TYPE.base_type(), Some(CartesianSeries::TYPE));
    for (name, value) in [("Title", "Revenue".to_owned())] {
        line.set_value_untyped(
            CartesianSeries::TYPE.find_property(name).unwrap(),
            &value,
            BindingPriority::LocalValue,
        );
    }
    line.set_value_untyped(
        CartesianSeries::TYPE.find_property("IsVisible").unwrap(),
        &false,
        BindingPriority::LocalValue,
    );
    line.set_value_untyped(
        CartesianSeries::TYPE.find_property("MarkerShape").unwrap(),
        &MarkerShape::Diamond,
        BindingPriority::LocalValue,
    );
    line.set_value_untyped(
        CartesianSeries::TYPE
            .find_property("StrokeDashStyle")
            .unwrap(),
        &StrokeDashStyle::DashDot,
        BindingPriority::LocalValue,
    );
    assert_eq!(line.title(), "Revenue");
    assert!(!line.is_visible());
    assert_eq!(line.marker_shape(), MarkerShape::Diamond);
    assert_eq!(line.stroke_dash_style(), StrokeDashStyle::DashDot);
}

#[test]
fn xaml_metadata_area_and_bar_series_activate_and_expose_fill() {
    register_types();
    for type_ in [AreaSeries::TYPE, BarSeries::TYPE] {
        let instance = type_.create_instance().unwrap();
        assert_eq!(type_.base_type(), Some(CartesianSeries::TYPE));
        let fill = type_.find_property("Fill").unwrap();
        assert_eq!(fill.owner_type(), type_);
        let brush: Option<Rc<dyn ferroui_base::media::IBrush>> =
            Some(ferroui_base::media::Brushes::green());
        instance.set_value_untyped(fill, &brush, BindingPriority::LocalValue);
        assert!(instance
            .get_value_untyped(fill)
            .downcast_ref::<Option<Rc<dyn ferroui_base::media::IBrush>>>()
            .unwrap()
            .is_some());
    }
}

#[test]
fn xaml_metadata_enum_types_create_from_string() {
    register_types();
    for (name, value) in [
        ("BarOrientation", "Vertical"),
        ("MarkerShape", "Circle"),
        ("StrokeDashStyle", "Dash"),
        ("GridLines", "Major"),
        ("SortOrder", "Descending"),
        ("DateTimeIntervalType", "Month"),
        ("CategorySortKey", "Value"),
    ] {
        let metadata = MarkupType::find("FerroUI.Controls.Charts", name).unwrap();
        assert!(
            metadata.find_enum_member(value, false).is_some(),
            "{name}.{value}"
        );
        assert!(metadata
            .find_enum_member(&value.to_lowercase(), false)
            .is_none());
    }
    let circle = <MarkerShape as MarkupTyped>::MARKUP
        .find_enum_member("Circle", false)
        .unwrap();
    assert_eq!(
        (circle.get)().downcast_ref::<MarkerShape>(),
        Some(&MarkerShape::Circle)
    );
}

#[test]
fn xaml_metadata_reports_underlying_types() {
    register_types();
    for (type_, handle) in [
        (Samples::TYPE, TypeId::of::<Ref<Samples>>()),
        (Chart::TYPE, TypeId::of::<Ref<Chart>>()),
        (LinearAxis::TYPE, TypeId::of::<Ref<LinearAxis>>()),
        (CategoryAxis::TYPE, TypeId::of::<Ref<CategoryAxis>>()),
        (DateTimeAxis::TYPE, TypeId::of::<Ref<DateTimeAxis>>()),
        (LineSeries::TYPE, TypeId::of::<Ref<LineSeries>>()),
        (AreaSeries::TYPE, TypeId::of::<Ref<AreaSeries>>()),
        (BarSeries::TYPE, TypeId::of::<Ref<BarSeries>>()),
        (
            XamlChartsResources::TYPE,
            TypeId::of::<Ref<XamlChartsResources>>(),
        ),
    ] {
        assert_eq!(type_.handle(), Some(handle));
        assert_eq!(TypeInfo::find_by_handle(handle).map(|x| x.0), Some(type_));
    }
}

#[test]
fn xaml_metadata_base_type_chain_and_content_properties_are_reported() {
    register_types();
    for series in [LineSeries::TYPE, AreaSeries::TYPE, BarSeries::TYPE] {
        assert_eq!(series.base_type(), Some(CartesianSeries::TYPE));
        assert_eq!(
            series.base_type().unwrap().base_type(),
            Some(FerroObject::TYPE)
        );
    }
    // TemplatedControl supplies the native typography/background/foreground API
    // corresponding to WinUI Control; the additional native level is deliberate.
    assert_eq!(Chart::TYPE.base_type(), Some(TemplatedControl::TYPE));
    assert_eq!(
        XamlChartsResources::TYPE.base_type(),
        Some(ResourceDictionary::TYPE)
    );
    assert_eq!(Samples::TYPE.content_property(), Some("ItemsSource"));
    assert_eq!(Chart::TYPE.content_property(), Some("Series"));
    assert_eq!(CartesianSeries::TYPE.content_property(), None);
}

#[test]
fn xaml_metadata_type_overload_and_xmlns_definitions_work() {
    register_types();
    let type_ = TypeInfo::find_by_handle(TypeId::of::<Ref<Samples>>())
        .unwrap()
        .0;
    assert_eq!(type_.full_name(), "FerroUI.Controls.Charts.Samples");
    let assembly = MarkupAssembly::find("FerroUI.Controls.Charts").unwrap();
    assert!(assembly
        .xmlns_definitions
        .iter()
        .any(|d| d.xml_namespace == CHARTS_XML_NAMESPACE));
}

#[test]
fn xaml_charts_resources_metadata_activates_native_theme_dictionaries() {
    register_types();
    let resources = XamlChartsResources::TYPE
        .create_instance()
        .unwrap()
        .cast::<XamlChartsResources>()
        .unwrap();
    // The native resource dictionary is constructed directly; no Windows package
    // Source URI or runtime XAML file load is necessary.
    assert_eq!(
        XamlChartsResources::TYPE.full_name(),
        "FerroUI.Controls.Charts.XamlChartsResources"
    );
    assert!(resources.theme_dictionaries().count() >= 2);
}

#[test]
fn xaml_reader_loads_chart_markup_with_series_axes_samples_and_enums() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    let chart: Ref<Chart> = FerroRuntimeXamlLoader::parse_as(r#"
<charts:Chart xmlns="https://github.com/ferroui"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    xmlns:charts="https://github.com/ferroui/charts"
    xmlns:col="using:System.Collections" xmlns:sys="using:System"
    Width="400" Height="300" ShowLegend="True" LegendTitle="Revenue">
  <charts:Chart.Data><charts:Samples ItemsSource="chart-level" /></charts:Chart.Data>
  <charts:Chart.Axes>
    <charts:LinearAxis Label="Value" />
    <charts:CategoryAxis Label="Category" SortOrder="Descending" />
    <charts:DateTimeAxis Label="Date" IntervalType="Month" />
  </charts:Chart.Axes>
  <charts:LineSeries Title="Line" StrokeDashStyle="Dash" MarkerShape="Circle">
    <charts:LineSeries.XValues>
      <charts:Samples><col:ArrayList><sys:String>Jan</sys:String><sys:String>Feb</sys:String></col:ArrayList></charts:Samples>
    </charts:LineSeries.XValues>
    <charts:LineSeries.YValues>
      <charts:Samples><col:ArrayList><sys:Double>1</sys:Double><sys:Double>2</sys:Double></col:ArrayList></charts:Samples>
    </charts:LineSeries.YValues>
  </charts:LineSeries>
  <charts:AreaSeries Title="Area" Fill="Green" />
  <charts:BarSeries Title="Bar" Orientation="Vertical" />
</charts:Chart>"#, None).expect("Charts markup should load");
    assert!(chart.show_legend());
    assert_eq!(chart.legend_title(), "Revenue");
    assert_eq!(
        (
            chart.data().count(),
            chart.axes().count(),
            chart.series().count()
        ),
        (1, 3, 3)
    );
    assert_eq!(
        chart
            .data()
            .get(0)
            .unwrap()
            .items_source()
            .unwrap()
            .downcast_ref::<String>()
            .map(String::as_str),
        Some("chart-level")
    );
    assert_eq!(
        chart
            .axes()
            .get(1)
            .unwrap()
            .cast::<CategoryAxis>()
            .unwrap()
            .sort_order(),
        SortOrder::Descending
    );
    assert_eq!(
        chart
            .axes()
            .get(2)
            .unwrap()
            .cast::<DateTimeAxis>()
            .unwrap()
            .interval_type(),
        DateTimeIntervalType::Month
    );
    let line = chart.series().get(0).unwrap();
    assert_eq!(line.stroke_dash_style(), StrokeDashStyle::Dash);
    assert_eq!(line.marker_shape(), MarkerShape::Circle);
    assert_eq!(line.x_values().unwrap().count(), 2);
    assert_eq!(line.y_values().unwrap().count(), 2);
    // An object-typed source retains RuntimeList identity while its live
    // collection adapter observes additions after loading the document.
    let source = line.x_values().unwrap().items_source().unwrap();
    let source = source
        .downcast_ref::<ferroui_markup_xaml_loader::runtime::type_system::RuntimeList>()
        .unwrap();
    source.add(boxed("Mar".to_owned()));
    assert_eq!(line.x_values().unwrap().count(), 3);
    assert_eq!(
        chart
            .series()
            .get(2)
            .unwrap()
            .cast::<BarSeries>()
            .unwrap()
            .orientation(),
        BarOrientation::Vertical
    );
    let window = Window::new();
    window.set_content(Some(Control::boxed(chart.clone())));
    window.show();
    window.update_layout();
    assert_eq!(chart.bounds().size(), Size::new(400.0, 300.0));
    window.close();
}

#[test]
fn xaml_reader_loads_xaml_charts_resources_in_merged_dictionaries() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    register_types();
    let resources: Ref<ResourceDictionary> = FerroRuntimeXamlLoader::parse_as(r#"
<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:charts="https://github.com/ferroui/charts">
  <ResourceDictionary.MergedDictionaries><charts:XamlChartsResources /></ResourceDictionary.MergedDictionaries>
</ResourceDictionary>"#, None).expect("Chart resources should merge");
    assert_eq!(resources.merged_dictionaries().count(), 1);
}

#[test]
fn immutable_override_descriptors_have_readonly_native_metadata() {
    register_types();
    let label = <DataLabelOverride as MarkupTyped>::MARKUP;
    let value = (label.constructors[0].invoke)(&[boxed("Peak".to_owned()), None]).unwrap();
    assert_eq!(
        from_markup_value::<Rc<DataLabelOverride>>(&value)
            .unwrap()
            .text(),
        "Peak"
    );
    assert!(label.find_property("Text").unwrap().set.is_none());
    let marker = <DataMarkerOverride as MarkupTyped>::MARKUP;
    let value = (marker.constructors[0].invoke)(&[boxed(MarkerShape::Diamond), None]).unwrap();
    assert_eq!(
        from_markup_value::<Rc<DataMarkerOverride>>(&value)
            .unwrap()
            .shape(),
        MarkerShape::Diamond
    );
    assert!(marker.find_property("Shape").unwrap().set.is_none());
}
