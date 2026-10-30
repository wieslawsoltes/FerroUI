//! End-to-end checks of native timeline controls and the source transactions they trigger.
use crate::{DesignerWorkspace, PreviewStatus};
use ferroui_base::Ref;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{
    Button, ComboBox, ContentControl, Control, Grid, ScrollViewer, StackPanel, TextBlock, TextBox,
};
use ferroui_designer_animation::{AnimationValue, BehaviorAction, BehaviorTrigger, Easing};

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}
fn caption(button: &Button) -> Option<String> {
    button
        .content()
        .and_then(|value| value.downcast_ref::<String>().cloned())
}

fn keyframe_editor(workspace: &DesignerWorkspace) -> Ref<StackPanel> {
    let host = workspace
        .control()
        .get_control::<ContentControl>("TimelineInspectorHost");
    Control::from_boxed(&host.content().expect("A selected key has an inspector"))
        .and_then(|control| control.cast::<StackPanel>())
        .expect("The contextual inspector is a compact field row")
}
fn keyframe_input(editor: &StackPanel, name: &str) -> Ref<TextBox> {
    editor
        .children()
        .to_vec()
        .iter()
        .filter_map(|control| control.cast::<TextBox>())
        .find(|input| input.name().as_deref() == Some(name))
        .unwrap()
}
fn apply_keyframe(editor: &StackPanel) {
    let apply = editor
        .children()
        .to_vec()
        .iter()
        .filter_map(|control| control.cast::<Button>())
        .find(|button| caption(button).as_deref() == Some("Apply keyframe"))
        .unwrap();
    click(&apply);
}
fn timeline_ruler(workspace: &DesignerWorkspace) -> Ref<crate::TimelineLane> {
    let host = workspace
        .control()
        .get_control::<ContentControl>("TimelineRulerHost");
    Control::from_boxed(&host.content().unwrap())
        .unwrap()
        .cast::<Grid>()
        .unwrap()
        .children()
        .to_vec()
        .into_iter()
        .filter_map(|control| control.cast::<crate::TimelineLane>())
        .find(|lane| lane.is_ruler())
        .unwrap()
}
fn timeline_key(lane: &crate::TimelineLane, key: ferroui_base::input::Key) {
    let mut event = ferroui_base::input::KeyEventArgs::new();
    event.key = key;
    event.set_routed_event(Some(ferroui_base::input::InputElement::key_down_event()));
    lane.raise_event(&event);
    assert!(event.handled());
}

fn authoring_row(workspace: &DesignerWorkspace, name: &str) -> Ref<Grid> {
    workspace
        .control()
        .get_control::<StackPanel>("AuthoringPanel")
        .children()
        .to_vec()
        .into_iter()
        .filter_map(|control| control.cast::<Grid>())
        .find(|row| {
            row.children()
                .to_vec()
                .iter()
                .any(|control| control.name().as_deref() == Some(name))
        })
        .unwrap()
}

