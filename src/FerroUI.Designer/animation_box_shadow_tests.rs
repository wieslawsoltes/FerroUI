use crate::{DesignerSession, animation_integration_tests::attach_preview};
use ferroui_base::media::BoxShadows;
use ferroui_controls::{
    Border, ContentControl,
    testing::{TestServices, UnitTestApplication},
};
use ferroui_designer_animation::{CurveMetric, CurvePoint, CurveWindow, TrackCurveChannel};

#[test]
fn layered_box_shadows_share_native_curves_recording_and_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r##"<Border xmlns="https://github.com/ferroui" Name="Card" Width="160" Height="90"
  Background="White" BoxShadow="0 2 4 -2 #40263452, 4 8 12 2 #80325AC0">
  <Border.Styles><Style Selector="Border#Card"><Style.Animations>
    <Animation Duration="0:0:2">
      <KeyFrame Cue="50%"><Setter Property="BoxShadow" Value="8 10 20 4 rgba(38, 52, 82, 0.5), inset 14 18 28 10 #C0325AC0"/></KeyFrame>
      <KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="2 4 8 0 #40263452"/></KeyFrame>
    </Animation>
  </Style.Animations></Style></Border.Styles>
</Border>"##;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let _window = attach_preview(&session);
    let target = session.document().ast().root();
    let program = &session.animation_programs()[0];
    let timeline = program.timeline.as_ref().expect("typed shadow animation");
    let track = timeline.track(target, "BoxShadow").unwrap();
    let channel: TrackCurveChannel = "box_shadow/2/spread".parse().unwrap();
    assert_eq!(channel.label(), "Shadow 2 · Spread (px)");
    assert!(
        TrackCurveChannel::for_track(track, timeline.duration, timeline.fill).contains(&channel)
    );

    let id = program.id;
    let window = CurveWindow {
        start: 0.0,
        end: 2.0,
    };
    let points = session
        .animation_curve_channel(id, target, "BoxShadow", channel.clone(), window)
        .unwrap();
    assert!(points.contains(&CurvePoint::new(0.5, 6.0)));
    assert!(
        points
            .iter()
            .any(|point| point.time == 1.0 && point.break_before)
    );
    assert!(points.iter().all(|point| point.time < 2.0));
    let velocity = CurveMetric::Velocity.measure(points.clone()).unwrap();
    assert!(
        velocity
            .iter()
            .any(|point| (point.time - 0.5).abs() < 0.05 && (point.value - 8.0).abs() < 1e-8)
    );

    let border = session
        .preview
        .visual(target)
        .unwrap()
        .cast::<Border>()
        .unwrap();
    let base = border.box_shadow();
    session.scrub_animation_program(id, 0.5).unwrap();
    let preview = border.box_shadow();
    assert_eq!(preview.get(1).spread, 6.0);
    assert!(!preview.get(1).is_inset);
    assert_eq!(
        session
            .animation_curve_channel(id, target, "BoxShadow", channel, window)
            .unwrap(),
        points
    );
    let sample = session
        .sample_keyframe(id, target, "BoxShadow", 0.5)
        .unwrap();
    assert_eq!(BoxShadows::parse(&sample.value).unwrap(), preview);
    session.stop();
    assert_eq!(border.box_shadow(), base);

    session
        .record_effective_keyframe(target, "BoxShadow", 0.5)
        .unwrap();
    let edited = session.document().source();
    assert_eq!(edited.matches("<KeyFrame ").count(), 3);
    assert!(edited.contains("rgba(38, 52, 82, 0.5)"));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}

#[test]
fn box_shadow_add_owner_samples_the_empty_base_while_scrubbed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<ContentPresenter xmlns="https://github.com/ferroui" Name="Card" Width="180" Height="90">
      <ContentPresenter.Styles><Style Selector="ContentPresenter#Card"><Style.Animations>
        <Animation Duration="0:0:2">
          <KeyFrame Cue="25%"><Setter Property="BoxShadow" Value="0 0 4 2 Red"/></KeyFrame>
          <KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="0 0 8 8 Blue"/></KeyFrame>
        </Animation>
      </Style.Animations></Style></ContentPresenter.Styles>
    </ContentPresenter>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let _window = attach_preview(&session);
    let program = session.animation_programs()[0].id;
    let target = session.document().ast().root();
    let presenter = session
        .preview
        .visual(target)
        .unwrap()
        .cast::<ferroui_controls::presenters::ContentPresenter>()
        .unwrap();
    let window = CurveWindow {
        start: 0.0,
        end: 2.0,
    };
    let channel = "box_shadow/1/spread".parse::<TrackCurveChannel>().unwrap();
    let points = session
        .animation_curve_channel(program, target, "BoxShadow", channel.clone(), window)
        .unwrap();
    assert_eq!(points.first(), Some(&CurvePoint::new(0.5, 2.0)));
    assert_eq!(presenter.box_shadow().count(), 0);
    session.scrub_animation_program(program, 1.0).unwrap();
    assert_eq!(presenter.box_shadow().get(0).spread, 4.0);
    assert_eq!(
        session
            .animation_curve_channel(program, target, "BoxShadow", channel, window)
            .unwrap(),
        points
    );
    session.stop();
    assert_eq!(presenter.box_shadow().count(), 0);
}
