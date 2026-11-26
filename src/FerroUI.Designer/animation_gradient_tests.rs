use crate::{DesignerSession, animation_integration_tests::attach_preview};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, ContentControl};
use ferroui_designer_animation::{
    AnimationBrush, AnimationValue, CurveWindow, TrackCurveChannel, ValueKind,
};

const SOURCE: &str = r##"<Border xmlns="https://github.com/ferroui" Name="Card"
  Width="320" Height="180" Background="#102030">
  <!-- Keep the card -->
  <Border.Styles><Style Selector="Border#Card"><Style.Animations>
    <Animation Duration="0:0:2" FillMode="Forward">
      <KeyFrame Cue="0%"><Setter Property="Background"><Setter.Value>
        <LinearGradientBrush StartPoint="0%,0%" EndPoint="100%,0%">
          <GradientStop Offset="0.2" Color="#FF183055"/><GradientStop Offset="1" Color="#FF335589"/>
        </LinearGradientBrush>
      </Setter.Value></Setter></KeyFrame>
      <KeyFrame Cue="50%"><Setter Property="Background">
        <LinearGradientBrush StartPoint="40%,0%" EndPoint="100%,100%" Opacity="0.6">
          <GradientStop Offset="0.6" Color="#FF448BC1"/><GradientStop Offset="1" Color="#FF51AE8C"/>
        </LinearGradientBrush>
      </Setter></KeyFrame>
      <KeyFrame Cue="100%"><Setter Property="Background">
        <RadialGradientBrush Center="35%,60%" RadiusX="45%" RadiusY="30%">
          <GradientStop Offset="0.4" Color="#FF51AE8C"/><GradientStop Offset="1" Color="#FF1A5050"/>
        </RadialGradientBrush>
      </Setter></KeyFrame>
    </Animation>
  </Style.Animations></Style></Border.Styles>
</Border>"##;

#[test]
fn gradient_curves_and_recording_use_the_loaded_brush() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(SOURCE, ContentControl::new()).unwrap();
    let _window = attach_preview(&session);
    let program = session.active_animation_program().unwrap();
    let target = session.document().ast().root();
    let track = session.timeline().track(target, "Background").unwrap();
    assert!(
        track
            .keyframes
            .iter()
            .all(|key| key.value.kind() == ValueKind::Brush)
    );
    let read_curve = |session: &DesignerSession, name: &str| {
        let channel = name.parse::<TrackCurveChannel>().unwrap();
        session
            .animation_curve_channel(
                program,
                target,
                "Background",
                channel,
                CurveWindow {
                    start: 0.0,
                    end: 2.0,
                },
            )
            .unwrap()
    };
    let offset = read_curve(&session, "gradient_stop/1/stop_offset");
    assert!(
        offset
            .iter()
            .any(|point| point.time == 0.5 && (point.value - 0.4).abs() < 1e-12)
    );
    assert!(
        offset
            .iter()
            .any(|point| point.time == 1.5 && point.break_before)
    );
    let start = read_curve(&session, "gradient/start_point/percent_x");
    assert!(
        start
            .iter()
            .any(|point| point.time == 0.5 && (point.value - 20.0).abs() < 1e-12)
    );
    assert!(start.iter().all(|point| point.time < 1.5));
    let radius = read_curve(&session, "gradient/radius_y/percent");
    assert!(
        radius
            .iter()
            .all(|point| point.time >= 1.5 && (point.value - 30.0).abs() < 1e-12)
    );
    session.seek(0.5).unwrap();
    let border = session
        .preview
        .visual(target)
        .unwrap()
        .cast::<Border>()
        .unwrap();
    let before = border.background().unwrap();
    let actual = AnimationBrush::from_native(&*before).unwrap();
    let draft = session
        .sample_keyframe(program, target, "Background", 0.5)
        .unwrap();
    let sampled = session
        .plan_program_keyframe(program, target, "Background", draft.edit())
        .unwrap();
    let expected = &sampled.track(target, "Background").unwrap().keyframes[1].value;
    assert_eq!(expected, &AnimationValue::Brush(actual));
    session
        .record_effective_keyframe(target, "Background", 0.5)
        .unwrap();
    assert!(session.authored_source().contains("<Setter.Value"));
    assert!(session.authored_source().contains("<!-- Keep the card -->"));
    session.undo().unwrap();
    assert_eq!(session.authored_source(), SOURCE);
}

#[test]
fn gradient_parser_rejects_computed_values_and_handles_empty_endpoints() {
    use ferroui_base::animation::animators::GradientBrushAnimator;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    assert!(
        AnimationBrush::parse(
            r#"<LinearGradientBrush><GradientStop Color="{Binding Tint}"/></LinearGradientBrush>"#
        )
        .is_err()
    );
    let empty = AnimationBrush::parse("<ConicGradientBrush Angle='30'/>").unwrap();
    let full = AnimationBrush::parse(
        "<ConicGradientBrush Angle='90'><GradientStop Color='Red'/></ConicGradientBrush>",
    )
    .unwrap();
    let a = Some(empty.to_native());
    let b = Some(full.to_native());
    let read = |progress| {
        let native = GradientBrushAnimator::interpolate_core(progress, &a, &b).unwrap();
        AnimationBrush::from_native(&*native)
            .unwrap()
            .as_gradient()
            .unwrap()
            .stops
            .len()
    };
    assert_eq!(read(0.25), 0);
    assert_eq!(read(0.75), 1);
}