#[test]
fn behavior_pickers_reveal_relevant_fields_and_attach_one_undoable_action() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" /></Canvas>"#,
    )
    .unwrap();
    let session = workspace.session();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    session.borrow_mut().select([target]);
    workspace.refresh(true);
    // Show the workspace before entering the mode: controls become styled as soon
    // as their rows attach, so names must already be assigned at construction.
    let window = ferroui_controls::Window::new();
    window.set_width(1280.0);
    window.set_height(820.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    click(&workspace.control().get_control::<Button>("BehaviorsMode"));
    assert!(
        !workspace
            .control()
            .get_control::<Grid>("InspectorDesignPane")
            .is_visible()
    );
    assert!(
        workspace
            .control()
            .get_control::<ScrollViewer>("AuthoringScroll")
            .is_visible()
    );
    let input = |name: &str| {
        authoring_row(&workspace, name)
            .children()
            .to_vec()
            .into_iter()
            .find_map(|control| control.cast::<TextBox>())
            .unwrap()
    };
    let picker = |name: &str| {
        authoring_row(&workspace, name)
            .children()
            .to_vec()
            .into_iter()
            .find_map(|control| control.cast::<ComboBox>())
            .unwrap()
    };
    let original = session.borrow().document().source().to_owned();
    assert!(!authoring_row(&workspace, "BehaviorComparison").is_visible());
    picker("BehaviorTriggerKind").set_selected_index(1);
    assert!(authoring_row(&workspace, "BehaviorComparison").is_visible());
    assert!(!authoring_row(&workspace, "BehaviorTriggerType").is_visible());
    input("BehaviorTriggerSource").set_text(Some("{Binding IsEnabled}"));
    picker("BehaviorActionKind").set_selected_index(3);
    assert!(!authoring_row(&workspace, "BehaviorActionValue").is_visible());
    input("BehaviorActionMember").set_text(Some("active"));
    assert_eq!(
        session.borrow().document().source(),
        original,
        "Picker and field edits remain staged until Attach"
    );
    let attach = workspace
        .control()
        .get_control::<StackPanel>("AuthoringPanel")
        .children()
        .to_vec()
        .into_iter()
        .filter_map(|control| control.cast::<Button>())
        .find(|button| caption(button).as_deref() == Some("Attach behavior"))
        .unwrap();
    click(&attach);
    let behaviors = session.borrow().behaviors(target);
    assert_eq!(behaviors.len(), 1);
    assert_eq!(
        behaviors[0].trigger,
        BehaviorTrigger::Data {
            binding: "{Binding IsEnabled}".to_owned(),
            comparison: "Equal".to_owned(),
            value: "True".to_owned(),
            revert_on_false: false,
        }
    );
    assert_eq!(
        behaviors[0].actions,
        [BehaviorAction::AddClass {
            target: None,
            class: "active".to_owned()
        }]
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), original);
    click(&workspace.control().get_control::<Button>("CloseAuthoring"));
    assert!(
        workspace
            .control()
            .get_control::<Grid>("InspectorDesignPane")
            .is_visible()
    );
    assert!(
        !workspace
            .control()
            .get_control::<ScrollViewer>("AuthoringScroll")
            .is_visible()
    );
    window.close();
}

