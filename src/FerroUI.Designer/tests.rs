use super::*;
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, ContentControl, Control};
use ferroui_markup_xaml::FromXamlObject;

fn source() -> &'static str {
    r#"<Canvas xmlns="https://github.com/ferroui" Width="400" Height="240"><Button Name="Target" Content="Preview" Width="100" Height="40" Canvas.Left="16" Canvas.Top="24" /></Canvas>"#
}

#[test]
fn native_source_and_property_edits_share_history_and_preview() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(source(), ContentControl::new()).unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    let id = session.document().ast().find_name("Target").unwrap();
    session.select([id]);
    session.set_property(&[id], "Width", "180").unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Width"),
        Some("180")
    );
    assert_eq!(
        session
            .preview
            .visual(id)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        180.0
    );
    session.undo().unwrap();
    assert_eq!(
        session
            .preview
            .visual(id)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        100.0
    );
    session.redo().unwrap();
    assert_eq!(
        session
            .preview
            .visual(id)
            .unwrap()
            .cast::<Button>()
            .unwrap()
            .width(),
        180.0
    );
}

#[test]
fn invalid_or_runtime_unsupported_source_retains_the_last_preview() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = ContentControl::new();
    let mut session = DesignerSession::new(source(), host.clone()).unwrap();
    let before = Ref::<Control>::from_xaml_object(&host.content().unwrap()).unwrap();
    let update = session.update_source("<Canvas");
    assert!(!update.valid);
    assert_eq!(session.document().source(), "<Canvas");
    let after = Ref::<Control>::from_xaml_object(&host.content().unwrap()).unwrap();
    assert!(std::ptr::eq(&*before, &*after));
    session.update_source(r#"<MissingControl xmlns="https://github.com/ferroui" />"#);
    assert!(matches!(
        session.preview_status(),
        PreviewStatus::Failed { .. }
    ));
    let after = Ref::<Control>::from_xaml_object(&host.content().unwrap()).unwrap();
    assert!(std::ptr::eq(&*before, &*after));
}

#[test]
fn workspace_loads_native_shell_and_selects_source_nodes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(source()).unwrap();
    assert!(workspace.control().find_control::<Button>("Undo").is_some());
    let session = workspace.session();
    let id = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    session.borrow_mut().select([id]);
    workspace.refresh(true);
    assert_eq!(session.borrow().selection(), &[id]);
    assert_eq!(workspace.source_editor().text().as_deref(), Some(source()));
    workspace.set_outline_mode(OutlineMode::Visual);
    workspace.set_outline_mode(OutlineMode::Logical);
}

#[test]
fn keyframe_recording_roundtrips_xaml_and_preview_layers_restore_base_values() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(source(), ContentControl::new()).unwrap();
    let id = session.document().ast().find_name("Target").unwrap();
    session
        .record_keyframe(
            id,
            "Width",
            ferroui_designer_animation::AnimationValue::Number(100.0),
        )
        .unwrap();
    session.seek(3.0).unwrap();
    session
        .record_keyframe(
            id,
            "Width",
            ferroui_designer_animation::AnimationValue::Number(200.0),
        )
        .unwrap();
    assert!(session.document().source().contains("Style.Animations"));
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    session.seek(1.5).unwrap();
    let visual = session.preview.visual(id).unwrap();
    let button = visual.cast::<Button>().unwrap();
    assert!((button.width() - 150.0).abs() < 0.001);
    session.stop();
    assert_eq!(button.width(), 100.0);
    session.undo().unwrap();
    assert_eq!(
        session
            .timeline()
            .track(id, "Width")
            .unwrap()
            .keyframes
            .last()
            .unwrap()
            .value,
        ferroui_designer_animation::AnimationValue::Number(100.0)
    );
}

#[test]
fn collaboration_merges_fine_grained_edits_and_refuses_undo_over_a_remote_write() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut a = DesignerSession::new(source(), ContentControl::new()).unwrap();
    let mut b = DesignerSession::new(source(), ContentControl::new()).unwrap();
    a.enable_collaboration("alice", "view").unwrap();
    b.enable_collaboration("bob", "view").unwrap();
    let aid = a.document().ast().find_name("Target").unwrap();
    let bid = b.document().ast().find_name("Target").unwrap();
    a.set_property(&[aid], "Width", "160").unwrap();
    b.set_property(&[bid], "Height", "75").unwrap();
    let am = a.take_collaboration_changes().unwrap();
    let bm = b.take_collaboration_changes().unwrap();
    a.merge_collaboration(bm).unwrap();
    b.merge_collaboration(am).unwrap();
    assert_eq!(a.document().source(), b.document().source());
    assert_eq!(
        a.document().ast().node(aid).unwrap().attribute("Height"),
        Some("75")
    );
    assert!(a
        .undo()
        .unwrap_err()
        .to_string()
        .contains("collaboration operation"));
}

