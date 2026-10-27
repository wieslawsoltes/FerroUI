//! Component automation exercises the same session boundaries as native designer controls.
use super::*;
use ferroui_controls::ContentControl;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_tools::component_property_object;

const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel.Resources><SolidColorBrush x:Key="Accent" Color="Red"/><ControlTheme x:Key="Action" TargetType="Button"><Setter Property="Padding" Value="12"/></ControlTheme><ControlTemplate x:Key="Chrome" TargetType="Button"><Border Name="ChromePart" Background="{TemplateBinding Background}"/></ControlTemplate></StackPanel.Resources><Button Name="First" Width="100" Background="{StaticResource Accent}" Theme="{StaticResource Action}"/><Button Name="Second" Theme="{StaticResource Action}"/></StackPanel>"#;

fn fixture(source: &str) -> (Rc<RefCell<DesignerSession>>, DesignerMcp) {
    let session = Rc::new(RefCell::new(
        DesignerSession::new(source, ContentControl::new()).unwrap(),
    ));
    let provider = DesignerMcp::new(&session);
    (session, provider)
}

fn write() -> Permissions {
    Permissions {
        allow_property_writes: true,
        allow_input: false,
    }
}

fn call_edit(
    session: &Rc<RefCell<DesignerSession>>,
    provider: &DesignerMcp,
    name: &str,
    mut args: Value,
) -> Value {
    args["expected_revision"] = json!(session.borrow().document().revision());
    provider.call(name, &args, write()).unwrap()
}

fn edit(session: &Rc<RefCell<DesignerSession>>, provider: &DesignerMcp, args: Value) -> Value {
    call_edit(session, provider, "designer_component", args)
}

fn resource(session: &Rc<RefCell<DesignerSession>>, key: &str) -> u64 {
    session
        .borrow()
        .component_catalog()
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.key.as_deref() == Some(key))
        .unwrap()
        .node
}

fn named(session: &Rc<RefCell<DesignerSession>>, name: &str) -> u64 {
    session.borrow().document().ast().find_name(name).unwrap()
}

fn attribute(session: &Rc<RefCell<DesignerSession>>, node: u64, name: &str) -> Option<String> {
    session
        .borrow()
        .document()
        .ast()
        .node(node)
        .unwrap()
        .attribute(name)
        .map(str::to_owned)
}

fn assert_expression(
    session: &Rc<RefCell<DesignerSession>>,
    node: u64,
    property: &str,
    expected: &str,
) {
    use ferroui_designer_core::MarkupValue;
    let actual = attribute(session, node, property).expect("An authored expression");
    assert_eq!(
        MarkupValue::parse(&actual).unwrap(),
        MarkupValue::parse(expected).unwrap()
    );
}

#[test]
fn component_action_schemas_reject_cross_action_arguments_and_invalid_reset_values() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (_session, provider) = fixture(SOURCE);
    let invalid = [
        json!({"action":"create_resource","owner":1,"key":"New","source":null}),
        json!({"action":"create_resource","owner":1,"key":"New","source":"<SolidColorBrush/>","node":2}),
        json!({"action":"set_resource_value","resource":2,"value":null}),
        json!({"action":"replace_definition","node":2,"source":null}),
        json!({"action":"set_property","node":2,"property":"Width","value":12}),
        json!({"action":"set_object_property","node":2,"property":"Background"}),
        json!({"action":"apply_resource","resource":2,"targets":[],"property":"Background"}),
        json!({"action":"apply_resource","resource":2,"targets":[0],"property":"Background"}),
        json!({"action":"apply_resource","resource":2,"targets":[3],"property":"Background","dynamic":"true"}),
        json!({"action":"create_theme","owner":1,"key":"Theme","target_type":null}),
        json!({"action":"copy_definition_for_instance","definition":2,"target":3,"property":"Theme","key":"Copy","owner":1}),
    ];
    for mut args in invalid {
        args["expected_revision"] = json!(0);
        assert!(
            provider.validate("designer_component", &args).is_err(),
            "{args}"
        );
    }
    for mut args in [
        json!({"action":"set_property","node":2,"property":"Width","value":null}),
        json!({"action":"set_object_property","node":2,"property":"Background","source":null}),
        json!({"action":"set_setter","owner":2,"property":"Padding","value":null}),
        json!({"action":"set_setter_object","owner":2,"property":"Template","source":null}),
        json!({"action":"create_template","owner":1,"key":"Template","content":"<Border/>","target_type":null}),
    ] {
        args["expected_revision"] = json!(0);
        provider.validate("designer_component", &args).unwrap();
    }
    for mut args in [
        json!({"action":"enter","node":1,"options":{"unknown":true}}),
        json!({"action":"enter","node":1,"options":{"width":0}}),
        json!({"action":"enter","node":1,"options":{"height":8193}}),
        json!({"action":"enter","node":1,"options":{"sample_content":"x".repeat(65537)}}),
        json!({"action":"exit","node":1}),
        json!({"action":"exit","expected_scope_epoch":-1}),
    ] {
        args["expected_revision"] = json!(0);
        assert!(provider.validate("designer_definition", &args).is_err());
    }
    assert!(
        provider
            .validate(
                "designer_component",
                &json!({"action":"remove_resource","resource":2})
            )
            .is_err()
    );
    assert!(
        provider
            .validate(
                "designer_component_resources",
                &json!({"target":1,"dynamic":true})
            )
            .is_err()
    );
}

