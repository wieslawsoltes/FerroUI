use crate::{
    DesignerWorkspace, PreviewEnvironment, PreviewEnvironmentEditor, PreviewFlowDirection,
    PreviewTheme,
};
use ferroui_base::media::FlowDirection;
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::ThemeVariantScope;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn preview_environment_changes_inheritance_without_authoring_or_touching_chrome() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui"><Button Name="Inherited"/><Button Name="Explicit" FlowDirection="LeftToRight"/></StackPanel>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    let session = workspace.session();
    let node = session
        .borrow()
        .document()
        .ast()
        .find_name("Inherited")
        .unwrap();
    session.borrow_mut().select([node]);
    session.borrow_mut().set_read_only(true);
    let revision = session.borrow().document().revision();
    let generation = session.borrow().generation();
    let environment = PreviewEnvironment {
        width: 390.0,
        height: 844.0,
        theme: PreviewTheme::Dark,
        flow_direction: PreviewFlowDirection::RightToLeft,
    };
    workspace.set_preview_environment(environment).unwrap();
    workspace.control().update_layout();
    let scope = workspace
        .control()
        .get_control::<ThemeVariantScope>("PreviewThemeScope");
    assert_eq!(scope.requested_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(scope.flow_direction(), FlowDirection::RightToLeft);
    assert_eq!(
        workspace.control().flow_direction(),
        FlowDirection::LeftToRight
    );
    let session = session.borrow();
    let inherited = session.preview.visual(node).unwrap();
    let explicit = session.document().ast().find_name("Explicit").unwrap();
    assert_eq!(inherited.flow_direction(), FlowDirection::RightToLeft);
    assert_eq!(
        session.preview.visual(explicit).unwrap().flow_direction(),
        FlowDirection::LeftToRight
    );
    assert_eq!(session.document().source(), source);
    assert_eq!(session.document().revision(), revision);
    assert!(session.generation() > generation);
    assert_eq!(session.selection(), &[node]);
    assert!(!session.can_undo());
    drop(session);
    assert_eq!(workspace.artboard_size(), (390.0, 844.0));
    workspace.rotate_preview().unwrap();
    assert_eq!(workspace.artboard_size(), (844.0, 390.0));
    workspace.set_preview_dark(false);
    assert_eq!(workspace.preview_environment().theme, PreviewTheme::Light);
    workspace
        .set_preview_environment(PreviewEnvironment {
            flow_direction: PreviewFlowDirection::Inherit,
            theme: PreviewTheme::Inherit,
            ..environment
        })
        .unwrap();
    assert_eq!(scope.flow_direction(), FlowDirection::LeftToRight);
    assert_eq!(scope.requested_theme_variant(), None);
}

#[test]
fn invalid_preview_environment_is_rejected_before_any_live_change() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(crate::STARTER_XAML).unwrap();
    let before = workspace.preview_environment();
    for width in [0.0, -1.0, f64::NAN, f64::INFINITY, 16385.0] {
        assert!(
            workspace
                .set_preview_environment(PreviewEnvironment {
                    width,
                    height: 900.0,
                    theme: PreviewTheme::Dark,
                    flow_direction: PreviewFlowDirection::RightToLeft
                })
                .is_err()
        );
        assert_eq!(workspace.preview_environment(), before);
    }
    assert_eq!(
        workspace
            .control()
            .get_control::<ThemeVariantScope>("PreviewThemeScope")
            .requested_theme_variant(),
        Some(ThemeVariant::light())
    );
}

#[test]
fn preview_editor_stages_drafts_and_cancel_never_calls_the_host() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = PreviewEnvironmentEditor::new(PreviewEnvironment::default());
    let accepted = Rc::new(Cell::new(0));
    let count = accepted.clone();
    editor.on_apply(move |value| {
        assert_eq!(value.width, 390.0);
        count.set(count.get() + 1);
        Ok(())
    });
    let cancelled = Rc::new(Cell::new(false));
    let flag = cancelled.clone();
    editor.on_cancel(move || flag.set(true));
    editor.width.set_text(Some("390"));
    editor.height.set_text(Some("not a number"));
    assert!(editor.apply().is_err());
    assert_eq!(accepted.get(), 0);
    editor.cancel();
    assert!(cancelled.get());
    assert_eq!(accepted.get(), 0);
    editor.height.set_text(Some("844"));
    assert!(
        editor.apply().is_err(),
        "A retained cancelled draft is retired"
    );
    let editor = PreviewEnvironmentEditor::new(PreviewEnvironment {
        width: 390.0,
        height: 844.0,
        ..Default::default()
    });
    let count = accepted.clone();
    editor.on_apply(move |value| {
        assert_eq!(value.width, 390.0);
        count.set(count.get() + 1);
        Ok(())
    });
    editor.apply().unwrap();
    assert_eq!(accepted.get(), 1);
}

#[test]
fn inherited_preview_theme_keeps_the_artboard_backing_in_sync() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(crate::STARTER_XAML).unwrap();
    let parent = ThemeVariantScope::new();
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));
    parent.set_child(Some(workspace.control()));
    workspace
        .set_preview_environment(PreviewEnvironment {
            theme: PreviewTheme::Inherit,
            ..Default::default()
        })
        .unwrap();
    let artboard = workspace
        .control()
        .get_control::<ferroui_controls::Grid>("Artboard");
    let background = || {
        artboard
            .background()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color()
    };
    assert_eq!(
        background(),
        ferroui_base::media::Color::parse("#FFFFFFFF").unwrap()
    );
    parent.set_requested_theme_variant(Some(ThemeVariant::dark()));
    assert_eq!(
        background(),
        ferroui_base::media::Color::parse("#FF202020").unwrap()
    );
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert_eq!(
        background(),
        ferroui_base::media::Color::parse("#FFFFFFFF").unwrap()
    );
}

