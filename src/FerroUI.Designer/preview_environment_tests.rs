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
