//! Native class and XAML metadata for the Charts library.

use crate::*;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::media::IBrush;
use ferroui_base::metadata::{
    MarkupAssembly, MarkupType, MarkupTyped, XmlnsDefinition, FERRO_XML_NAMESPACE,
};
use ferroui_base::TypeInfo;
use std::rc::Rc;

ferroui_base::ferro_markup_type!(class DataLabelOverride {
    handles: [Rc<DataLabelOverride>, Option<Rc<DataLabelOverride>>],
    this: Rc<DataLabelOverride>,
    constructors: [(String, Option<Rc<dyn IBrush>>) => |text: String, brush: Option<Rc<dyn IBrush>>| DataLabelOverride::new(text, brush)],
    properties: [
        Text: String { get: |value: &Rc<DataLabelOverride>| value.text().to_owned() },
        Brush: Option<Rc<dyn IBrush>> { get: |value: &Rc<DataLabelOverride>| value.brush() },
    ],
});
ferroui_base::ferro_markup_type!(class DataMarkerOverride {
    handles: [Rc<DataMarkerOverride>, Option<Rc<DataMarkerOverride>>],
    this: Rc<DataMarkerOverride>,
    constructors: [(MarkerShape, Option<Rc<dyn IBrush>>) => DataMarkerOverride::new],
    properties: [
        Shape: MarkerShape { get: |value: &Rc<DataMarkerOverride>| value.shape() },
        Brush: Option<Rc<dyn IBrush>> { get: |value: &Rc<DataMarkerOverride>| value.brush() },
    ],
});

/// XML namespace for declarative Charts controls and data.
pub const CHARTS_XML_NAMESPACE: &str = "https://github.com/ferroui/charts";

const NAMESPACES: &[(&str, &str)] = &[
    ("ferroui_controls_charts", "FerroUI.Controls.Charts"),
    (
        "ferroui_controls_charts::chart_automation_peer",
        "FerroUI.Automation.Peers",
    ),
];

/// Assembly identity used by both compiled markup and the runtime loader.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.Controls.Charts",
    crate_name: "ferroui_controls_charts",
    xmlns_definitions: &[
        XmlnsDefinition {
            xml_namespace: CHARTS_XML_NAMESPACE,
            namespace: "FerroUI.Controls.Charts",
        },
        XmlnsDefinition {
            xml_namespace: FERRO_XML_NAMESPACE,
            namespace: "FerroUI.Controls.Charts",
        },
    ],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[
    Chart::TYPE,
    Axis::TYPE,
    CartesianAxis::TYPE,
    CategoryAxis::TYPE,
    DateTimeAxis::TYPE,
    LinearAxis::TYPE,
    CartesianSeries::TYPE,
    LineSeries::TYPE,
    AreaSeries::TYPE,
    BarSeries::TYPE,
    Samples::TYPE,
    XamlChartsResources::TYPE,
    ChartAutomationPeer::TYPE,
];

const MARKUP_TYPES: &[&MarkupType] = &[
    <BarOrientation as MarkupTyped>::MARKUP,
    <CategorySortKey as MarkupTyped>::MARKUP,
    <DateTimeIntervalType as MarkupTyped>::MARKUP,
    <GridLines as MarkupTyped>::MARKUP,
    <MarkerShape as MarkupTyped>::MARKUP,
    <SortOrder as MarkupTyped>::MARKUP,
    <StrokeDashStyle as MarkupTyped>::MARKUP,
    <DataLabelOverride as MarkupTyped>::MARKUP,
    <DataMarkerOverride as MarkupTyped>::MARKUP,
    <ChartSeriesCollection as MarkupTyped>::MARKUP,
    <ChartAxisCollection as MarkupTyped>::MARKUP,
    <ChartDataCollection as MarkupTyped>::MARKUP,
];

/// Registers all Charts classes, enums and collection adapters. Idempotent;
/// property initializers still run on the current UI thread when types are used.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupType::register_all(MARKUP_TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        crate::rust_paths::register_rust_paths();
        ValueTypes::register_global(register_value_types);
    });
}

fn register_value_types() {
    ValueTypes::register_nullable::<BarOrientation>();
    ValueTypes::register_nullable::<CategorySortKey>();
    ValueTypes::register_nullable::<DateTimeIntervalType>();
    ValueTypes::register_nullable::<GridLines>();
    ValueTypes::register_nullable::<MarkerShape>();
    ValueTypes::register_nullable::<SortOrder>();
    ValueTypes::register_nullable::<StrokeDashStyle>();
    ValueTypes::register_reference::<DataLabelOverride>();
    ValueTypes::register_reference::<DataMarkerOverride>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_charts_type_and_collection_has_native_metadata() {
        register_types();
        for type_ in TYPES {
            assert!(type_.namespace().starts_with("FerroUI"), "{type_}");
            assert!(TypeInfo::find(type_.namespace(), type_.name()).is_some());
        }
        for type_ in MARKUP_TYPES {
            assert_eq!(type_.namespace(), "FerroUI.Controls.Charts");
            assert!(MarkupType::find(type_.namespace(), type_.name).is_some());
        }
        assert!(Chart::TYPE.default_constructor().is_some());
        assert!(CartesianSeries::TYPE.default_constructor().is_none());
    }
}
