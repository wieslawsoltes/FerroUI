//! Native selection commands must preserve curve/value pairs and remain one source edit.
use super::*;
use ferroui_base::input::KeyEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn workspace() -> (Rc<DesignerWorkspace>, NodeId) {
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Width="100" Height="30" Opacity="0.2" /></Canvas>"#,
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
            .create_animation_program("Motion", target, "Button#Target", 6.0)
            .unwrap();
        for time in [0.0, 1.0, 2.0, 4.0, 6.0] {
            session.seek(time).unwrap();
            session
                .record_keyframe(target, "Opacity", AnimationValue::Number(time / 10.0))
                .unwrap();
            session
                .record_keyframe(target, "Width", AnimationValue::Number(100.0 + time * 10.0))
                .unwrap();
        }
        session
            .set_keyframe_easing(
                target,
                "Opacity",
                1.0,
                Easing::CubicBezier {
                    x1: 0.1,
                    y1: 0.2,
                    x2: 0.8,
                    y2: 0.9,
                },
            )
            .unwrap();
    }
    workspace.refresh(true);
    (workspace, target)
}

fn select(workspace: &Rc<DesignerWorkspace>, target: NodeId) {
    workspace
        .select_keyframes(&[
            (target, "Opacity".into(), 1.0),
            (target, "Opacity".into(), 2.0),
            (target, "Width".into(), 1.0),
            (target, "Width".into(), 2.0),
        ])
        .unwrap();
}

fn times(workspace: &DesignerWorkspace, target: NodeId, property: &str) -> Vec<f64> {
    workspace
        .session
        .borrow()
        .timeline()
        .track(target, property)
        .unwrap()
        .keyframes
        .iter()
        .map(|key| key.time)
        .collect()
}

fn key(lane: &TimelineLane, key: Key, modifiers: KeyModifiers) {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.key_modifiers = modifiers;
    event.set_routed_event(Some(InputElement::key_down_event()));
    lane.raise_event(&event);
    assert!(event.handled());
}

fn selected_lane(workspace: &DesignerWorkspace) -> Ref<TimelineLane> {
    workspace
        .timeline_lanes
        .borrow()
        .iter()
        .filter_map(WeakRef::upgrade)
        .find(|lane| !lane.is_ruler() && !lane.selected_keys().is_empty())
        .unwrap()
}

#[test]
fn multi_track_shift_crosses_original_times_atomically_and_undo_restores_curves() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, target) = workspace();
    select(&workspace, target);
    let original = workspace.session.borrow().document().source().to_owned();
    let revision = workspace.session.borrow().document().revision();
    workspace.move_selected_keyframes(1.0).unwrap();
    assert_eq!(
        workspace.session.borrow().document().revision(),
        revision + 1
    );
    for property in ["Opacity", "Width"] {
        assert_eq!(
            times(&workspace, target, property),
            [0.0, 2.0, 3.0, 4.0, 6.0]
        );
    }
    assert_eq!(workspace.selected_keyframes().len(), 4);
    let session = workspace.session.borrow();
    let moved = &session
        .timeline()
        .track(target, "Opacity")
        .unwrap()
        .keyframes[1];
    assert_eq!(moved.time, 2.0);
    assert_eq!(moved.value, AnimationValue::Number(0.1));
    assert_eq!(
        moved.easing,
        Easing::CubicBezier {
            x1: 0.1,
            y1: 0.2,
            x2: 0.8,
            y2: 0.9
        }
    );
    drop(session);
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), original);
}

#[test]
fn group_collision_or_invalid_offset_rejects_every_track_without_an_undo_entry() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, target) = workspace();
    select(&workspace, target);
    let original = workspace.session.borrow().document().source().to_owned();
    let revision = workspace.session.borrow().document().revision();
    for delta in [2.0, -2.0, f64::INFINITY, f64::NAN] {
        assert!(workspace.move_selected_keyframes(delta).is_err());
        assert_eq!(workspace.session.borrow().document().source(), original);
        assert_eq!(workspace.session.borrow().document().revision(), revision);
    }
    assert!(workspace.duplicate_selected_keyframes(2.0).is_err());
    assert_eq!(workspace.session.borrow().document().source(), original);
}

