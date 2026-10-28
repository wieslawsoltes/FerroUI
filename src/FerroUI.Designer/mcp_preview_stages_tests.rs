use super::*;
use crate::mcp::DesignerMcp;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_mcp::UiToolProvider;
use std::sync::Arc;

const SOURCE: &str =
    r#"<Grid xmlns="https://github.com/ferroui"><Button Name="Action" Content="Preview"/></Grid>"#;
const URI: &str = "ferres://FerroUI.Designer/Stages.xaml";

fn fixture() -> (Rc<DesignerWorkspace>, DesignerMcp) {
    let workspace = DesignerWorkspace::with_uri(SOURCE, URI.into()).unwrap();
    let provider = DesignerMcp::with_workspace(&workspace);
    (workspace, provider)
}
fn permissions(write: bool, input: bool) -> Permissions {
    Permissions {
        allow_property_writes: write,
        allow_input: input,
    }
}
fn config(name: &str) -> PreviewStageConfiguration {
    PreviewStageConfiguration::new(name, URI, PreviewEnvironment::default())
}
fn read(provider: &DesignerMcp) -> Value {
    provider
        .call(
            "designer_preview_stages",
            &json!({}),
            permissions(false, false),
        )
        .unwrap()
}
fn command(provider: &DesignerMcp, mut args: Value) -> Result<Value, String> {
    args["expected_model"] = read(provider)["model"].clone();
    provider.call("designer_preview_stage", &args, permissions(true, false))
}

#[test]
fn stage_tools_are_workspace_only_and_reject_unbounded_or_mixed_schema_inputs() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, provider) = fixture();
    let session_only = DesignerMcp::new(&workspace.session());
    assert!(!session_only
        .tools()
        .iter()
        .any(|tool| tool["name"] == "designer_preview_stages"));
    let valid =
        json!({"action":"add","expected_model":"0000000000000000","configuration":config("Phone")});
    provider.validate("designer_preview_stage", &valid).unwrap();
    for invalid in [
        json!({"action":"remove","stage":1}),
        json!({"action":"remove","stage":1,"expected_model":"0","configuration":config("Extra")}),
        json!({"action":"move","stage":1,"expected_model":"0","index":16}),
        json!({"action":"restore","expected_model":"0","manifest":{"version":2,"stages":[],"active":null}}),
    ] {
        assert!(provider
            .validate("designer_preview_stage", &invalid)
            .is_err());
    }
    for (field, value) in [
        ("width", json!(0)),
        ("height", json!(20000)),
        ("theme", json!("device")),
        ("ignored", json!(true)),
    ] {
        let mut invalid = valid.clone();
        invalid["configuration"]["environment"][field] = value;
        assert!(
            provider
                .validate("designer_preview_stage", &invalid)
                .is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn stage_reads_have_no_native_or_authoring_side_effects_and_viewers_can_configure_local_previews() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, provider) = fixture();
    let ast = workspace.session().borrow().document().ast_snapshot();
    let first = read(&provider);
    assert_eq!(first, read(&provider));
    assert_eq!(first["stages"], json!([]));
    assert!(!workspace.preview_stages_visible());
    let args =
        json!({"action":"add","expected_model":first["model"],"configuration":config("Phone")});
    assert!(provider
        .call("designer_preview_stage", &args, permissions(false, false))
        .is_err());
    workspace.session().borrow_mut().set_read_only(true);
    let result = provider
        .call("designer_preview_stage", &args, permissions(true, false))
        .unwrap();
    assert!(result["created_stage"].as_u64().is_some());
    let session = workspace.session();
    let session = session.borrow();
    assert_eq!(session.document().source(), SOURCE);
    assert_eq!(session.document().revision(), 0);
    assert!(Arc::ptr_eq(&ast, &session.document().ast_snapshot()));
    assert!(!session.can_undo());
    assert!(session.is_read_only());
}

