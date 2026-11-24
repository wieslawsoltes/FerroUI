use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;

const NAMESPACES: &[(&str, &str)] = &[("ferroui_controls_livecharts", "FerroUI.LiveCharts")];
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.LiveCharts",
    crate_name: "ferroui_controls_livecharts",
    xmlns_definitions: &[
        XmlnsDefinition { xml_namespace: "https://ferroui.dev/livecharts", namespace: "FerroUI.LiveCharts" },
        XmlnsDefinition { xml_namespace: FERRO_XML_NAMESPACE, namespace: "FerroUI.LiveCharts" },
    ],
    xmlns_prefixes: &[],
    // The names of the six classes before they took the `Live` prefix (the native charts have the
    // plain names in the default XML namespace): markup with the old XML namespace keeps working.
    metadata: &[
        ("FerroXmlnsTypeAlias https://ferroui.dev/livecharts Chart", "LiveChart"),
        ("FerroXmlnsTypeAlias https://ferroui.dev/livecharts Axis", "LiveAxis"),
        ("FerroXmlnsTypeAlias https://ferroui.dev/livecharts AreaSeries", "LiveAreaSeries"),
        ("FerroXmlnsTypeAlias https://ferroui.dev/livecharts LineSeries", "LiveLineSeries"),
        ("FerroXmlnsTypeAlias https://ferroui.dev/livecharts DateTimeAxis", "LiveDateTimeAxis"),
        ("FerroXmlnsTypeAlias https://ferroui.dev/livecharts ChartSeriesCollection", "LiveChartSeriesCollection"),
    ],
};
const TYPES: &[&TypeInfo] = &[
    crate::LiveChart::TYPE,
    crate::ChartPoint::TYPE,
    crate::ChartSeries::TYPE,
    crate::LiveAxis::TYPE,
    crate::LiveDateTimeAxis::TYPE,
    crate::TimeSpanAxis::TYPE,
    crate::CartesianChart::TYPE,
    crate::PieChart::TYPE,
    crate::PolarChart::TYPE,
    crate::SankeyChart::TYPE,
    crate::TreemapChart::TYPE,
    crate::GeoMap::TYPE,
    crate::LiveLineSeries::TYPE,
    crate::StepLineSeries::TYPE,
    crate::LiveAreaSeries::TYPE,
    crate::StackedAreaSeries::TYPE,
    crate::StackedStepAreaSeries::TYPE,
    crate::ColumnSeries::TYPE,
    crate::RowSeries::TYPE,
    crate::StackedColumnSeries::TYPE,
    crate::StackedRowSeries::TYPE,
    crate::ScatterSeries::TYPE,
    crate::HeatSeries::TYPE,
    crate::RangeColumnSeries::TYPE,
    crate::RangeRowSeries::TYPE,
    crate::RangeLineSeries::TYPE,
    crate::CandlesticksSeries::TYPE,
    crate::OhlcSeries::TYPE,
    crate::BoxSeries::TYPE,
    crate::PieSeries::TYPE,
    crate::PolarLineSeries::TYPE,
    crate::SankeySeries::TYPE,
    crate::TreemapSeries::TYPE,
    crate::SankeyNode::TYPE,
    crate::SankeyLink::TYPE,
    crate::TreemapNode::TYPE,
    crate::HeatLandSeries::TYPE,
    crate::HeatLand::TYPE,
    crate::GeoMarker::TYPE,
    crate::ChartVisualElement::TYPE,
    crate::AngularTicksVisual::TYPE,
    crate::NeedleVisual::TYPE,
    crate::ChartLegend::TYPE,
    crate::HeatLegend::TYPE,
    crate::ChartAutomationPeer::TYPE,
];
ferroui_base::ferro_rust_paths! {
    classes:[crate::LiveChart,crate::ChartPoint,crate::ChartSeries,crate::LiveAxis,crate::LiveDateTimeAxis,crate::TimeSpanAxis,
        crate::CartesianChart,crate::PieChart,crate::PolarChart,crate::SankeyChart,crate::TreemapChart,crate::GeoMap,
        crate::LiveLineSeries,crate::StepLineSeries,crate::LiveAreaSeries,crate::StackedAreaSeries,crate::StackedStepAreaSeries,
        crate::ColumnSeries,crate::RowSeries,crate::StackedColumnSeries,crate::StackedRowSeries,crate::ScatterSeries,
        crate::HeatSeries,crate::RangeColumnSeries,crate::RangeRowSeries,crate::RangeLineSeries,
        crate::CandlesticksSeries,crate::OhlcSeries,crate::BoxSeries,crate::PieSeries,crate::PolarLineSeries,
        crate::SankeySeries,crate::TreemapSeries,crate::SankeyNode,crate::SankeyLink,crate::TreemapNode,
        crate::HeatLandSeries,crate::HeatLand,crate::GeoMarker,
        crate::ChartVisualElement,crate::AngularTicksVisual,crate::NeedleVisual,
        crate::ChartLegend,crate::HeatLegend,
        crate::ChartAutomationPeer],
    types:[crate::ChartKind,crate::LegendPosition,crate::TooltipPosition,crate::ZoomAndPanMode,
        crate::MapInteractionMode,
        crate::ChartPoints,crate::LiveChartSeriesCollection,crate::Axes,
        crate::SankeyNodes,crate::SankeyLinks,crate::TreemapNodes,crate::HeatLands,crate::GeoMarkers,
        crate::ChartVisualElements],
    contracts:[],generics:[],generic_contracts:[],
}

/// Registers native classes, collections, enums and their runtime/compiled markup metadata.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        ferroui_livecharts::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_table(TYPES);
        MarkupAssembly::register_table(&ASSEMBLY);
        MarkupType::register_table(&[
            <crate::ChartKind as MarkupTyped>::MARKUP,
            <crate::LegendPosition as MarkupTyped>::MARKUP,
            <crate::TooltipPosition as MarkupTyped>::MARKUP,
            <crate::ZoomAndPanMode as MarkupTyped>::MARKUP,
            <crate::MapInteractionMode as MarkupTyped>::MARKUP,
            <crate::ChartPoints as MarkupTyped>::MARKUP,
            <crate::LiveChartSeriesCollection as MarkupTyped>::MARKUP,
            <crate::Axes as MarkupTyped>::MARKUP,
            <crate::SankeyNodes as MarkupTyped>::MARKUP,
            <crate::SankeyLinks as MarkupTyped>::MARKUP,
            <crate::TreemapNodes as MarkupTyped>::MARKUP,
            <crate::HeatLands as MarkupTyped>::MARKUP,
            <crate::GeoMarkers as MarkupTyped>::MARKUP,
            <crate::ChartVisualElements as MarkupTyped>::MARKUP,
        ]);
        ValueTypes::register_global(register_value_types);
        register_rust_paths();
    });
}

/// What the untyped value conversions must know: the children of a treemap node, the ends of a
/// Sankey link and the legend of a chart are nullable, so a collection or an object converts to
/// them (a null child list is a leaf, a null legend the legend of the series).
fn register_value_types() {
    ValueTypes::register_nullable::<crate::TreemapNodes>();
    ValueTypes::register_object::<crate::SankeyNode>();
    ValueTypes::register_object::<crate::ChartLegend>();
}
