//! End-to-end extension calls use the real UI-thread provider and document/session models.
use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, ContentControl, Control, Window};
use ferroui_designer_animation::Timeline;
use ferroui_designer_tools::{PropertyDescriptor, PropertyKind};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn fixture() -> (Rc<RefCell<DesignerSession>>, DesignerMcp) {
    let session = Rc::new(RefCell::new(DesignerSession::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Width="100" Height="40" /></Canvas>"#,
        ContentControl::new(),
    ).unwrap()));
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

#[test]
fn new_authoring_schemas_reject_unknown_nested_fields_and_stale_writes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture();
    let owner = session.borrow().document().ast().root();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    let create = json!({"action":"create","expected_revision":0,"name":"Entrance","owner":owner,"selector":"Button#Target","duration":1.0});
    assert!(provider
        .call("designer_program", &create, Permissions::default())
        .unwrap_err()
        .contains("allow_property_writes"));
    let mut wrong_action = create.clone();
    wrong_action["program"] = json!(target);
    assert!(provider
        .validate("designer_program", &wrong_action)
        .is_err());
    let bad_behavior = json!({"action":"upsert","expected_revision":0,"model":{"owner":target,"trigger":{"Event":{"event":"Click","source":null,"unknown":true}},"actions":[]}});
    assert!(provider
        .validate("designer_behavior", &bad_behavior)
        .is_err());
    assert_eq!(session.borrow().document().revision(), 0);
    provider.call("designer_program", &create, write()).unwrap();
    assert!(provider
        .call("designer_program", &create, write())
        .unwrap_err()
        .contains("revision"));
    assert_eq!(session.borrow().animation_programs().len(), 1);
}

