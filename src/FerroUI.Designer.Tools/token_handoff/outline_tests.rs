use super::{plan_export, plan_import, Options, EXTENSION};
use crate::components::{FRAMEWORK, XAML};
use serde_json::{json, Value};

#[test]
fn border_and_stroke_use_native_themes_and_preserve_absolute_dashes() {
    let tokens = json!({
        "ink":{"$type":"color","$value":{"colorSpace":"srgb","components":[0,0,1],"alpha":0.8}},
        "wide":{"$type":"dimension","$value":{"value":0.25,"unit":"rem"}},
        "dash":{"$type":"strokeStyle","$value":{"dashArray":["{wide}",{"value":2,"unit":"px"},{"value":1,"unit":"px"}],"lineCap":"square"}},
        "outline":{"$type":"border","$value":{"color":"{ink}","width":{"$ref":"#/wide/$value"},"style":"{dash}"}},
        "reading":{"$type":"border","$value":{"color":"{ink}","width":{"value":1.5,"unit":"px"},"style":"solid"}},
        "sample":{"$type":"strokeStyle","$value":"dotted"},
    });
    let plan = plan_import(
        &tokens.to_string(),
        &Options {
            pixels_per_rem: Some(16.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(plan.issues.is_empty(), "{:?}", plan.issues);
    assert!(plan
        .source
        .contains(r#"<ControlTheme x:Key="outline" TargetType="Shape">"#));
    assert!(plan
        .source
        .contains(r#"<ControlTheme x:Key="reading" TargetType="Border">"#));
    assert!(plan
        .source
        .contains(r##"<Setter Property="Stroke" Value="#CC0000FF"/>"##));
    assert!(plan
        .source
        .contains(r#"<Setter Property="StrokeDashArray" Value="1,0.5,0.25"/>"#));
    assert!(plan
        .source
        .contains(r#"<Setter Property="StrokeLineCap" Value="Square"/>"#));
    assert!(plan
        .source
        .contains(r#"<Setter Property="BorderThickness" Value="1.5"/>"#));

    let exported = plan_export(&plan.source, &Options::default()).unwrap();
    assert!(exported.issues.is_empty(), "{:?}", exported.issues);
    let result: Value = serde_json::from_str(&exported.source).unwrap();
    assert_eq!(result["reading"]["$value"]["style"], "solid");
    assert_eq!(result["sample"]["$value"], "dotted");
    assert_eq!(
        result["outline"]["$value"]["style"]["dashArray"],
        json!([
            {"value":4.0,"unit":"px"},{"value":2.0,"unit":"px"},{"value":1.0,"unit":"px"}
        ])
    );
    let restored = plan_import(&exported.source, &Options::default()).unwrap();
    assert!(restored.issues.is_empty(), "{:?}", restored.issues);
    assert!(restored
        .source
        .contains(r#"<Setter Property="StrokeDashArray" Value="1,0.5,0.25"/>"#));
}

#[test]
fn outline_round_trip_retains_native_target_and_explicit_stroke_width() {
    let xaml = format!(
        r##"<ResourceDictionary xmlns="{FRAMEWORK}" xmlns:l="{XAML}" xmlns:c="using:FerroUI.Controls.Shapes">
      <ControlTheme l:Key="Review" TargetType="{{l:Type c:Rectangle}}">
        <Setter Property="StrokeThickness" Value="4"/>
        <Setter Property="StrokeDashArray" Value="1.5,0.5"/>
        <Setter Property="StrokeLineCap" Value="Round"/>
      </ControlTheme>
      <ControlTheme l:Key="GridLine" TargetType="Line">
        <Setter Property="Stroke" Value="#224466"/>
        <Setter Property="StrokeThickness" Value="2"/>
        <Setter Property="StrokeDashArray" Value="{{l:Null}}"/>
      </ControlTheme>
    </ResourceDictionary>"##
    );
    let exported = plan_export(&xaml, &Options::default()).unwrap();
    assert!(exported.issues.is_empty(), "{:?}", exported.issues);
    let values: Value = serde_json::from_str(&exported.source).unwrap();
    assert_eq!(
        values["Review"]["$extensions"][EXTENSION]["outline"],
        json!({"targetType":"Rectangle","strokeWidth":4.0,"omitDefaultSetters":["StrokeDashOffset"]})
    );
    assert_eq!(values["GridLine"]["$value"]["style"], "solid");
    let plan = plan_import(&exported.source, &Options::default()).unwrap();
    assert!(plan.issues.is_empty(), "{:?}", plan.issues);
    assert!(plan
        .source
        .contains(r#"<ControlTheme x:Key="Review" TargetType="Rectangle">"#));
    assert!(plan
        .source
        .contains(r#"<ControlTheme x:Key="GridLine" TargetType="Line">"#));
    assert!(plan
        .source
        .contains(r#"<Setter Property="StrokeThickness" Value="4"/>"#));
    assert!(plan
        .source
        .contains(r#"<Setter Property="StrokeDashArray" Value="1.5,0.5"/>"#));
    assert!(!plan.source.contains("StrokeDashOffset"));
}

#[test]
fn unsupported_strokes_and_unfaithful_xaml_exports_are_reported() {
    let tokens = json!({
        "double":{"$type":"strokeStyle","$value":"double"},
        "negative":{"$type":"strokeStyle","$value":{"dashArray":[{"value":-1,"unit":"px"}],"lineCap":"butt"}},
        "zero":{"$type":"strokeStyle","$value":{"dashArray":[{"value":0,"unit":"px"}],"lineCap":"round"}},
        "absolute":{"$type":"strokeStyle","$value":{"dashArray":[{"value":2,"unit":"px"}],"lineCap":"triangle"}}
    });
    let imported = plan_import(&tokens.to_string(), &Options::default()).unwrap();
    assert!(imported.mappings.is_empty());
    assert_eq!(imported.issues.len(), 4);

    let xaml = format!(
        r##"<ResourceDictionary xmlns="{FRAMEWORK}" xmlns:x="{XAML}">
      <ControlTheme x:Key="Asymmetric" TargetType="Border">
        <Setter Property="BorderBrush" Value="#123456"/>
        <Setter Property="BorderThickness" Value="0,1,1,0"/>
      </ControlTheme>
      <ControlTheme x:Key="Named" TargetType="Shape">
        <Setter Property="StrokeDashArray" Value="2,2"/>
        <Setter Property="StrokeDashOffset" Value="1"/>
      </ControlTheme>
      <ControlTheme x:Key="Relative" TargetType="Shape">
        <Setter Property="StrokeDashArray" Value="3,1"/>
      </ControlTheme>
      <ControlTheme x:Key="Dynamic" TargetType="Border">
        <Setter Property="BorderBrush" Value="{{StaticResource Stroke}}"/>
        <Setter Property="BorderThickness" Value="2"/>
      </ControlTheme>
    </ResourceDictionary>"##
    );
    let exported = plan_export(&xaml, &Options::default()).unwrap();
    assert!(exported.mappings.is_empty());
    assert_eq!(exported.issues.len(), 4);
}
