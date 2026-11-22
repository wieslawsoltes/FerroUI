use crate::{DesignerSession, animation_integration_tests::attach_preview};
use ferroui_base::Visual;
use ferroui_base::animation::{Animation, KeyFrame, SignedDuration};
use ferroui_base::media::{ITransform, RotateTransform, transformation::TransformOperations};
use ferroui_base::reactive::Observable;
use ferroui_base::styling::Setter;
use ferroui_controls::{
    Border, ContentControl,
    testing::{TestServices, UnitTestApplication},
};
use ferroui_designer_animation::{CurveMetric, CurveWindow, TrackCurveChannel};
use std::rc::Rc;

const PROPERTY: &str = "RenderTransform";
const WINDOW: CurveWindow = CurveWindow {
    start: 0.0,
    end: 2.0,
};

#[test]
fn transform_list_graph_and_recorded_key_follow_the_native_pose() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Border xmlns="https://github.com/ferroui" Name="Card" Width="160" Height="90"
  RenderTransform="translate(8px, 10px) rotate(20deg) scale(1.05)">
  <!-- Preserve the card note -->
  <Border.Styles><Style Selector="Border#Card"><Style.Animations>
    <Animation Duration="0:0:2" FillMode="Forward">
      <KeyFrame Cue="50%"><Setter Property="RenderTransform" Value="translate(32px, 18px) rotate(60deg) scale(1.4, 1.2)"/></KeyFrame>
      <KeyFrame Cue="75%"><Setter Property="RenderTransform" Value="translate(50px, 22px) skew(8deg, 4deg)"/></KeyFrame>
      <KeyFrame Cue="100%"><Setter Property="RenderTransform" Value="translate(68px, 28px) rotate(30deg) scale(1.2)"/></KeyFrame>
    </Animation>
  </Style.Animations></Style></Border.Styles>
</Border>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let _window = attach_preview(&session);
    let program = &session.animation_programs()[0];
    let id = program.id;
    let track_target = session.document().ast().root();
    let timeline = program.timeline.as_ref().expect("typed transform list");
    let track = timeline.track(track_target, PROPERTY).unwrap();
    let rotation: TrackCurveChannel = "transform_operation/2/rotate/angle".parse().unwrap();
    assert!(
        TrackCurveChannel::for_track(track, timeline.duration, timeline.fill).contains(&rotation)
    );
    let curve = |session: &DesignerSession, channel: TrackCurveChannel| {
        session
            .animation_curve_channel(id, track_target, PROPERTY, channel, WINDOW)
            .unwrap()
    };
    let angles = curve(&session, rotation.clone());
    assert!((angles.iter().find(|point| point.time == 0.5).unwrap().value - 40.0).abs() < 1e-10);
    assert!(
        angles
            .iter()
            .all(|point| point.time <= 1.0 || point.time == 2.0)
    );
    assert!(angles.last().unwrap().break_before);
    let velocity = CurveMetric::Velocity.measure(angles.clone()).unwrap();
    assert!(
        velocity
            .iter()
            .any(|point| (point.time - 0.5).abs() < 0.05 && (point.value - 40.0).abs() < 1e-8)
    );
    let matrix = curve(&session, "transform_matrix/m31".parse().unwrap());
    assert!(matrix.iter().any(|point| point.time == 1.25));

    let card = session.preview.visual(track_target).unwrap();
    let base = card.render_transform().unwrap();
    session.scrub_animation_program(id, 0.5).unwrap();
    let pose = card.render_transform().unwrap().value();
    assert_eq!(
        matrix.iter().find(|point| point.time == 0.5).unwrap().value,
        pose.m31
    );
    assert_eq!(curve(&session, rotation), angles);
    let sampled = session
        .sample_keyframe(id, track_target, PROPERTY, 0.5)
        .unwrap();
    assert_eq!(
        TransformOperations::parse(&sampled.value).unwrap().value(),
        pose
    );
    session.stop();
    assert!(Rc::ptr_eq(&card.render_transform().unwrap(), &base));
    session
        .record_effective_keyframe(track_target, PROPERTY, 0.5)
        .unwrap();
    let edited = session.document().source();
    assert_eq!(edited.matches("<KeyFrame ").count(), 4);
    assert!(edited.contains(r#"RenderTransform="translate(8px, 10px) rotate(20deg) scale(1.05)""#));
    assert!(edited.contains("<!-- Preserve the card note -->"));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}

#[test]
fn transform_list_and_component_tracks_are_rejected_atomically() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let target = Border::new();
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(1.0));
    let list: Rc<dyn ITransform> = TransformOperations::parse("rotate(30deg)").unwrap();
    animation.children().add(KeyFrame::with_key_time(
        SignedDuration::from_seconds(1.0),
        [
            Setter::new(Visual::render_transform_property(), Some(list)) as _,
            Setter::new(RotateTransform::angle_property(), 60.0) as _,
        ],
    ));
    let error = animation
        .try_apply(&target, None, Observable::single_value(true), None, true)
        .err()
        .unwrap();
    assert!(error.to_string().contains("whole RenderTransform"));
    assert!(target.render_transform().is_none());
    assert!(!target.is_animating(Visual::render_transform_property()));
}