#[test]
fn independent_program_tools_preserve_identity_and_drive_the_native_preview() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture();
    let owner = session.borrow().document().ast().root();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    let created = call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"create","name":"Resize","owner":owner,"selector":"Button#Target","duration":1.0}),
    );
    let id = created["created"].as_u64().unwrap();
    let source_key = created["programs"][0]["source_key"].clone();
    assert!(source_key.is_string());
    let mut timeline = Timeline::new(1.0).unwrap();
    timeline
        .record(target, "Width", 0.0, AnimationValue::Number(100.0))
        .unwrap();
    timeline
        .record(target, "Width", 1.0, AnimationValue::Number(300.0))
        .unwrap();
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"update","program":id,"timeline":timeline}),
    );
    let authored = session.borrow().document().source().to_owned();
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"preview_targets","program":id,"targets":[target]}),
    );
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"preview","programs":[id],"time":0.5}),
    );
    assert_eq!(session.borrow().document().source(), authored);
    assert!(
        (session
            .borrow()
            .preview
            .visual(target)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width()
            - 200.0)
            .abs()
            < 1e-9
    );
    let duplicate = call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"duplicate","program":id,"name":"Other"}),
    )["created"]
        .as_u64()
        .unwrap();
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"rename","program":duplicate,"name":"Other renamed"}),
    );
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"selector","program":duplicate,"selector":"Button#Target"}),
    );
    let list = provider
        .call("designer_programs", &json!({}), Permissions::default())
        .unwrap();
    assert_eq!(list["programs"].as_array().unwrap().len(), 2);
    let original = list["programs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|program| program["id"] == id)
        .unwrap();
    assert_eq!(original["source_key"], source_key);
    let copied = list["programs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|program| program["id"] == duplicate)
        .unwrap();
    assert_ne!(copied["source_key"], source_key);
    assert_eq!(copied["name"], "Other renamed");
    let edited_source =
        copied["source"]
            .as_str()
            .unwrap()
            .replacen('>', "><!-- agent metadata -->", 1);
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"source","program":duplicate,"source":edited_source}),
    );
    assert!(session
        .borrow()
        .document()
        .source()
        .contains("agent metadata"));
    let before = session.borrow().document().source().to_owned();
    let revision = session.borrow().document().revision();
    assert!(provider.call("designer_program",&json!({"expected_revision":revision,"action":"source","program":id,"source":"<Animation"}),write()).is_err());
    assert_eq!(session.borrow().document().source(), before);
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"select","program":id}),
    );
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"remove","program":duplicate}),
    );
    assert_eq!(session.borrow().animation_programs().len(), 1);
    call_edit(&session, &provider, "designer_undo", json!({}));
    assert_eq!(session.borrow().animation_programs().len(), 2);
    let compact = provider
        .call(
            "designer_programs",
            &json!({"include_source":false}),
            Permissions::default(),
        )
        .unwrap();
    assert!(compact["programs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|program| program.get("source").is_none()));
}

#[test]
fn dynamic_property_schemas_validate_host_metadata_and_tagged_values_before_writes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    session.borrow_mut().property_tools_mut().register(
        "Button",
        PropertyDescriptor::new(
            "Width",
            "Host constraints",
            PropertyKind::Number {
                minimum: Some(50.0),
                maximum: Some(250.0),
                allow_auto: false,
            },
        ),
    );
    let metadata = provider
        .call(
            "designer_properties",
            &json!({"node":target}),
            Permissions::default(),
        )
        .unwrap();
    let width = metadata["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|property| property["descriptor"]["name"] == "Width")
        .unwrap();
    assert_eq!(width["descriptor"]["category"], "Host constraints");
    assert!(schema::validate(
        &width["value_schema"],
        &json!({"kind":"Number","value":240})
    )
    .is_ok());
    assert!(schema::validate(
        &width["value_schema"],
        &json!({"kind":"Number","value":260})
    )
    .is_err());
    for value in [
        json!(260),
        json!({"kind":"Number","value":260}),
        json!(true),
    ] {
        assert!(provider.call("designer_property",&json!({"expected_revision":0,"action":"set","nodes":[target],"property":"Width","value":value}),write()).is_err());
    }
    assert_eq!(session.borrow().document().revision(), 0);
    call_edit(
        &session,
        &provider,
        "designer_property",
        json!({"action":"set","nodes":[target],"property":"Width","value":{"kind":"Number","value":240}}),
    );
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Width"),
        Some("240")
    );
    call_edit(
        &session,
        &provider,
        "designer_property",
        json!({"action":"set","nodes":[target],"property":"Background","value":{"kind":"Color","value":{"a":1,"r":1,"g":0,"b":0}}}),
    );
    call_edit(
        &session,
        &provider,
        "designer_property",
        json!({"action":"set","nodes":[target],"property":"RenderTransformOrigin","value":{"kind":"Point","value":{"x":0.5,"y":0.5}}}),
    );
    let revision = session.borrow().document().revision();
    assert!(provider.call("designer_property",&json!({"expected_revision":revision,"action":"set","nodes":[target],"property":"Background","value":{"kind":"Color","value":{"a":1,"r":2,"g":0,"b":0}}}),write()).is_err());
    assert!(provider.call("designer_property",&json!({"expected_revision":revision,"action":"reset","nodes":[target,target],"property":"Width"}),write()).is_err());
    let mut locked = PropertyDescriptor::new("Width", "Host constraints", PropertyKind::Text);
    locked.read_only = true;
    session
        .borrow_mut()
        .property_tools_mut()
        .register("Button", locked);
    assert!(provider.call("designer_property",&json!({"expected_revision":revision,"action":"reset","nodes":[target],"property":"Width"}),write()).unwrap_err().contains("read-only"));
    assert_eq!(session.borrow().document().revision(), revision);
}