#[test]
fn stage_models_retire_after_source_replacement_even_before_workspace_refresh() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, provider) = fixture();
    let added = command(
        &provider,
        json!({"action":"add","configuration":config("Phone")}),
    )
    .unwrap();
    let id = added["created_stage"].as_u64().unwrap();
    let old_model = read(&provider)["model"].clone();
    workspace
        .session()
        .borrow_mut()
        .update_source(SOURCE.replace("Preview", "Changed"));
    assert_ne!(old_model, read(&provider)["model"]);
    let stale = provider
        .call(
            "designer_preview_stages",
            &json!({"stage":id}),
            permissions(false, false),
        )
        .unwrap();
    assert_eq!(stale["stages"][0]["current_source"], false);
    assert!(stale["tree"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["source_id"].is_null()));
    assert!(provider
        .call(
            "designer_preview_stage",
            &json!({"action":"remove","stage":id,"expected_model":old_model}),
            permissions(true, false)
        )
        .is_err());
    assert_eq!(workspace.preview_stages().len(), 1);
    command(&provider, json!({"action":"remove","stage":id})).unwrap();
    assert!(workspace.preview_stages().is_empty());
    workspace.design_surface().set_interactive(true);
    assert!(command(&provider, json!({"action":"show","visible":true}))
        .unwrap_err()
        .contains("allow_input"));
    assert!(workspace.preview_stages().is_empty());
}

#[test]
fn live_stage_construction_focus_and_refresh_require_native_input_permission() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, provider) = fixture();
    let mut live = config("Interactive");
    live.mode = PreviewStageMode::Live;
    let args =
        json!({"action":"add","expected_model":read(&provider)["model"],"configuration":live});
    assert!(provider
        .call("designer_preview_stage", &args, permissions(true, false))
        .unwrap_err()
        .contains("allow_input"));
    assert!(workspace.preview_stages().is_empty());
    let added = provider
        .call("designer_preview_stage", &args, permissions(true, true))
        .unwrap();
    let id = added["created_stage"].as_u64().unwrap();
    for action in ["refresh", "focus", "duplicate"] {
        assert!(command(&provider, json!({"action":action,"stage":id}))
            .unwrap_err()
            .contains("allow_input"));
    }
    assert!(command(
        &provider,
        json!({"action":"restore","manifest":workspace.preview_stage_manifest()})
    )
    .unwrap_err()
    .contains("allow_input"));
    command(&provider, json!({"action":"remove","stage":id})).unwrap();
    assert!(workspace.preview_stages().is_empty());
}

#[test]
fn compare_and_manifest_restore_share_native_validation_and_preserve_source_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, provider) = fixture();
    let added = command(
        &provider,
        json!({"action":"add","configuration":config("Original")}),
    )
    .unwrap();
    let id = added["created_stage"].as_u64().unwrap();
    let environment = PreviewEnvironment::default();
    command(
        &provider,
        json!({"action":"compare","stage":id,"variants":[
            {"name":"Portrait","environment":environment},
            {"name":"Landscape","environment":environment.rotated()}
        ]}),
    )
    .unwrap();
    assert_eq!(workspace.preview_stages().len(), 3);
    let saved = workspace.preview_stage_manifest();
    let before = workspace
        .preview_stages()
        .iter()
        .map(|stage| stage.id)
        .collect::<Vec<_>>();
    command(&provider, json!({"action":"restore","manifest":saved})).unwrap();
    assert_eq!(workspace.preview_stage_manifest(), saved);
    assert!(workspace
        .preview_stages()
        .iter()
        .all(|stage| !before.contains(&stage.id)));
    assert_eq!(workspace.session().borrow().document().source(), SOURCE);
    assert!(!workspace.session().borrow().can_undo());
    assert!(provider
        .call(
            "designer_preview_stages",
            &json!({"tree":"logical"}),
            permissions(false, false)
        )
        .is_err());
}