#[test]
fn component_writes_check_permissions_revision_read_only_and_atomic_dependencies() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture(SOURCE);
    let accent = resource(&session, "Accent");
    let first = named(&session, "First");
    let rename =
        json!({"expected_revision":0,"action":"rename_resource","resource":accent,"key":"Primary"});
    assert!(
        provider
            .call("designer_component", &rename, Permissions::default())
            .unwrap_err()
            .contains("allow_property_writes")
    );
    provider
        .call("designer_component", &rename, write())
        .unwrap();
    assert_expression(&session, first, "Background", "{StaticResource Primary}");
    assert_eq!(session.borrow().document().revision(), 1);
    assert!(
        provider
            .call("designer_component", &rename, write())
            .unwrap_err()
            .contains("revision")
    );
    let after_rename = session.borrow().document().source().to_owned();
    for args in [
        json!({"expected_revision":1,"action":"remove_resource","resource":accent}),
        json!({"expected_revision":1,"action":"apply_resource","resource":accent,"targets":[first,u64::MAX],"property":"Background","dynamic":true}),
    ] {
        assert!(provider.call("designer_component", &args, write()).is_err());
        assert_eq!(session.borrow().document().source(), after_rename);
        assert_eq!(session.borrow().document().revision(), 1);
    }
    call_edit(&session, &provider, "designer_undo", json!({}));
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert!(!session.borrow().can_undo());
    session.borrow_mut().set_read_only(true);
    let revision = session.borrow().document().revision();
    assert!(provider
        .call("designer_component", &json!({"expected_revision":revision,"action":"set_property","node":first,"property":"Width","value":"200"}), write())
        .unwrap_err()
        .contains("read-only"));
    assert!(
        provider
            .call("designer_components", &json!({}), Permissions::default())
            .is_ok()
    );
    assert_eq!(session.borrow().document().source(), SOURCE);
}

