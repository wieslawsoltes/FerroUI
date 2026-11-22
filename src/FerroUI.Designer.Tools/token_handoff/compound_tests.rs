use super::*;
use serde_json::{json, Value};

#[test]
fn imports_typed_compound_aliases_and_reports_native_value_conversion() {
    let source = r##"{
        "tint":{"$type":"color","$value":{"colorSpace":"srgb","components":[0,0.2,0.4],"alpha":0.5}},
        "corner":{"$type":"dimension","$value":{"value":0.5,"unit":"rem"}},
        "middle":{"$type":"number","$value":0.25},
        "base":{"$type":"shadow","$value":{"color":"{tint}","offsetX":{"value":-2,"unit":"px"},
            "offsetY":"{corner}","blur":"{corner}","spread":{"value":-1,"unit":"px"}}},
        "layered":{"$type":"shadow","$value":["{base}",{"color":"{tint}","offsetX":"{corner}",
            "offsetY":"{corner}","blur":"{corner}","spread":{"value":1,"unit":"px"},"inset":true}]},
        "draft":{"$type":"gradient","$value":[{"color":"{tint}","position":-1},
            {"color":"{tint}","position":"{middle}"},{"color":"{tint}","position":4}]},
        "single":{"$type":"gradient","$value":[{"$ref":"#/draft/$value/1"}]},
        "shared":{"$type":"gradient","$value":["{single}",{"$ref":"#/draft/$value/2"}]},
        "curve":{"$type":"cubicBezier","$value":["{middle}",-0.3,0.75,1.4]}
    }"##;
    let plan = plan_import(
        source,
        &Options {
            pixels_per_rem: Some(16.0),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(plan.issues.is_empty(), "{:?}", plan.issues);
    assert!(plan.source.contains(
        r##"<BoxShadows x:Key="layered">-2 8 8 -1 #80003366, inset 8 8 8 1 #80003366</BoxShadows>"##
    ));
    assert!(plan
        .source
        .contains(r#"<KeySpline x:Key="curve">0.25,-0.3,0.75,1.4</KeySpline>"#));
    assert!(plan
        .source
        .contains(r#"<LinearGradientBrush x:Key="draft" EndPoint="100%,0%" StartPoint="0%,0%">"#));
    assert!(plan
        .source
        .contains(r##"<GradientStop Offset="0.25" Color="#80003366"/>"##));
    assert!(plan
        .mappings
        .iter()
        .find(|row| row.key == "draft")
        .unwrap()
        .note
        .as_deref()
        .unwrap()
        .contains("Clamped"));
    let exported = plan_export(&plan.source, &Options::default()).unwrap();
    assert!(exported.issues.is_empty(), "{:?}", exported.issues);
    let values: Value = serde_json::from_str(&exported.source).unwrap();
    assert_eq!(values["layered"]["$value"][1]["inset"], true);
    assert_eq!(values["draft"]["$value"][2]["position"], 1.0);
    assert_eq!(values["curve"]["$value"], json!([0.25, -0.3, 0.75, 1.4]));
}

#[test]
fn round_trips_gradient_geometry_and_rejects_nonliteral_xaml() {
    let xaml = r##"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:m="using:FerroUI.Media" xmlns:app="urn:app">
        <LinearGradientBrush x:Key="Linear">
            <LinearGradientBrush.GradientStops><GradientStop Color="#813F" Offset="0.15"/><GradientStop Color="#123" Offset="0.15"/></LinearGradientBrush.GradientStops>
        </LinearGradientBrush>
        <m:RadialGradientBrush x:Key="Radial" Center="25%,75%" GradientOrigin="5,12" RadiusX="30%" RadiusY="8" SpreadMethod="Reflect" Opacity="0.75">
            <m:GradientStop Color="#335577"/>
        </m:RadialGradientBrush>
        <ConicGradientBrush x:Key="Conic" Angle="180" Center="55%,40%"><GradientStop Color="#335577"/></ConicGradientBrush>
        <LinearGradientBrush x:Key="Bound"><GradientStop Offset="0" Color="{StaticResource Tint}"/></LinearGradientBrush>
        <LinearGradientBrush x:Key="Transformed" Transform="{StaticResource Rotate}"><GradientStop Color="#335577"/></LinearGradientBrush>
        <LinearGradientBrush x:Key="Outside"><GradientStop Offset="1.5" Color="#335577"/></LinearGradientBrush>
        <app:ConicGradientBrush x:Key="Foreign"><GradientStop Color="#335577"/></app:ConicGradientBrush>
        <KeySpline x:Key="Ease" ControlPointX1="0.25" ControlPointY1="0.5" ControlPointX2="0.75" ControlPointY2="1.25"/>
        <BoxShadows x:Key="Empty">none</BoxShadows>
    </ResourceDictionary>"##;
    let output = plan_export(xaml, &Options::default()).unwrap();
    assert_eq!(
        output
            .issues
            .iter()
            .map(|row| row.path.as_str())
            .collect::<Vec<_>>(),
        ["Bound", "Transformed", "Outside", "Foreign"]
    );
    let mut tokens: Value = serde_json::from_str(&output.source).unwrap();
    assert_eq!(
        tokens["Linear"]["$extensions"][EXTENSION][GRADIENT_PROPERTIES],
        json!({})
    );
    assert_eq!(
        tokens["Radial"]["$extensions"][EXTENSION][GRADIENT_PROPERTIES]["RadiusY"],
        "8"
    );
    assert_eq!(tokens["Ease"]["$value"], json!([0.25, 0.5, 0.75, 1.25]));
    tokens["Alias"] = json!({"$value":"{Radial}","$extensions":{EXTENSION:{"key":"Alias"}}});
    let imported = plan_import(&tokens.to_string(), &Options::default()).unwrap();
    assert!(imported.issues.is_empty(), "{:?}", imported.issues);
    assert!(imported
        .source
        .contains(r#"<LinearGradientBrush x:Key="Linear">"#));
    assert!(imported.source.contains(r#"<RadialGradientBrush x:Key="Alias" Center="25%,75%" GradientOrigin="5,12" Opacity="0.75" RadiusX="30%" RadiusY="8" SpreadMethod="Reflect">"#));
    assert!(imported
        .source
        .contains(r#"<ConicGradientBrush x:Key="Conic" Angle="180" Center="55%,40%">"#));
    assert!(imported
        .source
        .contains(r#"<BoxShadows x:Key="Empty">none</BoxShadows>"#));
    let second = plan_export(&imported.source, &Options::default()).unwrap();
    assert!(second.issues.is_empty(), "{:?}", second.issues);
    let mut second: Value = serde_json::from_str(&second.source).unwrap();
    second.as_object_mut().unwrap().remove("Alias");
    tokens.as_object_mut().unwrap().remove("Alias");
    assert_eq!(second, tokens);

    let options: Options =
        serde_json::from_value(json!({"mappings":{"Radial":{"xamlType":"ConicGradientBrush"}}}))
            .unwrap();
    let chosen = plan_import(&tokens.to_string(), &options).unwrap();
    assert!(chosen.issues.is_empty(), "{:?}", chosen.issues);
    assert!(chosen
        .source
        .contains(r#"<ConicGradientBrush x:Key="Radial" Opacity="0.75" SpreadMethod="Reflect">"#));
    assert!(chosen
        .mappings
        .iter()
        .find(|row| row.key == "Radial")
        .unwrap()
        .note
        .as_deref()
        .unwrap()
        .contains("in place of RadialGradientBrush geometry"));
}

#[test]
fn rejects_nonrepresentable_compound_parts_without_affecting_other_tokens() {
    let source = r##"{
        "count":{"$type":"number","$value":12},
        "wrong":{"$type":"duration","$value":{"unit":"px","value":12}},
        "tint":{"$type":"color","$value":{"colorSpace":"srgb","components":[1,0,0]}},
        "shadow":{"$type":"shadow","$value":{"color":"{tint}","offsetX":"{wrong}","offsetY":"{wrong}","blur":"{wrong}","spread":"{wrong}"}},
        "stops":{"$type":"gradient","$value":[{"color":"{tint}","position":0},{"color":"{tint}","position":1}]},
        "nested":{"$type":"gradient","$value":["{stops}"]},
        "cycle":{"$type":"gradient","$value":["{cycle}"]},
        "badPoint":{"$type":"gradient","$value":"{stops}","$extensions":{"com.github.ferroui":{"gradientProperties":{"StartPoint":"0%,1"}}}},
        "evilPoint":{"$type":"gradient","$value":"{stops}","$extensions":{"com.github.ferroui":{"gradientProperties":{"StartPoint":"{Binding Name}"}}}},
        "badCurve":{"$type":"cubicBezier","$value":[1.5,0,0.6,1]}
    }"##;
    let plan = plan_import(source, &Options::default()).unwrap();
    for (name, expected) in [
        ("shadow", "does not match"),
        ("nested", "single gradient stop"),
        ("cycle", "Circular"),
        ("badPoint", "same unit"),
        ("evilPoint", "literal"),
        ("badCurve", "between zero and one"),
    ] {
        assert!(
            plan.issues
                .iter()
                .any(|row| row.path == name && row.message.contains(expected)),
            "{name} {:?}",
            plan.issues
        );
    }
    assert_eq!(
        plan.mappings
            .iter()
            .map(|row| row.key.as_str())
            .collect::<Vec<_>>(),
        ["count", "stops", "tint"]
    );
}
