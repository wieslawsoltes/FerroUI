use crate::{DesignerSession, Document, Edit, LayerCommand};
use ferroui_base::input::InputElement;
use ferroui_base::{Ref, Visual};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{ContentControl, Control, Window};
use ferroui_designer_core::NodeId;

const URI: &str = "ferres://FerroUI.Designer/Layers.xaml";
const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="500" Height="300">
  <StackPanel Name="Group" Canvas.Left="20" Canvas.Top="20" Width="200" Spacing="10">
    <Button Name="First" Width="100" Height="30" Opacity="0.4" Content="One" />
    <Button Name="Second" Width="100" Height="30" Content="Two" />
  </StackPanel>
  <Button Name="Peer" Canvas.Left="260" Canvas.Top="20" Width="100" Height="30" Content="Peer" />
</Canvas>"#;

struct Fixture {
    session: DesignerSession,
    window: Ref<Window>,
}
impl Fixture {
    fn new() -> Self {
        let host = ContentControl::new();
        let mut session = DesignerSession::with_uri(SOURCE, host.clone(), URI.into()).unwrap();
        let window = Window::new();
        window.set_width(500.0);
        window.set_height(300.0);
        window.set_content(Some(Control::boxed(&host)));
        window.show();
        window.update_layout();
        session.refresh_runtime_mapping();
        Self { session, window }
    }
    fn id(&self, name: &str) -> NodeId {
        self.session.document().ast().find_name(name).unwrap()
    }
    fn visual(&self, name: &str) -> Ref<Visual> {
        self.session.preview.visual(self.id(name)).unwrap()
    }
}

#[test]
fn layer_visibility_is_counted_composition_state_and_preserves_layout_properties_and_history() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let group = fixture.id("Group");
    let first = fixture.id("First");
    let visual = fixture.visual("First");
    let input = visual.cast::<InputElement>().unwrap();
    let before_bounds = visual.bounds();
    let source = fixture.session.document().source().to_owned();
    let revision = fixture.session.document().revision();
    fixture
        .session
        .enable_collaboration("writer", "layers")
        .unwrap();
    let version = fixture.session.collaboration_version().unwrap();
    visual.add_composition_hide_request();
    fixture.session.set_layers_hidden(&[group], true).unwrap();
    assert!(fixture.session.layer_state(first).inherited_hidden);
    assert!(fixture.visual("Group").is_hidden_by_composition());
    assert!(visual.is_hidden_by_composition());
    assert!(visual.is_visible());
    assert_eq!(visual.opacity(), 0.4);
    assert!(!input.is_hit_test_visible());
    fixture.window.update_layout();
    assert_eq!(visual.bounds(), before_bounds);
    for _ in 0..3 {
        fixture.session.refresh_runtime_mapping();
    }
    fixture.session.set_layers_hidden(&[group], false).unwrap();
    assert!(!fixture.visual("Group").is_hidden_by_composition());
    assert!(
        visual.is_hidden_by_composition(),
        "another compositor owner still hides it"
    );
    visual.remove_composition_hide_request();
    assert!(
        !visual.is_hidden_by_composition(),
        "refresh did not stack requests"
    );
    assert!(input.is_hit_test_visible());
    assert_eq!(visual.opacity(), 0.4);
    assert_eq!(fixture.session.document().source(), source);
    assert_eq!(fixture.session.document().revision(), revision);
    assert!(!fixture.session.can_undo());
    assert_eq!(fixture.session.collaboration_version().unwrap(), version);
}