#[test]
fn component_reads_preserve_namespace_sources_references_and_native_theme_visibility() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture(SOURCE);
    let accent = resource(&session, "Accent");
    let first = named(&session, "First");
    let compact = provider
        .call("designer_components", &json!({}), Permissions::default())
        .unwrap();
    assert!(
        compact["components"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry.get("source").is_none())
    );
    let catalog = provider
        .call(
            "designer_components",
            &json!({"include_source":true}),
            Permissions::default(),
        )
        .unwrap();
    let brush = catalog["components"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["node"] == accent)
        .unwrap();
    let standalone =
        ferroui_designer_core::Document::new(brush["source"].as_str().unwrap()).unwrap();
    assert!(standalone.is_valid());
    assert_eq!(
        standalone
            .ast()
            .node(standalone.ast().root())
            .unwrap()
            .namespace_uri
            .as_deref(),
        Some("https://github.com/ferroui")
    );
    let references = provider
        .call(
            "designer_component_references",
            &json!({"resource":accent}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(
        references["references"],
        json!([{"node":first,"property":"Background","dynamic":false}])
    );
    assert!(
        provider
            .call(
                "designer_component_resources",
                &json!({"target":u64::MAX}),
                Permissions::default()
            )
            .is_err()
    );

    let themed = r#"<Border xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" RequestedThemeVariant="Dark"><Border.Resources><ResourceDictionary><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key="Dark"><SolidColorBrush x:Key="Accent" Color="Red"/></ResourceDictionary><ResourceDictionary x:Key="Light"><SolidColorBrush x:Key="Accent" Color="Blue"/></ResourceDictionary></ResourceDictionary.ThemeDictionaries></ResourceDictionary></Border.Resources></Border>"#;
    let (session, provider) = fixture(themed);
    let target = session.borrow().document().ast().root();
    let visible = provider
        .call(
            "designer_component_resources",
            &json!({"target":target,"include_source":true}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(visible["context"]["theme"], "Dark");
    let choices = visible["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["key"] == "Accent")
        .collect::<Vec<_>>();
    assert_eq!(choices.len(), 1);
    let dark = choices[0]["node"].as_u64().unwrap();
    assert_eq!(attribute(&session, dark, "Color").as_deref(), Some("Red"));
    edit(
        &session,
        &provider,
        json!({"action":"apply_resource","resource":dark,"targets":[target],"property":"Background","dynamic":true}),
    );
    assert_expression(&session, target, "Background", "{DynamicResource Accent}");
}

#[test]
fn component_property_and_setter_tools_replace_value_forms_and_reset_them() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture(SOURCE);
    let first = named(&session, "First");
    let theme = resource(&session, "Action");
    edit(
        &session,
        &provider,
        json!({"action":"set_property","node":first,"property":"Width","value":"160"}),
    );
    assert_eq!(attribute(&session, first, "Width").as_deref(), Some("160"));
    edit(
        &session,
        &provider,
        json!({"action":"set_property","node":first,"property":"Width","value":null}),
    );
    assert_eq!(attribute(&session, first, "Width"), None);
    edit(
        &session,
        &provider,
        json!({"action":"set_object_property","node":first,"property":"Background","source":"<SolidColorBrush Color=\"Blue\"/>"}),
    );
    assert_eq!(attribute(&session, first, "Background"), None);
    assert!(
        component_property_object(session.borrow().document(), first, "Background")
            .unwrap()
            .unwrap()
            .contains("Blue")
    );
    edit(
        &session,
        &provider,
        json!({"action":"set_object_property","node":first,"property":"Background","source":null}),
    );
    assert!(
        component_property_object(session.borrow().document(), first, "Background")
            .unwrap()
            .is_none()
    );
    edit(
        &session,
        &provider,
        json!({"action":"set_setter_object","owner":theme,"property":"Background","source":"<SolidColorBrush Color=\"Green\"/>"}),
    );
    let setter = session
        .borrow()
        .document()
        .ast()
        .subtree(theme)
        .find(|node| {
            node.local_name() == "Setter" && node.attribute("Property") == Some("Background")
        })
        .unwrap()
        .id;
    assert!(
        component_property_object(session.borrow().document(), setter, "Value")
            .unwrap()
            .unwrap()
            .contains("Green")
    );
    edit(
        &session,
        &provider,
        json!({"action":"set_setter","owner":theme,"property":"Background","value":"Red"}),
    );
    assert_eq!(attribute(&session, setter, "Value").as_deref(), Some("Red"));
    assert!(
        component_property_object(session.borrow().document(), setter, "Value")
            .unwrap()
            .is_none()
    );
    edit(
        &session,
        &provider,
        json!({"action":"set_setter_object","owner":theme,"property":"Background","source":null}),
    );
    assert!(session.borrow().document().ast().node(setter).is_none());
    edit(
        &session,
        &provider,
        json!({"action":"set_setter","owner":theme,"property":"Padding","value":null}),
    );
    assert!(
        !session
            .borrow()
            .document()
            .ast()
            .subtree(theme)
            .any(|node| node.attribute("Property") == Some("Padding"))
    );
}

#[test]
fn component_creation_copy_and_source_tools_use_the_source_preserving_session_planners() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture(SOURCE);
    let owner = session.borrow().document().ast().root();
    let accent = resource(&session, "Accent");
    let theme = resource(&session, "Action");
    edit(
        &session,
        &provider,
        json!({"action":"create_resource","owner":owner,"key":"Caption","source":"<x:String>Old<!-- keep --></x:String>"}),
    );
    let caption = resource(&session, "Caption");
    edit(
        &session,
        &provider,
        json!({"action":"set_resource_value","resource":caption,"value":"A & B"}),
    );
    assert!(
        session
            .borrow()
            .document()
            .source()
            .contains("A &amp; B<!-- keep -->")
    );
    edit(
        &session,
        &provider,
        json!({"action":"duplicate_resource","resource":accent,"key":"OtherAccent"}),
    );
    let duplicate = resource(&session, "OtherAccent");
    assert_eq!(
        attribute(&session, duplicate, "Color").as_deref(),
        Some("Red")
    );
    edit(
        &session,
        &provider,
        json!({"action":"remove_resource","resource":duplicate}),
    );
    assert!(session.borrow().document().ast().node(duplicate).is_none());
    edit(
        &session,
        &provider,
        json!({"action":"replace_definition","node":accent,"source":"<SolidColorBrush x:Key=\"Accent\" Color=\"Green\"/>"}),
    );
    assert_eq!(
        attribute(&session, resource(&session, "Accent"), "Color").as_deref(),
        Some("Green")
    );
    edit(
        &session,
        &provider,
        json!({"action":"create_theme","owner":owner,"key":"Derived","target_type":"Button","based_on":theme}),
    );
    let derived = resource(&session, "Derived");
    assert!(
        attribute(&session, derived, "BasedOn")
            .unwrap()
            .contains("Action")
    );
    edit(
        &session,
        &provider,
        json!({"action":"create_template","owner":owner,"key":"Card","target_type":"Button","content":"<Border Name=\"CardChrome\" Padding=\"4\"/>"}),
    );
    let card = resource(&session, "Card");
    edit(
        &session,
        &provider,
        json!({"action":"copy_template","template":card,"owner":owner,"key":"CardCopy"}),
    );
    assert_ne!(resource(&session, "CardCopy"), card);
    edit(
        &session,
        &provider,
        json!({"action":"create_template","owner":owner,"key":"Row","data_template":true,"content":"<TextBlock Text=\"{Binding}\"/>"}),
    );
    let row = resource(&session, "Row");
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(row)
            .unwrap()
            .local_name(),
        "DataTemplate"
    );
}

#[test]
fn extraction_and_instance_copy_are_atomic_and_restore_both_ends_on_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture(SOURCE);
    let owner = session.borrow().document().ast().root();
    let first = named(&session, "First");
    let second = named(&session, "Second");
    let theme = resource(&session, "Action");
    edit(
        &session,
        &provider,
        json!({"action":"set_property","node":first,"property":"Background","value":"Blue"}),
    );
    let before = session.borrow().document().source().to_owned();
    let revision = session.borrow().document().revision();
    edit(
        &session,
        &provider,
        json!({"action":"extract_property","node":first,"property":"Background","owner":owner,"key":"Extracted"}),
    );
    assert_eq!(session.borrow().document().revision(), revision + 1);
    let extracted = resource(&session, "Extracted");
    assert_eq!(
        attribute(&session, extracted, "Color").as_deref(),
        Some("Blue")
    );
    assert!(
        attribute(&session, first, "Background")
            .unwrap()
            .contains("Extracted")
    );
    call_edit(&session, &provider, "designer_undo", json!({}));
    assert_eq!(session.borrow().document().source(), before);

    let before = session.borrow().document().source().to_owned();
    let revision = session.borrow().document().revision();
    let copied = edit(
        &session,
        &provider,
        json!({"action":"copy_definition_for_instance","definition":theme,"target":first,"property":"Theme","key":"PrivateAction"}),
    );
    assert_eq!(session.borrow().document().revision(), revision + 1);
    assert_eq!(copied["created"], resource(&session, "PrivateAction"));
    assert_expression(&session, first, "Theme", "{StaticResource PrivateAction}");
    assert_eq!(
        attribute(&session, second, "Theme").as_deref(),
        Some("{StaticResource Action}")
    );
    call_edit(&session, &provider, "designer_undo", json!({}));
    assert_eq!(session.borrow().document().source(), before);
    let revision = session.borrow().document().revision();
    assert!(provider.call("designer_component", &json!({"expected_revision":revision,"action":"copy_definition_for_instance","definition":theme,"target":u64::MAX,"property":"Theme","key":"FailedCopy"}), write()).is_err());
    assert_eq!(session.borrow().document().source(), before);
    assert_eq!(session.borrow().document().revision(), revision);
}