#[test]
fn native_timeline_controls_record_scrubbed_values_rescale_keys_and_author_curves() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Opacity="0.1" Width="100" Height="40" /></Canvas>"#,
    ).unwrap();
    let session = workspace.session();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    {
        let mut session = session.borrow_mut();
        session.select([target]);
        session
            .create_animation_program("Fade", target, "Button#Target", 4.0)
            .unwrap();
        session
            .record_keyframe(target, "Opacity", AnimationValue::Number(0.2))
            .unwrap();
        session.seek(4.0).unwrap();
        session
            .record_keyframe(target, "Opacity", AnimationValue::Number(0.8))
            .unwrap();
        session.seek(2.0).unwrap();
    }
    click(&workspace.control().get_control::<Button>("Keyframe"));
    let value = session
        .borrow()
        .timeline()
        .track(target, "Opacity")
        .unwrap()
        .keyframes
        .iter()
        .find(|key| key.time == 2.0)
        .unwrap()
        .value
        .clone();
    assert!(matches!(value, AnimationValue::Number(value) if (value - 0.5).abs() < 1e-9));

    click(
        &workspace
            .control()
            .get_control::<Button>("AnimationSettings"),
    );
    let panel = workspace
        .control()
        .get_control::<StackPanel>("AuthoringPanel");
    let children = panel.children().to_vec();
    let duration = children
        .iter()
        .filter_map(|control| control.cast::<Grid>())
        .find_map(|row| {
            let children = row.children().to_vec();
            children
                .iter()
                .any(|control| {
                    control
                        .cast::<TextBlock>()
                        .is_some_and(|label| label.text().as_deref() == Some("Duration, s"))
                })
                .then(|| {
                    children
                        .iter()
                        .find_map(|control| control.cast::<TextBox>())
                })
                .flatten()
        })
        .expect("Duration editor");
    duration.set_text(Some("2"));
    let apply = children
        .iter()
        .filter_map(|control| control.cast::<Button>())
        .find(|button| caption(button).as_deref() == Some("Apply animation settings"))
        .unwrap();
    click(&apply);
    assert_eq!(session.borrow().timeline().duration, 2.0);
    assert_eq!(
        session
            .borrow()
            .timeline()
            .track(target, "Opacity")
            .unwrap()
            .keyframes
            .iter()
            .map(|key| key.time)
            .collect::<Vec<_>>(),
        vec![0.0, 1.0, 2.0]
    );

    let row = keyframe_editor(&workspace);
    let children = row.children().to_vec();
    let curve = children
        .iter()
        .filter_map(|control| control.cast::<TextBox>())
        .rfind(|input| input.text().as_deref() == Some("LinearEasing"))
        .unwrap();
    curve.set_text(Some("0.1,0.2,0.8,0.9"));
    let apply = children
        .iter()
        .filter_map(|control| control.cast::<Button>())
        .rfind(|button| caption(button).as_deref() == Some("Apply keyframe"))
        .unwrap();
    click(&apply);
    assert_eq!(
        session
            .borrow()
            .timeline()
            .track(target, "Opacity")
            .unwrap()
            .keyframes
            .last()
            .unwrap()
            .easing,
        Easing::CubicBezier {
            x1: 0.1,
            y1: 0.2,
            x2: 0.8,
            y2: 0.9
        }
    );
    assert!(
        session
            .borrow()
            .document()
            .source()
            .contains("KeySpline=\"0.1,0.2,0.8,0.9\"")
    );
    assert!(matches!(
        session.borrow().preview_status(),
        PreviewStatus::Ready { .. }
    ));
}

#[test]
fn contextual_keyframe_edits_validate_then_commit_time_value_and_curve_in_one_undo_step() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Opacity="0.2" /></Canvas>"#,
    ).unwrap();
    let session = workspace.session();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    {
        let mut session = session.borrow_mut();
        session.select([target]);
        session
            .create_animation_program("Fade", target, "Button#Target", 4.0)
            .unwrap();
        session.seek(4.0).unwrap();
        session
            .record_keyframe(target, "Opacity", AnimationValue::Number(0.8))
            .unwrap();
    }
    workspace.refresh(true);
    let editor = keyframe_editor(&workspace);
    let time = keyframe_input(&editor, "KeyframeTime");
    let value = keyframe_input(&editor, "KeyframeValue");
    let curve = keyframe_input(&editor, "KeyframeCurve");
    let source = session.borrow().document().source().to_owned();
    let revision = session.borrow().document().revision();
    time.set_text(Some("invalid"));
    value.set_text(Some("0.6"));
    curve.set_text(Some("0.1,0.2,0.8,0.9"));
    apply_keyframe(&editor);
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(
        time.text().as_deref(),
        Some("invalid"),
        "Invalid input stays available for correction"
    );
    assert_eq!(value.text().as_deref(), Some("0.6"));
    time.set_text(Some("2"));
    apply_keyframe(&editor);
    assert_eq!(session.borrow().document().revision(), revision + 1);
    let track = session
        .borrow()
        .timeline()
        .track(target, "Opacity")
        .unwrap()
        .clone();
    assert_eq!(track.keyframes.len(), 2);
    let edited = track.keyframes.iter().find(|key| key.time == 2.0).unwrap();
    assert_eq!(edited.value, AnimationValue::Number(0.6));
    assert_eq!(
        edited.easing,
        Easing::CubicBezier {
            x1: 0.1,
            y1: 0.2,
            x2: 0.8,
            y2: 0.9
        }
    );
    session.borrow_mut().undo().unwrap();
    assert_eq!(
        session.borrow().document().source(),
        source,
        "One undo restores all three fields"
    );
}

