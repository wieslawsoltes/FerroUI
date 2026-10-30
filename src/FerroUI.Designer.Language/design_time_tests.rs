use crate::{LanguageService, PropertyInfo, Schema, TypeInfo};

const URI: &str = "untitled:/DesignTime.xaml";
const DESIGN: &str = "http://schemas.microsoft.com/expression/blend/2008";
const NS: &str = "xmlns='https://github.com/ferroui' xmlns:preview='http://schemas.microsoft.com/expression/blend/2008' xmlns:compat='http://schemas.openxmlformats.org/markup-compatibility/2006'";

fn service(source: &str) -> LanguageService {
    let mut service = LanguageService::default();
    service.open(URI, source, 1).unwrap();
    service
}

fn complete(source: &str) -> Vec<crate::Completion> {
    let offset = source.find('|').unwrap();
    let source = source.replacen('|', "", 1);
    service(&source).completions(URI, offset).unwrap()
}

#[test]
fn static_attached_owners_use_the_framework_metadata_and_namespace_aliases() {
    let schema = Schema::ferroui();
    let navigation = schema.type_info("KeyboardNavigation", None).unwrap();
    assert!(!navigation.constructible);
    let tab = schema
        .property("Button", None, "KeyboardNavigation.TabNavigation")
        .unwrap();
    assert!(tab.attached);
    for mode in ["Continue", "Cycle", "Contained", "Once", "None", "Local"] {
        assert!(tab.values.iter().any(|value| value == mode), "{mode}");
    }
    assert!(
        schema
            .type_info("KeyboardNavigation", Some("using:FerroUI.Input"))
            .is_some()
    );
    assert!(
        schema
            .type_info("KeyboardNavigation", Some("using:Application.Input"))
            .is_none()
    );
    assert!(schema.property("Button", None, "Design.Width").is_some());
    assert!(schema.property("Button", None, "ToolTip.Tip").is_some());
}

#[test]
fn designer_workflows_sample_has_clean_language_diagnostics() {
    let source = include_str!("../../docs/designer/samples/DesignWorkflows.xaml");
    let diagnostics = service(source).diagnostics(URI).unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn design_namespace_aliases_resolve_native_controls_overrides_and_compatibility_metadata() {
    let source = format!(
        "<Grid {NS} compat:Ignorable='preview' preview:DesignWidth='480' preview:DesignHeight='640'>\
         <preview:Button Name='PreviewAction' Content='Preview' preview:IsEnabled='True' preview:Grid.Row='1'/>\
         <Border><preview:Border.Background><SolidColorBrush Color='Blue'/></preview:Border.Background></Border>\
         <TextBlock Design.Text='Preview attached text'/>\
         </Grid>"
    );
    let service = service(&source);
    let diagnostics = service.diagnostics(URI).unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let hover = service
        .hover(URI, source.find("preview:DesignWidth").unwrap())
        .unwrap()
        .unwrap();
    assert!(hover.contents.contains("f64"));
    let hover = service
        .hover(URI, source.find("compat:Ignorable").unwrap())
        .unwrap()
        .unwrap();
    assert!(hover.contents.contains("namespace prefixes"));
    assert_eq!(
        Schema::ferroui()
            .type_info("preview:Button", Some(DESIGN))
            .unwrap()
            .name,
        "Button"
    );
}

#[test]
fn design_metadata_preserves_unknown_type_property_and_value_diagnostics() {
    let source = format!(
        "<Grid {NS} preview:DesignWidth='wide' preview:Unknown='bad' preview:Name='identity'>\
         <Button preview:IsEnabled='Maybe' UnknownRuntime='bad'/>\
         <preview:MissingControl/><Border><preview:Border.Unknown/></Border>\
         </Grid>"
    );
    let diagnostics = service(&source).diagnostics(URI).unwrap();
    for code in ["numeric-value", "enum-value", "unknown-type"] {
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.code == code),
            "{diagnostics:?}"
        );
    }
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "unknown-property")
            .count(),
        4
    );
    // Prefix spelling never grants framework/designer meaning to an application namespace.
    let source = "<Grid xmlns:d='using:Application' d:DesignWidth='wide'><d:Button/></Grid>";
    let diagnostics = service(source).diagnostics(URI).unwrap();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "unknown-type");
    assert!(
        Schema::ferroui()
            .type_info(
                "Button",
                Some("http://schemas.microsoft.com/expression/blend/2008/app")
            )
            .is_none()
    );
}

#[test]
fn design_completion_uses_the_authored_prefix_for_types_properties_values_and_elements() {
    for (source, expected, insert) in [
        (
            format!("<Grid {NS}><preview:TextB|"),
            "preview:TextBlock",
            "preview:TextBlock",
        ),
        (
            format!("<Button {NS} preview:DesignW|"),
            "preview:DesignWidth",
            "preview:DesignWidth=\"\"",
        ),
        (
            format!("<Button {NS} preview:Con|"),
            "preview:Content",
            "preview:Content=\"\"",
        ),
        (
            format!("<Button {NS} preview:KeyboardNavigation.TabN|"),
            "preview:KeyboardNavigation.TabNavigation",
            "preview:KeyboardNavigation.TabNavigation=\"\"",
        ),
        (
            format!("<Button {NS} preview:IsEnabled='T|'/>"),
            "True",
            "True",
        ),
        (
            format!("<Button {NS} compat:Ign|"),
            "compat:Ignorable",
            "compat:Ignorable=\"\"",
        ),
        (
            format!("<Button {NS}><preview:Button.Back|ground/></Button>"),
            "preview:Button.Background",
            "preview:Button.Background",
        ),
    ] {
        let items = complete(&source);
        assert!(
            items
                .iter()
                .any(|item| item.label == expected && item.insert_text == insert),
            "{source}: {items:?}"
        );
    }
    let names = complete(&format!("<Button {NS} preview:|"));
    assert!(!names.iter().any(|item| item.label == "preview:Name"));
    assert!(complete("<Grid xmlns:preview='using:Application'><preview:TextB|").is_empty());
}