#[test]
fn definition_navigation_uses_native_scopes_without_source_edits_and_guards_stale_epochs() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture(SOURCE);
    let first = named(&session, "First");
    let template = resource(&session, "Chrome");
    session.borrow_mut().select([first]);
    let current = provider
        .call(
            "designer_definition_scope",
            &json!({}),
            Permissions::default(),
        )
        .unwrap();
    assert!(current["scope"]["definition"].is_null());
    let epoch = current["scope"]["epoch"].as_u64().unwrap();
    let enter = json!({"expected_revision":0,"expected_scope_epoch":epoch,"action":"enter","node":template,"options":{"target_type":"Button","sample_content":"Example","width":420,"height":220}});
    assert!(
        provider
            .call("designer_definition", &enter, Permissions::default())
            .unwrap_err()
            .contains("allow_property_writes")
    );
    // Viewer roles may inspect native previews; permission to change the UI is separate.
    session.borrow_mut().set_read_only(true);
    let entered = provider
        .call("designer_definition", &enter, write())
        .unwrap();
    assert_eq!(entered["scope"]["definition"]["node"], template);
    assert_eq!(entered["scope"]["preview_options"]["width"], 420.0);
    assert_eq!(
        entered["scope"]["preview_options"]["sample_content"],
        "Example"
    );
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert_eq!(session.borrow().document().revision(), 0);
    assert!(!session.borrow().can_undo());
    assert!(
        provider
            .call("designer_definition", &enter, write())
            .unwrap_err()
            .contains("scope changed")
    );
    let active_epoch = entered["scope"]["epoch"].as_u64().unwrap();
    assert!(
        provider
            .call(
                "designer_definition",
                &json!({"expected_revision":1,"action":"exit"}),
                write()
            )
            .unwrap_err()
            .contains("revision")
    );
    let left = call_edit(
        &session,
        &provider,
        "designer_definition",
        json!({"action":"exit","expected_scope_epoch":active_epoch}),
    );
    assert!(left["scope"]["definition"].is_null());
    assert_eq!(session.borrow().selection(), &[first]);
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert_eq!(session.borrow().document().revision(), 0);
    assert!(!session.borrow().can_undo());
}

#[test]
fn accepted_component_edits_report_catalog_diagnostics_without_false_transaction_failure() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture(SOURCE);
    let ambiguous = SOURCE.replace(
        "<ControlTheme x:Key=\"Action\" TargetType=\"Button\"",
        "<ControlTheme x:Key=\"Action\" TargetType=\"Button\" ControlTheme.TargetType=\"TextBox\"",
    );
    assert!(session.borrow_mut().update_source(&ambiguous).valid);
    assert!(session.borrow().component_catalog().is_err());
    let first = named(&session, "First");
    let revision = session.borrow().document().revision();
    let result = edit(
        &session,
        &provider,
        json!({"action":"set_property","node":first,"property":"Width","value":"160"}),
    );
    assert_eq!(result["document"]["revision"], revision + 1);
    assert!(result["components"].is_null());
    assert!(result["catalog_error"].is_string());
    assert_eq!(attribute(&session, first, "Width").as_deref(), Some("160"));
    call_edit(&session, &provider, "designer_undo", json!({}));
    assert_eq!(session.borrow().document().source(), ambiguous);
}
