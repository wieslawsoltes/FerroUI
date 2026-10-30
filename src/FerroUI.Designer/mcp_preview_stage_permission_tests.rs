use super::*;
use crate::mcp::DesignerMcp;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_mcp::UiToolProvider;

#[test]
fn presentation_actions_cannot_refresh_another_live_stage_without_input_permission() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Button xmlns="https://github.com/ferroui" Content="Initial"/>"#;
    let workspace =
        DesignerWorkspace::with_uri(source, "memory://stages/Guard.xaml".into()).unwrap();
    let mut config = PreviewStageConfiguration::new(
        "Live",
        "memory://stages/Guard.xaml",
        PreviewEnvironment::default(),
    );
    config.mode = PreviewStageMode::Live;
    let live = workspace.add_preview_stage(config.clone()).unwrap();
    config.mode = PreviewStageMode::Select;
    config.name = "Select".into();
    let select = workspace.add_preview_stage(config).unwrap();
    let epoch = workspace.preview_stage(live).unwrap().epoch;
    workspace
        .session()
        .borrow_mut()
        .update_source(source.replace("Initial", "Updated"));
    let provider = DesignerMcp::with_workspace(&workspace);
    let readonly = Permissions {
        allow_property_writes: false,
        allow_input: false,
    };
    let writer = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    let report = provider
        .call("designer_preview_stages", &json!({}), readonly)
        .unwrap();
    for mut args in [
        json!({"action":"remove","stage":select}),
        json!({"action":"move","stage":select,"index":0}),
        json!({"action":"show","visible":false}),
    ] {
        args["expected_model"] = report["model"].clone();
        assert!(provider
            .call("designer_preview_stage", &args, writer)
            .unwrap_err()
            .contains("allow_input"));
        assert_eq!(workspace.preview_stage(live).unwrap().epoch, epoch);
        assert_eq!(workspace.preview_stages().len(), 2);
    }
}