#[test]
fn geometry_and_spring_values_use_strict_schemas_and_native_authoring_models() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    let values = [
        json!({"kind":"Vector","value":{"x":-2.0,"y":4.0}}),
        json!({"kind":"Size","value":{"width":20.0,"height":40.0}}),
        json!({"kind":"Rect","value":{"x":-2.0,"y":4.0,"width":20.0,"height":40.0}}),
        json!({"kind":"CornerRadius","value":{"top_left":1.0,"top_right":2.0,"bottom_right":3.0,"bottom_left":4.0}}),
        json!({"kind":"RelativePoint","value":{"x":0.25,"y":0.75,"unit":"Relative"}}),
        json!({"kind":"RelativeScalar","value":{"scalar":0.5,"unit":"Relative"}}),
    ];
    let metadata = provider
        .call(
            "designer_properties",
            &json!({"node":target}),
            Permissions::default(),
        )
        .unwrap();
    let origin_schema = &metadata["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|property| property["descriptor"]["name"] == "RenderTransformOrigin")
        .unwrap()["value_schema"];
    for value in &values {
        schema::validate(origin_schema, value).unwrap();
        let decoded: AnimationValue = serde_json::from_value(value.clone()).unwrap();
        decoded.validate().unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), *value);
        let mut unexpected = value.clone();
        unexpected["value"]["unrecognized"] = json!(1);
        assert!(schema::validate(&schema::animation_value(), &unexpected).is_err());
    }
    for value in [
        json!({"kind":"RelativePoint","value":{"x":0.5,"y":0.5,"unit":"Percent"}}),
        json!({"kind":"Rect","value":{"x":0.0,"y":0.0,"width":20.0}}),
        json!({"kind":"RelativeScalar","value":{"scalar":0.5,"unit":"Relative","x":0.5}}),
    ] {
        assert!(schema::validate(&schema::animation_value(), &value).is_err());
    }
    call_edit(
        &session,
        &provider,
        "designer_property",
        json!({"action":"set","nodes":[target],"property":"RenderTransformOrigin","value":values[4]}),
    );
    call_edit(
        &session,
        &provider,
        "designer_property",
        json!({"action":"set","nodes":[target],"property":"CornerRadius","value":values[3]}),
    );
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("RenderTransformOrigin"),
        Some("25%,75%")
    );
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("CornerRadius"),
        Some("1,2,3,4")
    );

    let spring = json!({"kind":"Spring","value":{"mass":1.0,"stiffness":100.0,"damping":10.0,"initial_velocity":-0.5}});
    schema::validate(&schema::easing(), &spring).unwrap();
    for (field, value) in [("mass", 0.0), ("stiffness", -1.0), ("damping", -1.0)] {
        let mut invalid = spring.clone();
        invalid["value"][field] = json!(value);
        assert!(schema::validate(&schema::easing(), &invalid).is_err());
    }
    let owner = session.borrow().document().ast().root();
    let id = call_edit(&session, &provider, "designer_program", json!({"action":"create","name":"Spring","owner":owner,"selector":"Button#Target","duration":1.0}))["created"].as_u64().unwrap();
    let mut timeline = Timeline::new(1.0).unwrap();
    timeline
        .record(target, "Opacity", 0.0, AnimationValue::Number(1.0))
        .unwrap();
    timeline
        .record(target, "Opacity", 1.0, AnimationValue::Number(0.5))
        .unwrap();
    let mut timeline = serde_json::to_value(timeline).unwrap();
    timeline["easing"] = spring.clone();
    call_edit(
        &session,
        &provider,
        "designer_program",
        json!({"action":"update","program":id,"timeline":timeline}),
    );
    let programs = provider
        .call("designer_programs", &json!({}), Permissions::default())
        .unwrap();
    assert_eq!(programs["programs"][0]["timeline"]["easing"], spring);
    assert!(session
        .borrow()
        .document()
        .source()
        .contains("SpringEasing"));
}