#[test]
fn shared_source_draft_survives_remote_edits_and_requires_explicit_resolution() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut a = DesignerSession::new(source(), ContentControl::new()).unwrap();
    let mut b = DesignerSession::new(source(), ContentControl::new()).unwrap();
    a.enable_collaboration("alice", "view").unwrap();
    b.enable_collaboration("bob", "view").unwrap();
    a.stage_source("<Canvas");
    let bid = b.document().ast().find_name("Target").unwrap();
    b.set_property(&[bid], "Width", "240").unwrap();
    a.merge_collaboration(b.take_collaboration_changes().unwrap())
        .unwrap();
    assert_eq!(a.document().source(), "<Canvas");
    a.stage_source(source().replace("Preview", "Draft"));
    assert!(a.apply_staged_preview().is_err());
    assert!(a.collaboration_error().is_some());
    assert!(a.document().source().contains("Draft"));
    a.discard_collaboration_draft().unwrap();
    assert_eq!(a.document().source(), b.document().source());
}

#[test]
fn shared_undo_redo_are_new_guarded_operations() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(source(), ContentControl::new()).unwrap();
    session.enable_collaboration("alice", "view").unwrap();
    let id = session.document().ast().find_name("Target").unwrap();
    session.set_property(&[id], "Width", "160").unwrap();
    session.undo().unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Width"),
        Some("100")
    );
    session.redo().unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Width"),
        Some("160")
    );
    assert_eq!(
        session
            .take_collaboration_changes()
            .unwrap()
            .operations
            .len(),
        3
    );
}

#[test]
fn window_and_user_control_roots_render_children_in_the_artboard() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for root in ["Window", "UserControl"] {
        let source = format!("<{} xmlns=\"https://github.com/ferroui\" Width=\"400\" Height=\"240\"><Button Name=\"Target\" Content=\"Root preview\" Width=\"100\" /></{}>", root, root);
        let session = DesignerSession::new(source, ContentControl::new()).unwrap();
        assert!(
            matches!(session.preview_status(), PreviewStatus::Ready { .. }),
            "{root}: {:?}",
            session.preview_status()
        );
        assert!(session.preview.root().is_some());
        let target = session.document().ast().find_name("Target").unwrap();
        assert!(
            session.preview.visual(target).is_some(),
            "{root} target has no mapped live control"
        );
    }
}

#[test]
fn transformed_canvas_manipulation_uses_parent_units_and_preserves_authored_margin() {
    use ferroui_base::{Point, Visual};
    use ferroui_controls::Window;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = ContentControl::new();
    let mut session=DesignerSession::new(r#"<Canvas xmlns="https://github.com/ferroui" Width="600" Height="400"><Canvas Name="Group" Canvas.Left="40" Canvas.Top="20" Width="200" Height="150" RenderTransformOrigin="0,0" RenderTransform="scale(2)"><Button Name="Target" Width="40" Height="20" Margin="5,3,0,0" Canvas.Left="10" Canvas.Top="12" /></Canvas></Canvas>"#,host.clone()).unwrap();
    let window = Window::new();
    window.set_width(600.0);
    window.set_height(400.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    session.refresh_runtime_mapping();
    let id = session.document().ast().find_name("Target").unwrap();
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::Move {
            nodes: vec![id],
            delta: Point::new(20.0, 10.0),
        },
    )
    .unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Canvas.Left"),
        Some("20")
    );
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Canvas.Top"),
        Some("17")
    );
    window.update_layout();
    session.refresh_runtime_mapping();
    let before = session
        .preview
        .bounds(id, &host.clone().upcast::<Visual>())
        .unwrap();
    let mut after = before;
    after.width += 20.0;
    after.height += 20.0;
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::Resize {
            node: id,
            before,
            after,
        },
    )
    .unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Width"),
        Some("50")
    );
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Height"),
        Some("30")
    );
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Margin"),
        Some("5,3,0,0")
    );
    window.close();
}

