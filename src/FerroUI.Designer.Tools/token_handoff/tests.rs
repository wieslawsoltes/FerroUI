use super::*;
use ferroui_designer_core::Document;
use serde_json::{json, Value};

fn options(value: Value) -> Options {
    serde_json::from_value(value).unwrap()
}

#[test]
fn imports_typed_groups_root_tokens_and_property_references() {
    let source = r##"{
        "brand": {"$type":"color", "a/b": {"$value":{"colorSpace":"srgb","components":[0,0.2,0.4],"alpha":0.5}}},
        "accent": {"$value":"{brand.a/b}"},
        "highlight": {"$ref":"#/brand/a~1b/$value"},
        "green": {"$type":"number","$ref":"#/brand/a~1b/$value/components/1"},
        "layout": {"$type":"dimension", "$root":{"$value":{"value":12,"unit":"px"}},
            "wide":{"$value":{"value":1.5,"unit":"rem"}},
            "narrow":{"$value":{"value":{"$ref":"#/layout/$root/$value/value"},"unit":"px"}}},
        "fade":{"$type":"duration","$value":{"value":250,"unit":"ms"}}
    }"##;
    let plan = plan_import(source, &options(json!({"pixelsPerRem":16,"mappings":{"highlight":{"key":"HighlightColor","xamlType":"Color"}}}))).unwrap();
    assert!(plan.issues.is_empty(), "{:?}", plan.issues);
    assert_eq!(plan.mappings.len(), 8);
    assert!(plan
        .source
        .contains(r##"<SolidColorBrush x:Key="accent" Color="#80003366"/>"##));
    assert!(plan
        .source
        .contains(r##"<Color x:Key="HighlightColor">#80003366</Color>"##));
    assert!(plan
        .source
        .contains(r#"<x:Double x:Key="green">0.2</x:Double>"#));
    assert!(plan
        .source
        .contains(r#"<x:Double x:Key="layout.wide">24</x:Double>"#));
    assert!(plan
        .source
        .contains(r#"<sys:TimeSpan x:Key="fade">0.00:00:00.2500000</sys:TimeSpan>"#));
    assert!(plan
        .mappings
        .iter()
        .find(|entry| entry.key == "accent")
        .unwrap()
        .note
        .as_deref()
        .unwrap()
        .contains("{brand.a/b}"));
    Document::new(plan.source).unwrap();
}

#[test]
fn reports_cycles_invalid_paths_conflicts_and_unsupported_values() {
    let source = r##"{
        "a":{"$value":"{b}"}, "b":{"$ref":"#/a"},
        "bad":{"$ref":"#/bad~2key"}, "remote":{"$ref":"other.json#/c"},
        "deep":{"$type":"number","$value":2}, "typed":{"$type":"color","$value":"{deep}"},
        "a b":{"$type":"number","$value":1}, "a-b":{"$type":"number","$value":4},
        "stretch":{"$type":"dimension","$value":{"value":1,"unit":"rem"}},
        "gamut":{"$type":"color","$value":{"colorSpace":"display-p3","components":[0,1,0]}},
        "typography":{"$type":"typography","$value":{}},
        "variants":{"$extends":"{other}"}
    }"##;
    let result = plan_import(source, &Options::default()).unwrap();
    assert_eq!(
        result
            .mappings
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        ["deep"]
    );
    for (path, message) in [
        ("a", "Circular"),
        ("b", "Circular"),
        ("bad", "escape"),
        ("remote", "same-document"),
        ("typed", "does not match"),
        ("a b", "Duplicate"),
        ("a-b", "Duplicate"),
        ("stretch", "pixelsPerRem"),
        ("gamut", "sRGB"),
        ("typography", "Missing fontFamily"),
        ("variants", "inheritance"),
    ] {
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.path == path && issue.message.contains(message)),
            "{path}: {:?}",
            result.issues
        );
    }
    let remap = plan_import(
        source,
        &options(json!({"mappings":{"a b":{"key":"firstValue"}}})),
    )
    .unwrap();
    assert!(remap.mappings.iter().any(|entry| entry.key == "firstValue"));
    assert!(remap.mappings.iter().any(|entry| entry.key == "a-b"));
}

#[test]
fn exports_literal_dictionary_resources_with_explicit_round_trip_mappings() {
    let xaml = r##"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:u="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:s="using:System">
        <SolidColorBrush u:Key="Brand" Color="#84345678"/>
        <Color u:Key="Tint">#336699</Color>
        <u:Double u:Key="size.card">12</u:Double>
        <s:TimeSpan u:Key="time.fade">0:0:0.25</s:TimeSpan>
        <SolidColorBrush u:Key="Bound" Color="{DynamicResource Tint}"/>
        <SolidColorBrush u:Key="Opacity" Color="#123" Opacity="0.5"/>
    </ResourceDictionary>"##;
    let plan = plan_export(xaml, &options(json!({"mappings":{
        "Brand":{"token":"palette.$root"},"Tint":{"token":"palette.tint"},"size.card":{"kind":"dimension"}
    }}))).unwrap();
    assert_eq!(
        plan.issues
            .iter()
            .map(|issue| issue.path.as_str())
            .collect::<Vec<_>>(),
        ["Bound", "Opacity"]
    );
    let json: Value = serde_json::from_str(&plan.source).unwrap();
    assert_eq!(
        json["palette"]["$root"]["$extensions"][EXTENSION]["key"],
        "Brand"
    );
    assert_eq!(json["size"]["card"]["$value"]["unit"], "px");
    assert_eq!(json["time"]["fade"]["$value"]["value"], 250.0);
    let roundtrip = plan_import(&plan.source, &Options::default()).unwrap();
    assert!(roundtrip.issues.is_empty(), "{:?}", roundtrip.issues);
    assert!(roundtrip
        .source
        .contains(r##"<Color x:Key="Tint">#336699</Color>"##));
    assert!(roundtrip
        .source
        .contains(r##"<SolidColorBrush x:Key="Brand" Color="#84345678"/>"##));
}
