use crate::{DesignerSession, animation_integration_tests::attach_preview};
use ferroui_base::media::{RotateTransform, TransformGroup, TranslateTransform};
use ferroui_controls::{
    ContentControl,
    testing::{TestServices, UnitTestApplication},
};
use ferroui_designer_animation::{AnimationValue, CurvePoint, CurveWindow};

#[test]
fn transform_tracks_sample_the_live_components_and_keep_implicit_keys() {
    let _application = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui">
      <Canvas.Styles><Style Selector="Border#Card"><Style.Animations>
        <Animation Duration="0:0:2">
          <KeyFrame Cue="50%">
            <Setter Property="RotateTransform.Angle" Value="80"/>
            <Setter Property="TranslateTransform.X" Value="44"/>
          </KeyFrame>
          <KeyFrame Cue="100%"><Setter Property="RotateTransform.Angle" Value="100"/></KeyFrame>
        </Animation>
      </Style.Animations></Style></Canvas.Styles>
      <Border Name="Card" Width="180" Height="100" Background="Cyan">
        <Border.RenderTransform><TransformGroup>
          <RotateTransform Angle="20"/>
          <TranslateTransform X="4"/>
        </TransformGroup></Border.RenderTransform>
      </Border>
    </Canvas>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let _window = attach_preview(&session);
    let (program, target) = (
        session.animation_programs()[0].id,
        session.document().ast().find_name("Card").unwrap(),
    );
    let curve_window = CurveWindow {
        start: 0.0,
        end: 2.0,
    };
    for (property, value) in [
        ("RotateTransform.Angle", 50.0),
        ("TranslateTransform.X", 24.0),
    ] {
        let points = session
            .animation_curve(program, target, property, curve_window)
            .unwrap();
        assert!(points.contains(&CurvePoint::new(0.5, value)));
    }
    let visual = session.preview.visual(target).unwrap();
    let transform = visual.render_transform().unwrap();
    let group = transform
        .as_object()
        .unwrap()
        .downcast_ref::<TransformGroup>()
        .unwrap();
    let rotation = group.children().get(0).cast::<RotateTransform>().unwrap();
    let translation = group
        .children()
        .get(1)
        .cast::<TranslateTransform>()
        .unwrap();
    assert_eq!((rotation.angle(), translation.x()), (20.0, 4.0));
    session.scrub_animation_program(program, 0.5).unwrap();
    assert_eq!((rotation.angle(), translation.x()), (50.0, 24.0));
    session.stop();
    assert_eq!((rotation.angle(), translation.x()), (20.0, 4.0));

    session
        .record_effective_keyframe(target, "RotateTransform.Angle", 0.5)
        .unwrap();
    let timeline = session.timeline();
    let rotation = timeline.track(target, "RotateTransform.Angle").unwrap();
    assert_eq!(rotation.base_value, Some(AnimationValue::Number(20.0)));
    assert_eq!(
        rotation
            .keyframes
            .iter()
            .map(|key| key.time)
            .collect::<Vec<_>>(),
        [0.5, 1.0, 2.0]
    );
    assert_eq!(rotation.keyframes[0].value, AnimationValue::Number(50.0));
    assert!(
        session
            .document()
            .source()
            .contains(r#"<RotateTransform Angle="20"/>"#)
    );
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}
