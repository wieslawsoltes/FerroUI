use ferroui_base::metadata::{MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition};
use ferroui_base::TypeInfo;

const NAMESPACES: &[(&str, &str)] = &[("ferroui_livecharts", "FerroUI.LiveCharts.Core")];
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.LiveCharts",
    crate_name: "ferroui_livecharts",
    xmlns_definitions: &[XmlnsDefinition {
        xml_namespace: "https://ferroui.dev/livecharts/core",
        namespace: "FerroUI.LiveCharts.Core",
    }],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {
    classes:[],
    types:[crate::SeriesKind,crate::AxisPosition,crate::FindingStrategy,crate::BrushSpace,crate::RadialAlignment,crate::PolarLabelsPosition,
        crate::geo::MapProjection,crate::sankey_chart_engine::SankeyLayoutKind,
        crate::sankey_chart_engine::SankeyLabelPosition],
    contracts:[],generics:[],generic_contracts:[],
}
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_base::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        MarkupAssembly::register(&ASSEMBLY);
        MarkupType::register_all(&[
            <crate::SeriesKind as MarkupTyped>::MARKUP,
            <crate::AxisPosition as MarkupTyped>::MARKUP,
            <crate::FindingStrategy as MarkupTyped>::MARKUP,
            <crate::BrushSpace as MarkupTyped>::MARKUP,
            <crate::RadialAlignment as MarkupTyped>::MARKUP,
            <crate::PolarLabelsPosition as MarkupTyped>::MARKUP,
            <crate::geo::MapProjection as MarkupTyped>::MARKUP,
            <crate::sankey_chart_engine::SankeyLayoutKind as MarkupTyped>::MARKUP,
            <crate::sankey_chart_engine::SankeyLabelPosition as MarkupTyped>::MARKUP,
        ]);
        register_rust_paths();
    });
}