#[test]
fn sample_data_completes_aliases_parameters_and_signature_and_validates_limits() {
    let names = complete(&format!(
        "<ListBox {NS} preview:ItemsSource='{{preview:Sa|}}'/>"
    ));
    assert!(names.iter().any(|item| item.label == "preview:SampleData"));
    let names = complete(&format!(
        "<ListBox {NS} preview:ItemsSource='{{preview:SampleData It|}}'/>"
    ));
    assert!(
        names
            .iter()
            .any(|item| item.label == "ItemCount" && item.insert_text == "ItemCount=")
    );
    let source =
        format!("<ListBox {NS} preview:ItemsSource='{{preview:SampleData ItemCount=4}}'/>");
    let service = service(&source);
    let signature = service
        .signature_help(
            URI,
            source.find("ItemCount=4").unwrap() + "ItemCount=".len(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(signature.signatures[0].parameters, vec!["ItemCount"]);
    assert!(signature.signatures[0].label.contains("preview:SampleData"));
    assert!(service.diagnostics(URI).unwrap().is_empty());
    for arguments in ["", " ItemCount=0", " ItemCount=200"] {
        let source =
            format!("<ListBox {NS} preview:ItemsSource='{{preview:SampleData{arguments}}}'/>");
        assert!(
            self::service(&source).diagnostics(URI).unwrap().is_empty(),
            "{source}"
        );
    }
    for (property, arguments) in [
        ("preview:ItemsSource", "ItemCount=201"),
        ("preview:ItemsSource", "ItemCount=-1"),
        ("preview:ItemsSource", "ItemCount={Binding Count}"),
        ("preview:ItemsSource", "Count=4"),
        ("preview:Height", "ItemCount=4"),
        ("ItemsSource", "ItemCount=4"),
    ] {
        let source = format!("<ListBox {NS} {property}='{{preview:SampleData {arguments}}}'/>");
        assert!(
            self::service(&source)
                .diagnostics(URI)
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic.code == "sample-data"),
            "{source}"
        );
    }
    let app = "<ListBox xmlns:d='using:Application' ItemsSource='{d:SampleData Count=900}'/>";
    assert!(self::service(app).diagnostics(URI).unwrap().is_empty());
    assert!(complete("<ListBox xmlns:d='using:Application' ItemsSource='{d:Sa|}'/>").is_empty());
}

#[test]
fn custom_design_overrides_retain_custom_metadata_and_readonly_checks() {
    let source = format!(
        "<app:Widget {NS} xmlns:app='using:Application' preview:Caption='Preview' preview:Version='2'><preview:Widget.Caption/></app:Widget>"
    );
    let mut service = service(&source);
    service.schema_mut().register(TypeInfo {
        name: "Widget".into(),
        namespace: "using:Application".into(),
        base: Some("Button".into()),
        constructible: true,
        properties: [
            PropertyInfo {
                name: "Caption".into(),
                type_name: "String".into(),
                ..Default::default()
            },
            PropertyInfo {
                name: "Version".into(),
                type_name: "String".into(),
                read_only: true,
                ..Default::default()
            },
        ]
        .into_iter()
        .map(|property| (property.name.clone(), property))
        .collect(),
        ..Default::default()
    });
    let diagnostics = service.diagnostics(URI).unwrap();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "read-only");
    let completions = service
        .completions(
            URI,
            source.find("preview:Caption").unwrap() + "preview:Cap".len(),
        )
        .unwrap();
    assert!(
        completions
            .iter()
            .any(|item| item.label == "preview:Caption")
    );
    let completions = service
        .completions(
            URI,
            source.find("preview:Widget.Caption").unwrap() + "preview:Widget.Cap".len(),
        )
        .unwrap();
    assert!(
        completions
            .iter()
            .any(|item| item.label == "preview:Widget.Caption")
    );
}

#[test]
fn concrete_design_context_objects_supply_typed_binding_members() {
    let source = format!(
        "<Grid {NS} xmlns:vm='using:Models'><preview:Grid.DataContext><vm:PreviewModel/></preview:Grid.DataContext><TextBlock Text='{{Binding Title}}'/></Grid>"
    );
    let mut service = service(&source);
    service.schema_mut().register(TypeInfo {
        name: "PreviewModel".into(),
        namespace: "using:Models".into(),
        constructible: true,
        properties: [PropertyInfo {
            name: "Title".into(),
            type_name: "String".into(),
            documentation: "Title of the preview model".into(),
            ..Default::default()
        }]
        .into_iter()
        .map(|property| (property.name.clone(), property))
        .collect(),
        ..Default::default()
    });
    assert!(service.diagnostics(URI).unwrap().is_empty());
    let offset = source.find("Binding Title").unwrap() + "Binding Ti".len();
    let completions = service.completions(URI, offset).unwrap();
    assert!(completions.iter().any(|item| item.label == "Title"));
    assert!(
        service
            .hover(URI, offset)
            .unwrap()
            .unwrap()
            .contents
            .contains("preview model")
    );
}