#[test]
fn duplicate_and_delete_preserve_batch_selection_and_each_is_one_undo_step() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, target) = workspace();
    select(&workspace, target);
    let original = workspace.session.borrow().document().source().to_owned();
    workspace.duplicate_selected_keyframes(0.5).unwrap();
    for property in ["Opacity", "Width"] {
        assert_eq!(
            times(&workspace, target, property),
            [0.0, 1.0, 1.5, 2.0, 2.5, 4.0, 6.0]
        );
    }
    assert!(
        workspace
            .selected_keyframes()
            .iter()
            .all(|(_, _, time)| [1.5, 2.5].contains(time))
    );
    let duplicated = workspace.session.borrow().document().source().to_owned();
    workspace.remove_selected_keyframes().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), original);
    assert!(workspace.selected_keyframes().is_empty());
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), duplicated);
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), original);
}

#[test]
fn keyboard_group_nudge_keeps_all_lanes_selected_at_native_frame_precision() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, target) = workspace();
    select(&workspace, target);
    workspace.set_timeline_frame_rate(Some(30));
    let lane = selected_lane(&workspace);
    let original = workspace.session.borrow().document().source().to_owned();
    key(&lane, Key::Right, KeyModifiers::NONE);
    let selection = workspace.selected_keyframes();
    assert_eq!(selection.len(), 4);
    assert!(selection.iter().all(|(_, _, time)| {
        [1.0333333, 2.0333333]
            .iter()
            .any(|expected| (time - expected).abs() < 1e-9)
    }));
    assert_eq!(workspace.timeline_frame_rate(), Some(30));
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), original);
}

#[test]
fn retained_lane_cannot_edit_after_a_document_revision_or_program_switch() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, target) = workspace();
    select(&workspace, target);
    let lane = selected_lane(&workspace);
    workspace
        .session
        .borrow_mut()
        .set_property(&[target], "Content", "New text")
        .unwrap();
    let original = workspace.session.borrow().document().source().to_owned();
    key(&lane, Key::Delete, KeyModifiers::NONE);
    assert_eq!(workspace.session.borrow().document().source(), original);
    assert!(
        workspace
            .status
            .text()
            .unwrap()
            .contains("timeline changed")
    );
    workspace.refresh(true);
    let lane = selected_lane(&workspace);
    workspace
        .session
        .borrow_mut()
        .create_animation_program("Other", target, "Button#Target", 6.0)
        .unwrap();
    let original = workspace.session.borrow().document().source().to_owned();
    key(&lane, Key::Right, KeyModifiers::NONE);
    assert_eq!(workspace.session.borrow().document().source(), original);
}

#[test]
fn selection_and_frame_grid_changes_do_not_author_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, target) = workspace();
    let original = workspace.session.borrow().document().source().to_owned();
    let revision = workspace.session.borrow().document().revision();
    select(&workspace, target);
    workspace.set_timeline_frame_rate(Some(60));
    assert!(
        workspace
            .timeline_lanes
            .borrow()
            .iter()
            .filter_map(WeakRef::upgrade)
            .all(|lane| lane.frame_rate() == Some(60))
    );
    workspace.set_timeline_frame_rate(Some(120));
    let picker = workspace
        .timeline_ui
        .borrow()
        .snap_picker
        .as_ref()
        .and_then(WeakRef::upgrade)
        .unwrap();
    assert_eq!(
        picker
            .selected_item()
            .and_then(|item| item.downcast_ref::<String>().cloned())
            .as_deref(),
        Some("120 fps")
    );
    picker.set_selected_index(0);
    assert_eq!(workspace.timeline_frame_rate(), None);
    let custom_index = picker.items().count() as i32 - 1;
    picker.set_selected_index(custom_index);
    assert_eq!(workspace.timeline_frame_rate(), Some(120));
    workspace.set_timeline_frame_rate(Some(0));
    assert_eq!(workspace.timeline_frame_rate(), None);
    workspace.select_keyframes(&[]).unwrap();
    assert!(workspace.selected_keyframes().is_empty());
    assert_eq!(workspace.session.borrow().document().revision(), revision);
    assert_eq!(workspace.session.borrow().document().source(), original);
}