#[test]
fn inherited_preview_conditions_retire_layout_and_editor_snapshots_after_round_trips() {
    use crate::{DesignTimeSnapshot, LayerCommand};
    use ferroui_controls::{Control, Window};

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<StackPanel xmlns="https://github.com/ferroui"><Button Name="Target" Content="Sample" Width="100" Height="30"/></StackPanel>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    let parent = ThemeVariantScope::new();
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));
    parent.set_child(Some(workspace.control()));
    let window = Window::new();
    window.set_width(1400.0);
    window.set_height(900.0);
    window.set_content(Some(Control::boxed(parent.clone())));
    window.show();
    workspace
        .set_preview_environment(PreviewEnvironment {
            theme: PreviewTheme::Inherit,
            ..Default::default()
        })
        .unwrap();
    window.update_layout();
    let session = workspace.session();
    session.borrow_mut().refresh_runtime_mapping();
    let root = session.borrow().document().ast().root();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    session.borrow_mut().select([target]);
    let layout = session
        .borrow()
        .layout_helper_snapshot(root, None, &[])
        .unwrap();
    let data = DesignTimeSnapshot::capture(&session.borrow());
    let focus = session.borrow().focus_order_snapshot().unwrap();
    let layer = session.borrow().capture_layer_edit(&[target]).unwrap();
    let epoch = session.borrow().definition_scope_epoch();
    let revision = session.borrow().document().revision();
    assert!(layout.is_current(&session.borrow()));
    assert!(data.is_current(&session.borrow()));
    assert!(focus.is_current(&session.borrow()));

    parent.set_flow_direction(FlowDirection::RightToLeft);
    parent.set_flow_direction(FlowDirection::LeftToRight);
    parent.set_requested_theme_variant(Some(ThemeVariant::dark()));
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));

    assert!(session.borrow().definition_scope_epoch() > epoch);
    assert!(!layout.is_current(&session.borrow()));
    assert!(!data.is_current(&session.borrow()));
    assert!(!focus.is_current(&session.borrow()));
    assert!(
        session
            .borrow_mut()
            .apply_layer_edit(&layer, LayerCommand::ToggleLock)
            .is_err()
    );
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().document().revision(), revision);
    assert!(!session.borrow().can_undo());

    let epoch = session.borrow().definition_scope_epoch();
    parent.set_flow_direction(FlowDirection::LeftToRight);
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));
    assert_eq!(session.borrow().definition_scope_epoch(), epoch);
    window.close();
}

#[test]
fn explicit_preview_conditions_retire_once_and_mask_unrelated_inherited_changes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(crate::STARTER_XAML).unwrap();
    let parent = ThemeVariantScope::new();
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));
    parent.set_child(Some(workspace.control()));
    let session = workspace.session();
    let epoch = session.borrow().definition_scope_epoch();
    let environment = PreviewEnvironment {
        width: 390.0,
        height: 844.0,
        theme: PreviewTheme::Dark,
        flow_direction: PreviewFlowDirection::RightToLeft,
    };
    workspace.set_preview_environment(environment).unwrap();
    assert_eq!(session.borrow().definition_scope_epoch(), epoch + 1);
    workspace.set_preview_environment(environment).unwrap();
    assert_eq!(session.borrow().definition_scope_epoch(), epoch + 1);

    parent.set_requested_theme_variant(Some(ThemeVariant::dark()));
    parent.set_flow_direction(FlowDirection::RightToLeft);
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));
    parent.set_flow_direction(FlowDirection::LeftToRight);
    assert_eq!(session.borrow().definition_scope_epoch(), epoch + 1);
    assert_eq!(session.borrow().document().source(), crate::STARTER_XAML);
    assert!(!session.borrow().can_undo());
}

#[test]
fn inherited_preview_invalidation_defers_safely_while_the_session_is_borrowed() {
    use crate::DesignTimeSnapshot;
    use ferroui_base::threading::{Dispatcher, DispatcherPriority};

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(crate::STARTER_XAML).unwrap();
    let parent = ThemeVariantScope::new();
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));
    parent.set_child(Some(workspace.control()));
    workspace
        .set_preview_environment(PreviewEnvironment {
            theme: PreviewTheme::Inherit,
            ..Default::default()
        })
        .unwrap();
    let session = workspace.session();
    let snapshot = DesignTimeSnapshot::capture(&session.borrow());
    let epoch = session.borrow().definition_scope_epoch();
    let held = session.borrow_mut();
    parent.set_requested_theme_variant(Some(ThemeVariant::dark()));
    parent.set_flow_direction(FlowDirection::RightToLeft);
    parent.set_requested_theme_variant(Some(ThemeVariant::light()));
    parent.set_flow_direction(FlowDirection::LeftToRight);
    assert_eq!(held.definition_scope_epoch(), epoch);
    drop(held);
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::SEND));
    assert_eq!(session.borrow().definition_scope_epoch(), epoch + 1);
    assert!(!snapshot.is_current(&session.borrow()));
    assert_eq!(session.borrow().document().source(), crate::STARTER_XAML);
    assert!(!session.borrow().can_undo());
}