#[test]
fn keyframe_button_records_the_selected_track_property_at_the_live_playhead() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Width="100" Opacity="0.2" /></Canvas>"#,
    ).unwrap();
    let session = workspace.session();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    {
        let mut session = session.borrow_mut();
        session.select([target]);
        session
            .create_animation_program("Motion", target, "Button#Target", 4.0)
            .unwrap();
        session
            .record_keyframe(target, "Opacity", AnimationValue::Number(0.2))
            .unwrap();
        session
            .record_keyframe(target, "Width", AnimationValue::Number(100.0))
            .unwrap();
        session.seek(4.0).unwrap();
        session
            .record_keyframe(target, "Width", AnimationValue::Number(200.0))
            .unwrap();
    }
    workspace.select_keyframe(target, "Width", 4.0).unwrap();
    assert!(workspace.select_keyframe(target, "Width", 3.0).is_err());
    session.borrow_mut().seek(2.0).unwrap();
    workspace.refresh(false);
    click(&workspace.control().get_control::<Button>("Keyframe"));
    let session = session.borrow();
    let keys = &session.timeline().track(target, "Width").unwrap().keyframes;
    assert_eq!(keys.len(), 3);
    assert_eq!(
        keys.iter().find(|key| key.time == 2.0).unwrap().value,
        AnimationValue::Number(150.0)
    );
    assert_eq!(
        session
            .timeline()
            .track(target, "Opacity")
            .unwrap()
            .keyframes
            .len(),
        2
    );
}

#[test]
fn timeline_viewport_changes_preserve_editor_drafts_and_duration_changes_reset_invalid_views() {
    use ferroui_base::input::Key;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Opacity="0.2" /></Canvas>"#,
    ).unwrap();
    let session = workspace.session();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    session.borrow_mut().select([target]);
    session
        .borrow_mut()
        .record_keyframe(target, "Opacity", AnimationValue::Number(0.2))
        .unwrap();
    workspace.refresh(true);
    let editor = keyframe_editor(&workspace);
    keyframe_input(&editor, "KeyframeValue").set_text(Some("0.65"));
    let source = session.borrow().document().source().to_owned();
    let ruler = timeline_ruler(&workspace);
    let previous_span = ruler.view_range().1 - ruler.view_range().0;
    timeline_key(&ruler, Key::OemPlus);
    assert!(ruler.view_range().1 - ruler.view_range().0 < previous_span);
    assert!(
        std::ptr::eq(&*editor, &*keyframe_editor(&workspace)),
        "Zoom preserves the editor instance and focus"
    );
    assert_eq!(
        keyframe_input(&editor, "KeyframeValue").text().as_deref(),
        Some("0.65")
    );
    assert_eq!(session.borrow().document().source(), source);
    session.borrow_mut().set_duration(1.0).unwrap();
    workspace.refresh(true);
    let ruler = timeline_ruler(&workspace);
    assert_eq!(ruler.view_range(), (0.0, 1.0));
    timeline_key(&ruler, Key::PageDown);
    assert_eq!(
        ruler.view_range(),
        (0.0, 1.0),
        "Panning after a duration reduction stays in bounds"
    );
}