#[test]
fn layer_locks_inherit_but_source_inspection_and_deliberate_property_edits_remain_available() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let group = fixture.id("Group");
    let first = fixture.id("First");
    let second = fixture.id("Second");
    fixture.session.set_layers_locked(&[group], true).unwrap();
    assert!(fixture.session.layer_state(group).locked);
    assert!(fixture.session.layer_state(first).inherited_locked);
    assert!(!fixture.session.can_canvas_select(first));
    assert!(fixture.session.ensure_layer_manipulable(first).is_err());
    fixture.session.select([first]);
    assert_eq!(fixture.session.selection(), &[first]);
    fixture
        .session
        .set_property(&[first], "Content", "Changed intentionally")
        .unwrap();
    assert!(
        fixture
            .session
            .document()
            .source()
            .contains("Changed intentionally")
    );
    assert!(fixture.session.layer_state(first).inherited_locked);
    fixture.session.set_layers_locked(&[first], false).unwrap();
    assert!(fixture.session.layer_state(first).is_locked());
    fixture
        .session
        .apply_layer_command(LayerCommand::UnlockAll)
        .unwrap();
    assert!(fixture.session.can_canvas_select(first));
    fixture.session.set_layers_locked(&[second], true).unwrap();
    fixture.session.select([first]);
    fixture
        .session
        .apply_layer_command(LayerCommand::SelectNextSibling)
        .unwrap();
    assert_eq!(
        fixture.session.selection(),
        &[first],
        "navigation skips locked siblings"
    );
}

#[test]
fn layer_isolation_preserves_ancestors_and_layout_and_retires_after_root_deletion() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let first = fixture.id("First");
    let second = fixture.id("Second");
    let group = fixture.id("Group");
    let peer = fixture.id("Peer");
    fixture.session.isolate_layers(&[first]).unwrap();
    assert_eq!(fixture.session.layer_isolation(), &[first]);
    assert!(!fixture.session.layer_state(group).outside_isolation);
    assert!(!fixture.visual("Group").is_hidden_by_composition());
    assert!(!fixture.visual("First").is_hidden_by_composition());
    assert!(fixture.visual("Second").is_hidden_by_composition());
    assert!(fixture.visual("Peer").is_hidden_by_composition());
    assert!(fixture.session.can_canvas_select(first));
    assert!(!fixture.session.can_canvas_select(group));
    assert!(!fixture.session.can_canvas_select(second));
    assert!(!fixture.session.can_canvas_select(peer));
    fixture.session.clear_layer_isolation();
    assert!(!fixture.visual("Peer").is_hidden_by_composition());
    fixture.session.isolate_layers(&[group, first]).unwrap();
    assert_eq!(fixture.session.layer_isolation(), &[group]);
    assert!(fixture.session.can_canvas_select(first));
    fixture
        .session
        .apply(
            "Remove isolated group",
            vec![Edit::RemoveNode { node: group }],
        )
        .unwrap();
    assert!(fixture.session.layer_isolation().is_empty());
    assert!(!fixture.visual("Peer").is_hidden_by_composition());
}

#[test]
fn layer_actions_validate_all_targets_and_retire_callbacks_after_a_view_round_trip() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let first = fixture.id("First");
    let before = fixture.session.layer_view_epoch();
    assert!(
        fixture
            .session
            .set_layers_locked(&[first, u64::MAX], true)
            .is_err()
    );
    assert!(!fixture.session.layer_state(first).locked);
    assert_eq!(fixture.session.layer_view_epoch(), before);
    let row = fixture.session.capture_layer_edit(&[first]).unwrap();
    fixture.session.set_layers_locked(&[first], true).unwrap();
    fixture.session.set_layers_locked(&[first], false).unwrap();
    assert!(
        fixture
            .session
            .apply_layer_edit(&row, LayerCommand::ToggleHidden)
            .is_err()
    );
    assert!(!fixture.session.layer_state(first).hidden);
    let row = fixture.session.capture_layer_edit(&[first]).unwrap();
    fixture
        .session
        .set_property(&[first], "Content", "Next revision")
        .unwrap();
    assert!(
        fixture
            .session
            .apply_layer_edit(&row, LayerCommand::ToggleLock)
            .is_err()
    );
}

