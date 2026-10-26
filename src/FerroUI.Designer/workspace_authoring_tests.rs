//! End-to-end checks of native timeline controls and the source transactions they trigger.
use crate::{DesignerWorkspace, PreviewStatus};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, StackPanel, TextBlock, TextBox};
use ferroui_designer_animation::{AnimationValue, Easing};

fn click(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}
fn caption(button: &Button) -> Option<String> {
    button
        .content()
        .and_then(|value| value.downcast_ref::<String>().cloned())
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
        .windows(2)
        .find_map(|pair| {
            pair[0]
                .cast::<TextBlock>()
                .filter(|label| label.text().as_deref() == Some("Duration (seconds)"))
                .and_then(|_| pair[1].cast::<TextBox>())
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

    let tracks = workspace.control().get_control::<StackPanel>("Tracks");
    let row = tracks
        .children()
        .to_vec()
        .into_iter()
        .filter_map(|control| control.cast::<StackPanel>())
        .find(|row| {
            row.children()
                .to_vec()
                .iter()
                .filter_map(|control| control.cast::<Button>())
                .any(|button| caption(&button).as_deref() == Some("Apply curve"))
        })
        .unwrap();
    let children = row.children().to_vec();
    let curve = children
        .iter()
        .filter_map(|control| control.cast::<TextBox>())
        .filter(|input| input.text().as_deref() == Some("LinearEasing"))
        .next_back()
        .unwrap();
    curve.set_text(Some("0.1,0.2,0.8,0.9"));
    let apply = children
        .iter()
        .filter_map(|control| control.cast::<Button>())
        .filter(|button| caption(button).as_deref() == Some("Apply curve"))
        .next_back()
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