#[test]
fn easing_presets_stage_the_curve_then_apply_in_one_undo_step() {
    use ferroui_base::interactivity::RoutedEventArgs;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (workspace, target) = workspace();
    workspace.select_keyframe(target, "Opacity", 1.0).unwrap();
    let editor = workspace
        .tracks
        .children()
        .to_vec()
        .iter()
        .filter_map(|control| control.cast::<StackPanel>())
        .find(|row| {
            row.children()
                .to_vec()
                .iter()
                .any(|control| control.name().as_deref() == Some("KeyframeEasingPreset"))
        })
        .unwrap();
    let children = editor.children().to_vec();
    let preset = children
        .iter()
        .filter_map(|control| control.cast::<ComboBox>())
        .find(|control| control.name().as_deref() == Some("KeyframeEasingPreset"))
        .unwrap();
    let apply = children
        .iter()
        .filter_map(|control| control.cast::<Button>())
        .find(|control| control.name().as_deref() == Some("ApplyKeyframe"))
        .unwrap();
    let original = workspace.session.borrow().document().source().to_owned();
    preset.set_selected_index(2);
    assert_eq!(
        workspace.session.borrow().document().source(),
        original,
        "Choosing a curve stages the editor without changing the document"
    );
    apply.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(
        workspace
            .session
            .borrow()
            .timeline()
            .track(target, "Opacity")
            .unwrap()
            .keyframes
            .iter()
            .find(|key| key.time == 1.0)
            .unwrap()
            .easing,
        Easing::CubicBezier {
            x1: 0.0,
            y1: 0.0,
            x2: 0.58,
            y2: 1.0
        }
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), original);
}

#[test]
fn applying_a_keyframe_from_the_keyboard_restores_focus_to_the_replacement_field() {
    use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
    use ferroui_controls::Window;
    let _app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    );
    let (workspace, target) = workspace();
    let window = Window::new();
    window.set_width(1280.0);
    window.set_height(820.0);
    window.set_content(Some(Control::boxed(&workspace.control())));
    window.show();
    workspace.select_keyframe(target, "Opacity", 1.0).unwrap();
    window.update_layout();
    let value_field = || {
        workspace
            .tracks
            .children()
            .to_vec()
            .into_iter()
            .filter_map(|control| control.cast::<StackPanel>())
            .flat_map(|row| row.children().to_vec())
            .filter_map(|control| control.cast::<TextBox>())
            .find(|input| input.name().as_deref() == Some("KeyframeValue"))
            .unwrap()
    };
    let previous = value_field();
    assert!(previous.focus());
    previous.set_text(Some("0.35"));
    previous.set_selection_start(1);
    previous.set_selection_end(4);
    let original = workspace.session.borrow().document().source().to_owned();
    let mut event = KeyEventArgs::new();
    event.key = Key::Enter;
    event.set_routed_event(Some(InputElement::key_down_event()));
    previous.raise_event(&event);
    assert!(event.handled());
    window.update_layout();
    let current = value_field();
    assert!(!std::ptr::eq(&*previous, &*current));
    assert!(
        current.is_focused(),
        "Accepted keyframe edits retain the active field for consecutive keyboard editing"
    );
    assert_eq!(current.selection_start(), 1);
    assert_eq!(current.selection_end(), 4);
    assert_eq!(
        workspace
            .session
            .borrow()
            .timeline()
            .track(target, "Opacity")
            .unwrap()
            .keyframes
            .iter()
            .find(|key| key.time == 1.0)
            .unwrap()
            .value,
        AnimationValue::Number(0.35)
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), original);
    window.close();
}
