use super::*;
use crate::mcp::DesignerMcp;
use crate::{
    PreviewEnvironment, PreviewPrototype, PreviewPrototypeAction, PreviewPrototypeFlow,
    PreviewPrototypeLink, PreviewSourceAnchor, PreviewStageConfiguration,
};
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, TextBox, Window};
use ferroui_mcp::UiToolProvider;
use std::sync::Arc;

const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui" Name="Scene" Spacing="12"><TextBox Name="Note" Text="Initial"/><Button Name="Go" Content="Continue" Width="140" Height="44"/></StackPanel>"#;
const URI: &str = "memory://prototype-mcp.xaml";
fn permissions(write: bool, input: bool) -> Permissions {
    Permissions {
        allow_property_writes: write,
        allow_input: input,
    }
}
struct Fixture {
    workspace: Rc<DesignerWorkspace>,
    provider: DesignerMcp,
    window: Ref<Window>,
    first: u64,
    second: u64,
}
impl Fixture {
    fn new() -> Self {
        let workspace = DesignerWorkspace::with_uri(SOURCE, URI.into()).unwrap();
        workspace.set_timeline_visible(false);
        let window = Window::new();
        window.set_width(1400.0);
        window.set_height(1000.0);
        window.set_content(Some(Control::boxed(workspace.control())));
        window.show();
        let first = workspace
            .add_preview_stage(PreviewStageConfiguration::new(
                "Home",
                URI,
                PreviewEnvironment::default(),
            ))
            .unwrap();
        let second = workspace
            .add_preview_stage(PreviewStageConfiguration::new(
                "Details",
                URI,
                PreviewEnvironment::default(),
            ))
            .unwrap();
        workspace.set_preview_stages_visible(true).unwrap();
        window.update_layout();
        workspace.refresh(true);
        window.update_layout();
        let provider = DesignerMcp::with_workspace(&workspace);
        Self {
            workspace,
            provider,
            window,
            first,
            second,
        }
    }
    fn graph(&self) -> PreviewPrototype {
        let session = self.workspace.session();
        let session = session.borrow();
        let node = session
            .document()
            .ast()
            .nodes()
            .find(|node| node.xaml_name() == Some("Go"))
            .unwrap()
            .id;
        let anchor = PreviewSourceAnchor::capture(session.document(), node).unwrap();
        PreviewPrototype {
            flows: vec![PreviewPrototypeFlow {
                name: "Browse".into(),
                start: self.first,
            }],
            links: vec![
                PreviewPrototypeLink {
                    name: "Open details".into(),
                    source: self.first,
                    trigger: anchor.clone(),
                    action: PreviewPrototypeAction::Navigate { stage: self.second },
                },
                PreviewPrototypeLink {
                    name: "Return".into(),
                    source: self.second,
                    trigger: anchor,
                    action: PreviewPrototypeAction::Back,
                },
            ],
        }
    }
    fn read(&self) -> Value {
        self.provider
            .call("designer_prototype", &json!({}), permissions(false, false))
            .unwrap()
    }
    fn command(&self, mut args: Value, input: bool) -> Result<Value, String> {
        args["expected_model"] = self.read()["model"].clone();
        self.provider
            .call("designer_prototype_action", &args, permissions(true, input))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

#[test]
fn prototype_catalog_is_workspace_only_and_schema_rejects_mixed_or_unbounded_actions() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let session_only = DesignerMcp::new(&fixture.workspace.session());
    assert!(!session_only
        .tools()
        .iter()
        .any(|tool| tool["name"] == "designer_prototype"));
    fixture
        .provider
        .validate(
            "designer_prototype_action",
            &json!({"action":"set_graph","expected_model":"0","graph":fixture.graph()}),
        )
        .unwrap();
    for invalid in [
        json!({"action":"play","flow":"Browse"}),
        json!({"action":"back","expected_model":"0","flow":"Extra"}),
        json!({"action":"activate","expected_model":"0","link":128}),
        json!({"action":"set_graph","expected_model":"0","graph":{"flows":[{"name":"A","start":0}],"links":[]}}),
        json!({"action":"set_graph","expected_model":"0","graph":{"flows":[],"links":[],"script":"ignored"}}),
        json!({"action":"set_graph","expected_model":"0","graph":{"flows":vec![json!({"name":"A","start":fixture.first});33],"links":[]}}),
    ] {
        assert!(
            fixture
                .provider
                .validate("designer_prototype_action", &invalid)
                .is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn graph_reads_are_observational_and_edits_are_atomic_guarded_presentation_for_viewers() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    let workspace = &fixture.workspace;
    workspace.session().borrow_mut().set_read_only(true);
    let document = workspace.session().borrow().document().clone();
    let accepted = fixture.read();
    let epoch = workspace.preview_stage_epoch();
    let control = workspace.preview_stage_control(fixture.first).unwrap();
    assert_eq!(accepted, fixture.read());
    assert_eq!(epoch, workspace.preview_stage_epoch());
    assert!(workspace.preview_prototype_playback().is_none());
    let command =
        json!({"action":"set_graph","expected_model":accepted["model"],"graph":fixture.graph()});
    assert!(fixture
        .provider
        .call(
            "designer_prototype_action",
            &command,
            permissions(false, false)
        )
        .is_err());
    fixture
        .provider
        .call(
            "designer_prototype_action",
            &command,
            permissions(true, false),
        )
        .unwrap();
    assert!(fixture
        .provider
        .call(
            "designer_prototype_action",
            &command,
            permissions(true, false)
        )
        .is_err());
    assert_eq!(
        control,
        workspace.preview_stage_control(fixture.first).unwrap()
    );
    let graph = workspace.preview_prototype();
    let mut invalid = graph.clone();
    invalid.links[0].trigger.path.last_mut().unwrap().name = Some("Removed".into());
    assert!(fixture
        .command(json!({"action":"set_graph","graph":invalid}), false)
        .is_err());
    assert_eq!(workspace.preview_prototype(), graph);
    assert!(Arc::ptr_eq(
        &document.ast_snapshot(),
        &workspace.session().borrow().document().ast_snapshot()
    ));
    assert_eq!(
        document.source(),
        workspace.session().borrow().document().source()
    );
    assert!(!workspace.session().borrow().can_undo());
}

#[test]
fn playback_requires_input_and_observational_reports_preserve_native_state_and_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    fixture
        .command(json!({"action":"set_graph","graph":fixture.graph()}), false)
        .unwrap();
    let before = fixture.read();
    assert!(fixture
        .command(json!({"action":"play","flow":"Browse"}), false)
        .unwrap_err()
        .contains("allow_input"));
    assert_eq!(before, fixture.read());
    fixture
        .command(json!({"action":"play","flow":"Browse"}), true)
        .unwrap();
    fixture.window.update_layout();
    fixture.workspace.refresh(true);
    fixture.window.update_layout();
    let home = fixture
        .workspace
        .preview_prototype_control(fixture.first)
        .unwrap();
    let note = home
        .get_logical_descendants()
        .find(|control| control.name().as_deref() == Some("Note"))
        .and_then(|control| control.cast::<TextBox>())
        .unwrap();
    note.set_text(Some("Retained player text"));
    fixture.window.update_layout();
    fixture.workspace.refresh(false);
    fixture.window.update_layout();
    let running = fixture.read();
    assert_eq!(running, fixture.read());
    assert_eq!(note.text().as_deref(), Some("Retained player text"));
    let stale = json!({"action":"activate","link":0,"expected_model":running["model"]});
    fixture
        .provider
        .call("designer_prototype_action", &stale, permissions(true, true))
        .unwrap();
    assert!(fixture
        .provider
        .call("designer_prototype_action", &stale, permissions(true, true))
        .is_err());
    assert_eq!(fixture.read()["playback"]["current"], fixture.second);
    assert_eq!(
        fixture.read()["playback"]["history"],
        json!([fixture.first])
    );
    assert!(fixture.command(json!({"action":"back"}), false).is_err());
    fixture.command(json!({"action":"back"}), true).unwrap();
    assert_eq!(fixture.read()["playback"]["current"], fixture.first);
    assert_eq!(note.text().as_deref(), Some("Retained player text"));
    fixture.command(json!({"action":"stop"}), false).unwrap();
    assert!(fixture.workspace.preview_prototype_playback().is_none());
}

#[test]
fn mcp_manifest_v2_restores_graph_to_fresh_ids_without_authoring_or_starting_player() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new();
    fixture
        .command(json!({"action":"set_graph","graph":fixture.graph()}), false)
        .unwrap();
    let manifest = fixture.workspace.preview_stage_manifest();
    assert_eq!(manifest.version, 2);
    let command = json!({"action":"restore","expected_model":fixture.workspace.preview_stage_model(),"manifest":manifest});
    fixture
        .provider
        .call("designer_preview_stage", &command, permissions(true, false))
        .unwrap();
    let graph = fixture.workspace.preview_prototype();
    assert_ne!(graph.flows[0].start, fixture.first);
    assert_eq!(
        graph.flows[0].start,
        fixture.workspace.preview_stages()[0].id
    );
    assert_eq!(fixture.workspace.preview_stage_manifest(), manifest);
    assert!(fixture.workspace.preview_prototype_playback().is_none());
    assert!(!fixture.workspace.session().borrow().can_undo());
}