#[test]
fn centered_grid_resize_keeps_the_dragged_leading_edge() {
    use ferroui_base::Visual;
    use ferroui_controls::Window;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let host = ContentControl::new();
    let mut session=DesignerSession::new(r#"<Grid xmlns="https://github.com/ferroui" Width="400" Height="240"><Button Name="Target" Width="100" Height="40" HorizontalAlignment="Center" VerticalAlignment="Center" /></Grid>"#,host.clone()).unwrap();
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(240.0);
    window.set_content(Some(Control::boxed(&host)));
    window.show();
    window.update_layout();
    let id = session.document().ast().find_name("Target").unwrap();
    let before = session
        .preview
        .bounds(id, &host.clone().upcast::<Visual>())
        .unwrap();
    let mut after = before;
    after.width += 40.0;
    crate::workspace_tools::commit(
        &mut session,
        DesignGesture::Resize {
            node: id,
            before,
            after,
        },
    )
    .unwrap();
    window.update_layout();
    let actual = session
        .preview
        .bounds(id, &host.clone().upcast::<Visual>())
        .unwrap();
    assert!(
        (actual.x - before.x).abs() < 0.001,
        "{} -> {}",
        before.x,
        actual.x
    );
    assert!((actual.width - after.width).abs() < 0.001);
    window.close();
}

#[test]
fn empty_values_reset_and_opening_another_document_preserve_revision_guards() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(source(), ContentControl::new()).unwrap();
    let id = session.document().ast().find_name("Target").unwrap();
    session.set_property(&[id], "Content", "").unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Content"),
        Some("")
    );
    session.reset_property(&[id], "Content").unwrap();
    assert_eq!(
        session
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Content"),
        None
    );
    let revision = session.document().revision();
    session
        .open_document(
            source().replace("Preview", "Another"),
            "file:///tmp/Another.xaml".into(),
        )
        .unwrap();
    assert!(session.document().revision() > revision);
    assert!(!session.can_undo());
    assert!(!session.document().is_dirty());
}

#[test]
fn unrepresented_animations_are_reported_and_source_is_preserved() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Width="100"><Button.Styles><Style Selector="Button#Target"><Style.Animations><Animation Duration="0:0:3"><KeyFrame Cue="0%"><Setter Property="Width" Value="100" /></KeyFrame><KeyFrame Cue="100%"><Setter Property="Width" Value="200" /></KeyFrame></Animation><Animation Duration="0:0:2"><KeyFrame Cue="0%"><Setter Property="Height" Value="20" /></KeyFrame><KeyFrame Cue="100%"><Setter Property="Height" Value="40" /></KeyFrame></Animation></Style.Animations></Style></Button.Styles></Button><Button><Button.Styles><Style Selector="Button"><Style.Animations><Animation Duration="0:0:1" /></Style.Animations></Style></Button.Styles></Button></Canvas>"#;
    let session = DesignerSession::new(source, ContentControl::new()).unwrap();
    assert!(
        session.animation_warnings().len() >= 2,
        "{:?}",
        session.animation_warnings()
    );
    assert_eq!(session.document().source(), source);
}

#[test]
fn duplicate_refuses_to_add_a_second_child_to_a_content_host() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(
        r#"<Border xmlns="https://github.com/ferroui"><Button Name="Target" /></Border>"#,
        ContentControl::new(),
    )
    .unwrap();
    session.select([session.document().ast().find_name("Target").unwrap()]);
    assert!(crate::workspace_tools::duplicate(&mut session)
        .unwrap_err()
        .contains("multiple"));
    assert_eq!(session.document().revision(), 0);
}

#[test]
fn recording_in_a_prefixed_document_reuses_root_namespaces() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<ui:Canvas xmlns:ui="https://github.com/ferroui"><ui:Button Name="Target" Width="100" /></ui:Canvas>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let id = session.document().ast().find_name("Target").unwrap();
    session
        .record_keyframe(
            id,
            "Width",
            ferroui_designer_animation::AnimationValue::Number(100.0),
        )
        .unwrap();
    session.seek(3.0).unwrap();
    session
        .record_keyframe(
            id,
            "Width",
            ferroui_designer_animation::AnimationValue::Number(180.0),
        )
        .unwrap();
    assert!(
        matches!(session.preview_status(), PreviewStatus::Ready { .. }),
        "{:?}",
        session.preview_status()
    );
    assert_eq!(session.document().source().matches("xmlns:").count(), 1);
    assert!(session.document().source().contains("<ui:Animation"));
}