#[test]
fn layer_tab_snapshots_restore_only_the_exact_document_and_mask_guards_restore_on_replacement() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let first = fixture.id("First");
    let old = fixture.visual("First");
    fixture.session.set_layers_hidden(&[first], true).unwrap();
    fixture.session.set_layers_locked(&[first], true).unwrap();
    let document = fixture.session.document().clone();
    let snapshot = fixture.session.layer_view_snapshot();
    fixture
        .session
        .open_document(SOURCE.into(), "ferres://FerroUI.Designer/Other.xaml".into())
        .unwrap();
    assert!(!old.is_hidden_by_composition());
    assert!(!fixture.session.restore_layer_view(&snapshot));
    fixture
        .session
        .restore_document(document, URI.into())
        .unwrap();
    assert!(fixture.session.restore_layer_view(&snapshot));
    assert!(fixture.session.layer_state(first).hidden);
    assert!(fixture.session.layer_state(first).locked);
    assert!(fixture.visual("First").is_hidden_by_composition());
    fixture
        .session
        .restore_document(Document::new(SOURCE).unwrap(), URI.into())
        .unwrap();
    assert!(!fixture.session.restore_layer_view(&snapshot));
    assert!(!fixture.session.layer_state(fixture.id("First")).hidden);
}

#[test]
fn layer_hidden_flags_survive_source_undo_without_becoming_undo_operations() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let first = fixture.id("First");
    fixture.session.set_layers_hidden(&[first], true).unwrap();
    assert!(!fixture.session.can_undo());
    fixture
        .session
        .apply("Remove layer", vec![Edit::RemoveNode { node: first }])
        .unwrap();
    assert!(fixture.session.document().ast().node(first).is_none());
    fixture.session.undo().unwrap();
    assert_eq!(fixture.id("First"), first);
    assert!(fixture.session.layer_state(first).hidden);
    assert!(fixture.visual("First").is_hidden_by_composition());
    assert!(!fixture.session.can_undo());
    fixture.session.set_read_only(true);
    fixture.session.set_layers_hidden(&[first], false).unwrap();
    assert!(!fixture.visual("First").is_hidden_by_composition());
    assert!(
        fixture
            .session
            .set_property(&[first], "Content", "Forbidden")
            .is_err()
    );
}

#[test]
fn layer_protection_applies_to_drop_planners_and_handle_actions_before_a_preview_is_accepted() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let group = fixture.id("Group");
    let first = fixture.id("First");
    let peer = fixture.id("Peer");
    let revision = fixture.session.document().revision();
    fixture.session.select([first]);
    fixture.session.set_layers_locked(&[group], true).unwrap();
    assert!(fixture.session.validate_content_insertion(group).is_err());
    assert!(
        fixture
            .session
            .plan_content_move(
                &[peer],
                group,
                0,
                ferroui_designer_tools::Point::new(0.0, 0.0)
            )
            .is_err()
    );
    let snapshot = fixture
        .session
        .layout_helper_snapshot(group, None, &[])
        .unwrap();
    let index = snapshot.scene.handles.iter().position(|handle| {
        matches!(&handle.action, ferroui_designer_tools::LayoutHandleAction::PanelProperty { panel, .. } if *panel == group)
    }).expect("StackPanel spacing handle");
    assert!(snapshot.scene.handles[index].disabled_reason.is_some());
    assert!(
        fixture
            .session
            .preview_layout_handle(&snapshot, index, ferroui_base::Point::new(0.0, 5.0))
            .is_err()
    );
    let root = fixture.session.document().ast().root();
    fixture.session.set_layers_locked(&[root], true).unwrap();
    assert!(
        fixture
            .session
            .preview_layout_insert("<Button/>", ferroui_base::Point::new(40.0, 40.0))
            .is_err()
    );
    assert_eq!(fixture.session.document().revision(), revision);
    assert_eq!(fixture.session.document().source(), SOURCE);
}