#[test]
fn visual_state_and_behavior_tools_use_native_models_and_explicit_execution_permission() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (session, provider) = fixture();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&session.borrow().preview.host)));
    window.show();
    window.update_layout();
    let owner = session.borrow().document().ast().root();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    let model = json!({"owner":owner,"name":"Interaction","states":[{"name":"Normal","setters":[{"target":"Target","property":"Opacity","value":"1"}]},{"name":"Muted","setters":[{"target":"Target","property":"Opacity","value":"0.25"}]}],"transitions":[{"from":null,"to":"Muted","duration":0.4,"easing":{"kind":"Linear"}}]});
    call_edit(
        &session,
        &provider,
        "designer_state",
        json!({"action":"upsert","model":model}),
    );
    assert_eq!(
        provider
            .call(
                "designer_states",
                &json!({"owner":owner}),
                Permissions::default()
            )
            .unwrap()["groups"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let source = session.borrow().document().source().to_owned();
    call_edit(
        &session,
        &provider,
        "designer_state",
        json!({"action":"preview","owner":owner,"group":"Interaction","state":"Muted"}),
    );
    assert!((session.borrow().preview.visual(target).unwrap().opacity() - 0.25).abs() < 1e-9);
    assert_eq!(session.borrow().document().source(), source);
    let actions = provider
        .call(
            "designer_state_actions",
            &json!({"owner":owner,"group":"Interaction","to":"Muted"}),
            Permissions::default(),
        )
        .unwrap();
    assert!(!actions["actions"].as_array().unwrap().is_empty());
    call_edit(
        &session,
        &provider,
        "designer_state",
        json!({"action":"clear_preview"}),
    );
    let model = json!({"owner":owner,"trigger":{"Event":{"event":"PointerPressed","source":null}},"actions":[{"ChangeProperty":{"target":"Target","property":"Width","value":"275"}}]});
    let behavior = call_edit(
        &session,
        &provider,
        "designer_behavior",
        json!({"action":"upsert","model":model}),
    )["behavior"]
        .as_u64()
        .unwrap();
    assert_eq!(
        provider
            .call(
                "designer_behaviors",
                &json!({"owner":owner}),
                Permissions::default()
            )
            .unwrap()["behaviors"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let revision = session.borrow().document().revision();
    let preview = json!({"expected_revision":revision,"action":"preview","behavior":behavior});
    assert!(provider
        .call("designer_behavior", &preview, write())
        .unwrap_err()
        .contains("allow_input"));
    assert_eq!(
        session
            .borrow()
            .preview
            .visual(target)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        100.0
    );
    provider
        .call(
            "designer_behavior",
            &preview,
            Permissions {
                allow_property_writes: true,
                allow_input: true,
            },
        )
        .unwrap();
    assert_eq!(
        session
            .borrow()
            .preview
            .visual(target)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        275.0
    );
    assert_eq!(session.borrow().document().revision(), revision);
    call_edit(
        &session,
        &provider,
        "designer_behavior",
        json!({"action":"clear_preview"}),
    );
    assert_eq!(
        session
            .borrow()
            .preview
            .visual(target)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        100.0
    );
    call_edit(
        &session,
        &provider,
        "designer_behavior",
        json!({"action":"remove","behavior":behavior}),
    );
    call_edit(
        &session,
        &provider,
        "designer_state",
        json!({"action":"remove","owner":owner,"group":"Interaction"}),
    );
    assert!(session.borrow().behaviors(owner).is_empty());
    assert!(session.borrow().state_groups().is_empty());
    window.close();
}

struct Files(PathBuf);
impl Files {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ferroui-designer-mcp-project-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
    fn write(&self, name: &str, source: &str) -> String {
        let path = self.0.join(name);
        std::fs::write(&path, source).unwrap();
        ferroui_designer_project::file_uri(&path).unwrap()
    }
}
impl Drop for Files {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn workspace_catalog_is_optional_weak_and_reentrant_calls_fail_without_panicking() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace =
        DesignerWorkspace::new(r#"<Button xmlns="https://github.com/ferroui" Name="Target"/>"#)
            .unwrap();
    let session = workspace.session();
    let provider = DesignerMcp::with_workspace(&workspace);
    let only_session = DesignerMcp::new(&session);
    assert!(provider
        .tools()
        .iter()
        .any(|tool| tool["name"] == "designer_language"));
    assert!(!only_session
        .tools()
        .iter()
        .any(|tool| tool["name"] == "designer_language"));
    let session_borrow = session.borrow();
    assert!(provider
        .call("designer_project", &json!({}), Permissions::default())
        .unwrap_err()
        .contains("processing a transaction"));
    drop(session_borrow);
    let language_borrow = workspace.language_service();
    assert!(provider
        .call(
            "designer_language",
            &json!({"action":"diagnostics"}),
            Permissions::default()
        )
        .unwrap_err()
        .contains("busy"));
    assert!(provider
        .call(
            "designer_project_edit",
            &json!({"expected_revision":0,"action":"save_all"}),
            write()
        )
        .unwrap_err()
        .contains("busy"));
    drop(language_borrow);
    session.borrow_mut().set_read_only(true);
    assert!(provider
        .call(
            "designer_project_edit",
            &json!({"expected_revision":0,"action":"save_all"}),
            write()
        )
        .unwrap_err()
        .contains("read-only"));
    assert!(provider
        .call(
            "designer_language",
            &json!({"action":"diagnostics"}),
            Permissions::default()
        )
        .is_ok());
    drop(workspace);
    assert!(provider
        .call("designer_project", &json!({}), Permissions::default())
        .unwrap_err()
        .contains("closed"));
    assert!(provider
        .call("designer_document", &json!({}), Permissions::default())
        .is_ok());
    drop(session);
    assert!(only_session
        .call("designer_document", &json!({}), Permissions::default())
        .unwrap_err()
        .contains("closed"));
}

#[test]
fn language_tools_rename_across_project_tabs_atomically_with_version_and_revision_guards() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files = Files::new();
    let resources = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><SolidColorBrush x:Key="Accent" Color="Red"/></ResourceDictionary>"#;
    let usage = r#"<Button xmlns="https://github.com/ferroui" Name="Target" Content="🦀" Background="{StaticResource Accent}"/>"#;
    let resource_uri = files.write("Resources.xaml", resources);
    let usage_uri = files.write("View.xaml", usage);
    let workspace = DesignerWorkspace::with_uri(usage, usage_uri.clone()).unwrap();
    workspace
        .set_solution(ferroui_designer_project::Solution::discover(&files.0).unwrap())
        .unwrap();
    let provider = DesignerMcp::with_workspace(&workspace);
    let session = workspace.session();
    let offset = usage.find("Accent").unwrap();
    let definitions = provider
        .call(
            "designer_language",
            &json!({"action":"definitions","offset":offset}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(definitions["result"][0]["uri"], resource_uri);
    let rename = provider
        .call(
            "designer_language",
            &json!({"action":"rename","offset":offset,"new_name":"Primary"}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(
        rename["result"]["document_changes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let mut invalid = rename["result"].clone();
    invalid["document_changes"][1]["version"] = json!(9999);
    let revision = session.borrow().document().revision();
    assert!(provider
        .call(
            "designer_language_apply",
            &json!({"expected_revision":revision,"edit":invalid}),
            write()
        )
        .is_err());
    assert_eq!(session.borrow().document().source(), usage);
    assert_eq!(workspace.document_uris(), vec![usage_uri.clone()]);
    call_edit(
        &session,
        &provider,
        "designer_language_apply",
        json!({"edit":rename["result"]}),
    );
    assert!(session
        .borrow()
        .document()
        .source()
        .contains("StaticResource Primary"));
    assert_eq!(
        std::fs::read_to_string(files.0.join("Resources.xaml")).unwrap(),
        resources
    );
    assert_eq!(
        std::fs::read_to_string(files.0.join("View.xaml")).unwrap(),
        usage
    );
    assert!(provider
        .call(
            "designer_language_apply",
            &json!({"expected_revision":revision,"edit":rename["result"]}),
            write()
        )
        .unwrap_err()
        .contains("revision"));
    call_edit(
        &session,
        &provider,
        "designer_project_edit",
        json!({"action":"activate","uri":resource_uri}),
    );
    assert!(session
        .borrow()
        .document()
        .source()
        .contains("x:Key=\"Primary\""));
    assert!(session.borrow().can_undo());
    let revision = session.borrow().document().revision();
    assert!(provider.call("designer_project_edit",&json!({"expected_revision":revision,"action":"close","uri":resource_uri,"discard":false}),write()).is_err());
    call_edit(
        &session,
        &provider,
        "designer_project_edit",
        json!({"action":"save_all"}),
    );
    assert!(std::fs::read_to_string(files.0.join("Resources.xaml"))
        .unwrap()
        .contains("x:Key=\"Primary\""));
    assert!(std::fs::read_to_string(files.0.join("View.xaml"))
        .unwrap()
        .contains("StaticResource Primary"));
    call_edit(
        &session,
        &provider,
        "designer_project_edit",
        json!({"action":"close","uri":resource_uri,"discard":false}),
    );
    assert_eq!(session.borrow().document_uri(), usage_uri);
    assert!(session.borrow().can_undo());
}

#[test]
fn language_queries_share_editor_schema_and_build_plans_never_start_processes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let files = Files::new();
    let source = r#"<Button xmlns="https://github.com/ferroui" Name="Target" Width="100"/>"#;
    let uri = files.write("View.xaml", source);
    let mut solution = ferroui_designer_project::Solution::discover(&files.0).unwrap();
    // Construct a host project descriptor without invoking Cargo during the native test.
    std::fs::write(
        files.0.join("Cargo.toml"),
        "[package]\nname='designer_mcp_fixture'\nversion='0.0.0'\n",
    )
    .unwrap();
    solution.projects[0]
        .targets
        .push(ferroui_designer_project::CargoTarget {
            name: "demo".into(),
            kind: vec!["bin".into()],
            source: files.0.join("main.rs"),
        });
    let workspace = DesignerWorkspace::with_uri(source, uri).unwrap();
    workspace.set_solution(solution).unwrap();
    let provider = DesignerMcp::with_workspace(&workspace);
    let session = workspace.session();
    for args in [
        json!({"action":"diagnostics"}),
        json!({"action":"symbols"}),
        json!({"action":"folding"}),
        json!({"action":"semantic_tokens"}),
        json!({"action":"completion","offset":7}),
        json!({"action":"hover","offset":3}),
        json!({"action":"references","offset":source.find("Target").unwrap()}),
        json!({"action":"signature","offset":0}),
        json!({"action":"format","options":{"tabSize":2}}),
        json!({"action":"workspace_symbols","query":"Target"}),
        json!({"action":"resource_uri","reference":"Resources.xaml"}),
    ] {
        provider
            .call("designer_language", &args, Permissions::default())
            .unwrap();
    }
    let plan = provider
        .call(
            "designer_build_plan",
            &json!({"action":"run","target":"demo","release":true}),
            Permissions::default(),
        )
        .unwrap();
    assert_eq!(plan["task"]["program"], "cargo");
    let args = plan["task"]["args"].as_array().unwrap();
    assert_eq!(args[0], "run");
    assert!(
        args.contains(&json!("--bin"))
            && args.contains(&json!("demo"))
            && args.contains(&json!("--release"))
    );
    assert!(provider
        .call(
            "designer_build_plan",
            &json!({"action":"run","target":"demo; touch forbidden"}),
            Permissions::default()
        )
        .is_err());
    let revision = session.borrow().document().revision();
    assert!(provider
        .call(
            "designer_project_task",
            &json!({"expected_revision":revision,"action":"build"}),
            write()
        )
        .unwrap_err()
        .contains("allow_input"));
    assert!(workspace.project_task_status().is_none());
    assert!(!session.borrow().document().is_dirty());
}