#[test]
fn retained_keyframe_editor_cannot_modify_a_different_animation_at_the_same_revision() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Opacity="0.2" /></Canvas>"#,
    ).unwrap();
    let session = workspace.session();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    let (first, second) = {
        let mut session = session.borrow_mut();
        session.select([target]);
        let first = session
            .create_animation_program("Fade", target, "Button#Target", 4.0)
            .unwrap();
        session
            .record_keyframe(target, "Opacity", AnimationValue::Number(0.2))
            .unwrap();
        let second = session
            .create_animation_program("Pulse", target, "Button#Target", 4.0)
            .unwrap();
        session
            .record_keyframe(target, "Opacity", AnimationValue::Number(0.8))
            .unwrap();
        session.select_animation_program(first).unwrap();
        (first, second)
    };
    assert_ne!(first, second);
    workspace.refresh(true);
    let editor = keyframe_editor(&workspace);
    keyframe_input(&editor, "KeyframeValue").set_text(Some("0.95"));
    let revision = session.borrow().document().revision();
    session
        .borrow_mut()
        .select_animation_program(second)
        .unwrap();
    let source = session.borrow().document().source().to_owned();
    assert_eq!(session.borrow().document().revision(), revision);
    apply_keyframe(&editor);
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().active_animation_program(), Some(second));
    assert!(
        workspace
            .control()
            .get_control::<TextBlock>("Status")
            .text()
            .unwrap()
            .contains("timeline changed")
    );

    // Reparse twice so even the URI, revision, local target IDs and animation
    // IDs coincide. A retained editor must still belong to its syntax snapshot.
    let uri = session.borrow().document_uri().to_owned();
    let host = session.borrow().preview_host();
    let fresh_session = |retired_suffix: &str| {
        // Hot reload permits one live owner per URI. Retiring the old preview
        // releases its registration without changing the captured editor context.
        session
            .borrow_mut()
            .set_document_uri(format!("{uri}.{retired_suffix}"))
            .unwrap();
        crate::DesignerSession::with_uri(source.clone(), host.clone(), uri.clone()).unwrap()
    };
    let mut baseline = fresh_session("retired-1");
    let active = baseline.animation_programs()[0].id;
    baseline.select_animation_program(active).unwrap();
    *session.borrow_mut() = baseline;
    workspace.refresh(true);
    let editor = keyframe_editor(&workspace);
    keyframe_input(&editor, "KeyframeValue").set_text(Some("0.65"));
    let syntax = session.borrow().document().ast_snapshot();
    let revision = session.borrow().document().revision();
    let mut replacement = fresh_session("retired-2");
    replacement.select_animation_program(active).unwrap();
    assert_eq!(replacement.document_uri(), uri);
    assert_eq!(replacement.document().revision(), revision);
    assert_eq!(
        replacement.active_animation_program(),
        session.borrow().active_animation_program()
    );
    assert!(!std::ptr::eq(replacement.document().ast(), syntax.as_ref()));
    *session.borrow_mut() = replacement;
    apply_keyframe(&editor);
    assert_eq!(session.borrow().document().source(), source);
    assert_eq!(session.borrow().document().revision(), revision);
}

#[test]
fn new_animation_uses_selected_owner_and_unique_defaults_when_details_are_closed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="First" /><Button Name="Second" /></Canvas>"#,
    ).unwrap();
    let session = workspace.session();
    let first = session
        .borrow()
        .document()
        .ast()
        .find_name("First")
        .unwrap();
    let second = session
        .borrow()
        .document()
        .ast()
        .find_name("Second")
        .unwrap();
    for (owner, selector) in [(first, "Button#First"), (second, "Button#Second")] {
        session.borrow_mut().select([owner]);
        workspace.refresh(true);
        click(&workspace.control().get_control::<Button>("NewAnimation"));
        let active = session.borrow().active_animation_program().unwrap();
        let session = session.borrow();
        let program = session
            .animation_programs()
            .iter()
            .find(|program| program.id == active)
            .unwrap();
        assert_eq!(program.owner, owner);
        assert_eq!(program.selector, selector);
        assert!(program.diagnostics.is_empty(), "{:?}", program.diagnostics);
    }
    let programs = session.borrow().animation_programs().to_vec();
    assert_eq!(programs.len(), 2);
    assert_ne!(programs[0].name, programs[1].name);
    assert!(
        programs
            .iter()
            .all(|program| !program.name.trim().is_empty())
    );
    click(
        &workspace
            .control()
            .get_control::<Button>("DuplicateAnimation"),
    );
    let programs = session.borrow().animation_programs().to_vec();
    assert_eq!(programs.len(), 3);
    let names: std::collections::BTreeSet<_> =
        programs.iter().map(|program| &program.name).collect();
    assert_eq!(
        names.len(),
        3,
        "Duplicate uses a new name when the details still display the original"
    );
}
