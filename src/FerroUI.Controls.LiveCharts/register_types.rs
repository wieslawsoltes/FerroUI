use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition};
use ferroui_base::TypeInfo;

const NAMESPACES: &[(&str, &str)] = &[("ferroui_controls_livecharts", "FerroUI.LiveCharts")];
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.LiveCharts",
    crate_name: "ferroui_controls_livecharts",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: "https://ferroui.dev/livecharts",
        namespace: "FerroUI.LiveCharts",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
const TYPES: &[&TypeInfo] = &[
    crate::Chart::TYPE,
    crate::ChartPoint::TYPE,
    crate::ChartSeries::TYPE,
    crate::Axis::TYPE,
    crate::CartesianChart::TYPE,
    crate::PieChart::TYPE,
    crate::PolarChart::TYPE,
    crate::SankeyChart::TYPE,
    crate::TreemapChart::TYPE,
    crate::GeoMap::TYPE,
    crate::LineSeries::TYPE,
    crate::StepLineSeries::TYPE,
    crate::AreaSeries::TYPE,
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
    crate::BoxSeries::TYPE,
    crate::PieSeries::TYPE,
    crate::PolarLineSeries::TYPE,
    crate::ChartAutomationPeer::TYPE,
    crate::SankeyNode::TYPE,
    crate::SankeyLink::TYPE,
    crate::TreemapNode::TYPE,
    crate::HeatLand::TYPE,
    crate::GeoMarker::TYPE,
];
ferroui_base::ferro_rust_paths! {
    classes:[crate::Chart,crate::ChartPoint,crate::ChartSeries,crate::Axis,
        crate::CartesianChart,crate::PieChart,crate::PolarChart,crate::SankeyChart,crate::TreemapChart,crate::GeoMap,
        crate::LineSeries,crate::StepLineSeries,crate::AreaSeries,crate::StackedAreaSeries,crate::StackedStepAreaSeries,
        crate::ColumnSeries,crate::RowSeries,crate::StackedColumnSeries,crate::StackedRowSeries,crate::ScatterSeries,
        crate::HeatSeries,crate::RangeColumnSeries,crate::RangeRowSeries,crate::RangeLineSeries,
        crate::CandlesticksSeries,crate::BoxSeries,crate::PieSeries,crate::PolarLineSeries,crate::ChartAutomationPeer,
        crate::SankeyNode,crate::SankeyLink,crate::TreemapNode,crate::HeatLand,crate::GeoMarker],
    types:[crate::ChartKind,crate::LegendPosition,crate::TooltipPosition,crate::ZoomAndPanMode,
        crate::ChartPoints,crate::ChartSeriesCollection,crate::Axes,
        crate::SankeyNodes,crate::SankeyLinks,crate::TreemapNodes,crate::HeatLands,crate::GeoMarkers],
    contracts:[],generics:[],generic_contracts:[],
}

/// Registers native classes, collections, enums and their runtime/compiled markup metadata.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        ferroui_livecharts::register_types();
        crate::advanced_models::register_value_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        MarkupType::register_all(&[
            <crate::ChartKind as MarkupTyped>::MARKUP,
            <crate::LegendPosition as MarkupTyped>::MARKUP,
            <crate::TooltipPosition as MarkupTyped>::MARKUP,
            <crate::ZoomAndPanMode as MarkupTyped>::MARKUP,
            <crate::ChartPoints as MarkupTyped>::MARKUP,
            <crate::ChartSeriesCollection as MarkupTyped>::MARKUP,
            <crate::Axes as MarkupTyped>::MARKUP,
            <crate::SankeyNodes as MarkupTyped>::MARKUP,
            <crate::SankeyLinks as MarkupTyped>::MARKUP,
            <crate::TreemapNodes as MarkupTyped>::MARKUP,
            <crate::HeatLands as MarkupTyped>::MARKUP,
            <crate::GeoMarkers as MarkupTyped>::MARKUP,
        ]);
        register_rust_paths();
    });
}
