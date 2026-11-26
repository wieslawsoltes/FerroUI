use super::*;
use serde_json::{json, Value};

#[test]
fn typography_resolves_typed_fields_and_round_trips_native_theme() {
    let source = json!({
        "family":{"$type":"fontFamily", "$value":["A & B", "Noto Sans JP", "Arial"]},
        "weight":{"$type":"fontWeight", "$value":"demi-bold"},
        "size":{"$type":"dimension", "$value":{"value":1.25,"unit":"rem"}},
        "spacing":{"$type":"dimension", "$value":{"value":-0.5,"unit":"px"}},
        "ratio":{"$type":"number", "$value":1.2},
        "caption":{"$type":"typography", "$value":{
            "fontFamily":"{family}", "fontWeight":"{weight}", "fontSize":"{size}",
            "letterSpacing":"{spacing}", "lineHeight":{"$ref":"#/ratio/$value"}
        }},
    });
    let options = Options {
        pixels_per_rem: Some(16.0),
        ..Options::default()
    };
    let imported = plan_import(&source.to_string(), &options).unwrap();
    assert!(imported.issues.is_empty(), "{:?}", imported.issues);
    assert!(imported
        .source
        .contains(r#"<FontFamily x:Key="family">A &amp; B, Noto Sans JP, Arial</FontFamily>"#));
    assert!(imported
        .source
        .contains(r#"<FontWeight x:Key="weight">600</FontWeight>"#));
    assert!(imported
        .source
        .contains(r#"<ControlTheme x:Key="caption" TargetType="TextBlock">"#));
    assert!(imported
        .source
        .contains(r#"<Setter Property="LineHeight" Value="24"/>"#));
    assert!(imported
        .source
        .contains(r#"<Setter Property="LetterSpacing" Value="-0.5"/>"#));
    let note = imported
        .mappings
        .iter()
        .find(|row| row.key == "caption")
        .unwrap()
        .note
        .as_deref()
        .unwrap();
    assert!(note.contains("1.25 rem at 16 logical pixels per rem"));
    assert!(note.contains("line-height multiplier"));

    let exported = plan_export(&imported.source, &Options::default()).unwrap();
    assert!(exported.issues.is_empty(), "{:?}", exported.issues);
    let values: Value = serde_json::from_str(&exported.source).unwrap();
    assert_eq!(values["family"]["$value"], source["family"]["$value"]);
    assert_eq!(values["weight"]["$value"], 600);
    assert_eq!(
        values["caption"]["$value"],
        json!({
            "fontFamily":["A & B", "Noto Sans JP", "Arial"],
            "fontSize":{"value":20.0,"unit":"px"},
            "fontWeight":600,
            "letterSpacing":{"value":-0.5,"unit":"px"},
            "lineHeight":1.2,
        })
    );
    let reimported = plan_import(&exported.source, &Options::default()).unwrap();
    assert_eq!(reimported.source, imported.source);
}

#[test]
fn typography_reports_unrepresentable_weights_names_and_aliases() {
    let source = json!({
        "comma":{"$type":"fontFamily","$value":"Arial, Helvetica"},
        "empty":{"$type":"fontFamily","$value":[]},
        "asset":{"$type":"fontFamily","$value":["Segoe UI","fonts:Brand#Brand"]},
        "fractional":{"$type":"fontWeight","$value":550.5},
        "wrongCase":{"$type":"fontWeight","$value":"Bold"},
        "maximum":{"$type":"fontWeight","$value":1000},
        "minimum":{"$type":"fontWeight","$value":1},
        "other":{"$type":"number","$value":500},
        "badReference":{"$type":"typography","$value":{
            "fontFamily":"Arial", "fontSize":{"value":12,"unit":"px"},
            "fontWeight":"{other}","letterSpacing":{"value":0,"unit":"px"},"lineHeight":1
        }},
        "overflow":{"$type":"typography","$value":{
            "fontFamily":"Arial", "fontSize":{"value":1e308,"unit":"px"},
            "fontWeight":"hairline","letterSpacing":{"value":0,"unit":"px"},"lineHeight":10
        }}
    });
    let plan = plan_import(&source.to_string(), &Options::default()).unwrap();
    assert_eq!(
        plan.issues
            .iter()
            .map(|issue| issue.path.as_str())
            .collect::<Vec<_>>(),
        [
            "asset",
            "badReference",
            "comma",
            "empty",
            "fractional",
            "overflow",
            "wrongCase"
        ]
    );
    assert!(plan
        .issues
        .iter()
        .any(|issue| issue.message.contains("does not match fontWeight")));
    assert!(plan
        .source
        .contains(r#"<FontWeight x:Key="minimum">1</FontWeight>"#));
    assert!(plan
        .source
        .contains(r#"<FontWeight x:Key="maximum">1000</FontWeight>"#));
}

#[test]
fn exports_only_complete_literal_typography_for_a_native_text_block() {
    let setters = r#"
        <!-- Authored order is immaterial. -->
        <s:Setter Property="LineHeight" Value="30"/>
        <s:Setter Property="LetterSpacing" Value="0.5"/>
        <s:Setter Property="FontSize" Value="20"/>
        <s:Setter Property="FontWeight" Value="UltraBlack"/>
        <s:Setter Property="FontFamily" Value="Arial, Helvetica"/>"#;
    let source = format!(
        r#"<ResourceDictionary xmlns="https://github.com/ferroui"
            xmlns:l="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:s="using:FerroUI.Styling"
            xmlns:c="using:FerroUI.Controls" xmlns:f="urn:foreign">
        <s:ControlTheme l:Key="Heading" TargetType="{{l:Type c:TextBlock}}">{setters}</s:ControlTheme>
        <ControlTheme l:Key="Inherited" TargetType="TextBlock" BasedOn="{{StaticResource Heading}}">{setters}</ControlTheme>
        <ControlTheme l:Key="ForeignTarget" TargetType="f:TextBlock">{setters}</ControlTheme>
        <ControlTheme l:Key="Extra" TargetType="TextBlock">{setters}<Setter Property="Foreground" Value="Black"/></ControlTheme>
        <ControlTheme l:Key="Duplicate" TargetType="TextBlock">{setters}<Setter Property="FontSize" Value="30"/></ControlTheme>
        <ControlTheme l:Key="Dynamic" TargetType="TextBlock">{}</ControlTheme>
        <ControlTheme l:Key="Invalid" TargetType="TextBlock">{}</ControlTheme>
        <FontFamily l:Key="Asset">fonts:Brand#Brand</FontFamily>
        <FontWeight l:Key="Named">semilight</FontWeight>
        <FontWeight l:Key="Over">1001</FontWeight>
    </ResourceDictionary>"#,
        setters.replace("Arial, Helvetica", "{DynamicResource Font}"),
        setters.replace("Value=\"20\"", "Value=\"0\""),
    );
    let exported = plan_export(&source, &Options::default()).unwrap();
    assert_eq!(
        exported
            .issues
            .iter()
            .map(|issue| issue.path.as_str())
            .collect::<Vec<_>>(),
        [
            "Inherited",
            "ForeignTarget",
            "Extra",
            "Duplicate",
            "Dynamic",
            "Invalid",
            "Asset",
            "Over"
        ]
    );
    let values: Value = serde_json::from_str(&exported.source).unwrap();
    assert_eq!(
        values["Heading"]["$value"]["fontFamily"],
        json!(["Arial", "Helvetica"])
    );
    assert_eq!(values["Heading"]["$value"]["fontWeight"], 950);
    assert_eq!(values["Heading"]["$value"]["lineHeight"], 1.5);
    assert_eq!(values["Named"]["$value"], 350);
}
