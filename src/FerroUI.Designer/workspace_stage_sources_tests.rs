use super::*;
use crate::{PreviewSourceAnchor, PreviewStageState, PreviewStageStatus};
use ferroui_base::media::FlowDirection;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Control, Window};
use ferroui_designer_animation::{StateSetter, VisualState};
use ferroui_designer_core::{Document, Edit};

const FIRST: &str = "memory://preview-stages/First.xaml";
const SECOND: &str = "memory://preview-stages/Second.xaml";
const FIRST_SOURCE: &str = r#"<Grid xmlns="https://github.com/ferroui"><Button Name="Pick" Content="First" Width="80"/></Grid>"#;
const SECOND_SOURCE: &str = r#"<Grid xmlns="https://github.com/ferroui"><Button Name="Pick" Content="Second" Width="200"/></Grid>"#;

fn attach(workspace: &Rc<DesignerWorkspace>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(1800.0);
    window.set_height(1100.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    window
}

fn configuration(name: &str, uri: &str) -> PreviewStageConfiguration {
    let mut config = PreviewStageConfiguration::new(name, uri, PreviewEnvironment::default());
    config.pinned = true;
    config
}

fn selection(
    workspace: &DesignerWorkspace,
    stage: PreviewStageId,
    node: NodeId,
) -> PreviewStageSelection {
    workspace
        .preview_stages
        .board
        .borrow_mut()
        .refresh_runtime_mappings();
    let board = workspace.preview_stages.board.borrow();
    let row = board
        .runtime_tree(stage, TreeKind::Visual)
        .unwrap()
        .into_iter()
        .chain(board.runtime_tree(stage, TreeKind::Logical).unwrap())
        .find(|row| row.source_id == Some(node))
        .unwrap_or_else(|| {
            panic!(
                "Node {node} has no mapping in stage {stage}: {:?}",
                board.stage(stage).unwrap().status
            )
        });
    let token = board.selection_token(stage, row.inspector_id).unwrap();
    board.resolve_selection(&token).unwrap()
}

#[test]
fn pinned_document_stages_focus_the_correct_source_environment_and_inspector_without_losing_history(
) {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::with_uri(FIRST_SOURCE, FIRST.into()).unwrap();
    let window = attach(&workspace);
    let first_node = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Pick")
        .unwrap();
    workspace
        .session
        .borrow_mut()
        .apply(
            "Authored width",
            vec![Edit::SetAttribute {
                node: first_node,
                name: "Width".into(),
                value: "144".into(),
            }],
        )
        .unwrap();
    workspace.refresh(true);
    let original = workspace
        .session
        .borrow()
        .document()
        .snapshot_without_history();
    let mut first_config = configuration("First phone", FIRST);
    first_config.environment = PreviewEnvironment {
        width: 390.0,
        height: 844.0,
        theme: crate::PreviewTheme::Dark,
        flow_direction: crate::PreviewFlowDirection::RightToLeft,
    };
    let first_stage = workspace.add_preview_stage(first_config.clone()).unwrap();
    workspace.set_preview_stages_visible(true).unwrap();
    window.update_layout();
    let accepted = selection(&workspace, first_stage, first_node);
    workspace
        .open_document(SECOND_SOURCE.into(), SECOND.into())
        .unwrap();
    let second_stage = workspace
        .add_preview_stage(configuration("Second desktop", SECOND))
        .unwrap();
    assert_eq!(workspace.preview_stage_manifest().stages.len(), 2);
    assert!(matches!(
        workspace.preview_stage(first_stage).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    assert!(matches!(
        workspace.preview_stage(second_stage).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    workspace.focus_preview_stage_selection(&accepted).unwrap();
    let session = workspace.session.borrow();
    assert_eq!(session.document_uri(), FIRST);
    assert_eq!(session.document().source(), original.source());
    assert!(Arc::ptr_eq(
        &session.document().ast_snapshot(),
        &original.ast_snapshot()
    ));
    assert!(
        session.document().revision() > original.revision(),
        "A retained tab may rebase its revision"
    );
    assert_eq!(session.selection(), &[first_node]);
    assert!(session.can_undo());
    let native = session
        .preview
        .visual(first_node)
        .unwrap()
        .cast::<Button>()
        .unwrap();
    assert_eq!(native.width(), 144.0);
    assert_eq!(native.flow_direction(), FlowDirection::RightToLeft);
    assert_eq!(native.actual_theme_variant(), Some(ThemeVariant::dark()));
    drop(session);
    assert_eq!(workspace.preview_environment(), first_config.environment);
    assert_eq!(workspace.selection_title.text().as_deref(), Some("Pick"));
    let rows = workspace.inspectors.property_source.items().to_vec();
    let width = rows.iter().find(|row| row.name == "Width").unwrap();
    assert_eq!(width.context.as_ref().unwrap().targets, vec![first_node]);
    assert!(width.display_value.contains("144"));
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(first_node)
            .unwrap()
            .attribute("Width"),
        Some("80")
    );
    workspace.activate_document(SECOND).unwrap();
    assert_eq!(
        workspace.session.borrow().document().source(),
        SECOND_SOURCE
    );
    assert!(!workspace.session.borrow().can_undo());
    window.close();
}

#[test]
fn stage_source_focus_refuses_pending_text_and_same_text_from_a_replacement_ast() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::with_uri(FIRST_SOURCE, FIRST.into()).unwrap();
    let node = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Pick")
        .unwrap();
    let stage = workspace
        .add_preview_stage(configuration("Accepted", FIRST))
        .unwrap();
    let accepted = selection(&workspace, stage, node);
    workspace.source.set_text(Some("<Grid"));
    let pending_source = workspace.session.borrow().document().source().to_owned();
    let revision = workspace.session.borrow().document().revision();
    assert!(workspace.pending_source.get());
    assert!(workspace
        .focus_preview_stage_selection(&accepted)
        .unwrap_err()
        .contains("source changed"));
    assert_eq!(
        workspace.session.borrow().document().source(),
        pending_source
    );
    assert_eq!(workspace.source.text().as_deref(), Some("<Grid"));
    assert_eq!(workspace.session.borrow().document().revision(), revision);
    assert!(workspace.pending_source.get());
    workspace.pending_source.set(false);
    workspace
        .session
        .borrow_mut()
        .restore_document(Document::new(FIRST_SOURCE).unwrap(), FIRST.into())
        .unwrap();
    assert_eq!(workspace.session.borrow().document().source(), FIRST_SOURCE);
    assert!(
        workspace.focus_preview_stage_selection(&accepted).is_err(),
        "Equal source text and reused local IDs do not authorize an old AST selection"
    );
}

#[test]
fn definition_stage_part_selection_enters_the_original_definition_and_preserves_authored_xaml() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Grid xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Grid.Resources><ControlTemplate x:Key="Card" TargetType="Button"><Border Name="Part" Background="Red"/></ControlTemplate></Grid.Resources></Grid>"#;
    let workspace = DesignerWorkspace::with_uri(source, FIRST.into()).unwrap();
    let window = attach(&workspace);
    let (definition, part, anchor) = {
        let session = workspace.session.borrow();
        let definition = session
            .document()
            .ast()
            .elements()
            .find(|node| node.local_name() == "ControlTemplate")
            .unwrap()
            .id;
        (
            definition,
            session.document().ast().find_name("Part").unwrap(),
            PreviewSourceAnchor::capture(session.document(), definition).unwrap(),
        )
    };
    let mut config = configuration("Card part", FIRST);
    config.environment.width = 768.0;
    config.environment.height = 1024.0;
    config.target = PreviewStageTarget::Definition {
        anchor,
        options: DefinitionPreviewOptions::default(),
    };
    let stage = workspace.add_preview_stage(config.clone()).unwrap();
    workspace.set_preview_stages_visible(true).unwrap();
    window.update_layout();
    workspace.refresh_preview_stage(stage).unwrap();
    let accepted = selection(&workspace, stage, part);
    workspace
        .open_document(SECOND_SOURCE.into(), SECOND.into())
        .unwrap();
    workspace.focus_preview_stage_selection(&accepted).unwrap();
    let session = workspace.session.borrow();
    assert_eq!(session.document_uri(), FIRST);
    assert_eq!(session.document().source(), source);
    assert_eq!(session.definition_scope().unwrap().node, definition);
    assert_eq!(session.selection(), &[part]);
    assert!(session.preview.visual(part).is_some());
    assert!(!session.can_undo());
    drop(session);
    assert_eq!(workspace.preview_environment(), config.environment);
    assert_eq!(workspace.selection_title.text().as_deref(), Some("Part"));
    window.close();
}

#[test]
fn focusing_state_and_data_variants_updates_native_inspection_without_changing_source_or_host_epoch(
) {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:d="http://schemas.microsoft.com/expression/blend/2008" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="d" Name="Owner"><Button Name="Pick" Content="Runtime" d:Content="Design" Opacity="1"/></Canvas>"#;
    let workspace = DesignerWorkspace::with_uri(source, FIRST.into()).unwrap();
    let window = attach(&workspace);
    let owner = workspace.session.borrow().document().ast().root();
    let node = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Pick")
        .unwrap();
    workspace
        .session
        .borrow_mut()
        .upsert_state_group(VisualStateGroup {
            name: "Interaction".into(),
            owner,
            states: vec![VisualState {
                name: "Muted".into(),
                setters: vec![StateSetter {
                    target: "Pick".into(),
                    property: "Opacity".into(),
                    value: "0.25".into(),
                }],
            }],
            transitions: Vec::new(),
        })
        .unwrap();
    workspace.refresh(true);
    let before = workspace.session.borrow().document().source().to_owned();
    let revision = workspace.session.borrow().document().revision();
    let host_epoch = workspace.session.borrow().preview_host_options_epoch();
    let owner_anchor =
        PreviewSourceAnchor::capture(workspace.session.borrow().document(), owner).unwrap();
    let mut config = configuration("Runtime muted", FIRST);
    config.design_time_data = false;
    config.states.push(PreviewStageState {
        owner: owner_anchor,
        group: "Interaction".into(),
        state: "Muted".into(),
    });
    let stage = workspace.add_preview_stage(config).unwrap();
    workspace.set_preview_stages_visible(true).unwrap();
    window.update_layout();
    workspace.refresh_preview_stage(stage).unwrap();
    let accepted = selection(&workspace, stage, node);
    workspace.focus_preview_stage_selection(&accepted).unwrap();
    let session = workspace.session.borrow();
    assert!(!session.design_time_enabled());
    assert_eq!(session.preview_host_options_epoch(), host_epoch);
    assert_eq!(session.document().source(), before);
    assert_eq!(session.document().revision(), revision);
    assert!(session.can_undo());
    assert_eq!(
        session.active_preview_states(),
        vec![(owner, "Interaction".into(), "Muted".into())]
    );
    let button = session
        .preview
        .visual(node)
        .unwrap()
        .cast::<Button>()
        .unwrap();
    assert_eq!(button.opacity(), 0.25);
    assert_eq!(
        ferroui_base::data::core::ValueTypes::to_display_string(button.content().as_ref()),
        "Runtime"
    );
    drop(session);
    let rows = workspace.inspectors.property_source.items().to_vec();
    let opacity = rows.iter().find(|row| row.name == "Opacity").unwrap();
    assert_eq!(opacity.context.as_ref().unwrap().targets, vec![node]);
    assert!(opacity.effective.contains("0.25"));
    window.close();
}

#[cfg(unix)]
#[test]
fn pinned_file_alias_keeps_its_original_loader_uri_after_tab_retention_and_focus() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let directory = std::env::temp_dir().join(format!(
        "ferroui-stage-alias-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("View.xaml");
    std::fs::write(&path, FIRST_SOURCE).unwrap();
    let uri = format!("file://localhost{}", path.display());
    let workspace = DesignerWorkspace::with_uri(FIRST_SOURCE, uri.clone()).unwrap();
    let node = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Pick")
        .unwrap();
    let stage = workspace
        .add_preview_stage(configuration("Pinned alias", &uri))
        .unwrap();
    let accepted = selection(&workspace, stage, node);
    workspace
        .open_document(SECOND_SOURCE.into(), SECOND.into())
        .unwrap();
    assert!(matches!(
        workspace.preview_stage(stage).unwrap().status,
        PreviewStageStatus::Ready { .. }
    ));
    assert!(workspace
        .preview_stage_sources()
        .iter()
        .any(|source| source.uri == uri));
    workspace.focus_preview_stage_selection(&accepted).unwrap();
    assert_eq!(workspace.session.borrow().document_uri(), uri);
    assert_eq!(workspace.session.borrow().selection(), &[node]);
    drop(workspace);
    std::fs::remove_dir_all(directory).unwrap();
}