#[test]
fn layer_isolation_keeps_child_margin_and_same_parent_reorder_context_but_blocks_parent_edits() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let group = fixture.id("Group");
    let first = fixture.id("First");
    let peer = fixture.id("Peer");
    fixture.session.select([first]);
    fixture.session.isolate_layers(&[first]).unwrap();
    assert!(fixture.session.ensure_layer_manipulable(group).is_err());
    assert!(
        fixture
            .session
            .ensure_layer_container_context(group, &[first])
            .is_ok()
    );
    assert!(fixture.session.validate_content_insertion(group).is_err());
    assert!(
        fixture
            .session
            .plan_content_move(
                &[first],
                group,
                2,
                ferroui_designer_tools::Point::new(0.0, 0.0)
            )
            .is_ok()
    );
    assert!(
        fixture
            .session
            .plan_content_move(
                &[peer],
                group,
                0,
                ferroui_designer_tools::Point::new(0.0, 0.0)
            )
            .is_err()
    );
    let snapshot = fixture.session.selection_layout_helpers().unwrap();
    assert_eq!(snapshot.scene.panel, group);
    let margin = snapshot.scene.handles.iter().position(|handle| {
        matches!(&handle.action, ferroui_designer_tools::LayoutHandleAction::ThicknessSide { node, property, .. } if *node == first && property == "Margin")
    }).expect("isolated child's native margin handle");
    assert!(snapshot.scene.handles[margin].disabled_reason.is_none());
    let preview = fixture
        .session
        .preview_layout_handle(&snapshot, margin, ferroui_base::Point::new(5.0, 5.0))
        .unwrap();
    assert!(preview.is_accepted());
    fixture.session.commit_layout_preview(&preview).unwrap();
    assert!(
        fixture
            .session
            .document()
            .ast()
            .node(first)
            .unwrap()
            .attribute("Margin")
            .is_some()
    );
    assert_eq!(
        fixture
            .session
            .document()
            .ast()
            .node(group)
            .unwrap()
            .attribute("Spacing"),
        Some("10")
    );
}

#[test]
fn layer_locked_siblings_remain_valid_positional_anchors_for_outline_ordering() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut fixture = Fixture::new();
    let group = fixture.id("Group");
    let first = fixture.id("First");
    let second = fixture.id("Second");
    fixture.session.set_layers_locked(&[second], true).unwrap();
    fixture.session.select([first]);
    fixture
        .session
        .edit_outline(crate::OutlineCommand::MoveLast)
        .unwrap();
    assert_eq!(
        fixture.session.document().ast().element_children(group),
        vec![second, first]
    );
    assert!(fixture.session.layer_state(second).locked);
    fixture.session.undo().unwrap();
    fixture
        .session
        .edit_outline(crate::OutlineCommand::MoveDown)
        .unwrap();
    assert_eq!(
        fixture.session.document().ast().element_children(group),
        vec![second, first]
    );
}

#[test]
fn layer_workspace_tabs_restore_their_own_view_without_leaking_reused_node_ids() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = crate::DesignerWorkspace::with_uri(SOURCE, URI.into()).unwrap();
    let session = workspace.session();
    let first = session
        .borrow()
        .document()
        .ast()
        .find_name("First")
        .unwrap();
    let group = session
        .borrow()
        .document()
        .ast()
        .find_name("Group")
        .unwrap();
    session
        .borrow_mut()
        .set_layers_locked(&[first], true)
        .unwrap();
    session
        .borrow_mut()
        .set_layers_hidden(&[first], true)
        .unwrap();
    session.borrow_mut().isolate_layers(&[group]).unwrap();
    workspace
        .open_document(
            SOURCE.replace("Content=\"One\"", "Content=\"Other tab\""),
            "ferres://FerroUI.Designer/OtherLayers.xaml".into(),
        )
        .unwrap();
    assert!(!session.borrow().layer_state(first).hidden);
    assert!(!session.borrow().layer_state(first).locked);
    assert!(session.borrow().layer_isolation().is_empty());
    workspace.activate_document(URI).unwrap();
    assert!(session.borrow().layer_state(first).hidden);
    assert!(session.borrow().layer_state(first).locked);
    assert_eq!(session.borrow().layer_isolation(), &[group]);
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert!(!session.borrow().can_undo());
    workspace
        .activate_document("ferres://FerroUI.Designer/OtherLayers.xaml")
        .unwrap();
    assert!(!session.borrow().layer_state(first).hidden);
    assert!(session.borrow().layer_isolation().is_empty());
}
