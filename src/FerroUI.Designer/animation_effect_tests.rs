use crate::{animation_integration_tests::attach_preview, DesignerSession};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::ContentControl;
use ferroui_designer_animation::{CurveChannel, CurveWindow};

#[test]
fn studio_effect_graph_and_key_edit_share_native_values_and_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui">
      <Canvas.Styles><Style Selector="Border#Card"><Style.Animations>
        <Animation Duration="0:0:2">
          <KeyFrame Cue="25%"><Setter Property="Effect" Value="blur(4)"/></KeyFrame>
          <KeyFrame Cue="75%"><Setter Property="Effect" Value="blur(12)"/></KeyFrame>
        </Animation>
      </Style.Animations></Style></Canvas.Styles>
      <Border Name="Card" Width="180" Height="100" Background="Cyan"/>
    </Canvas>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let _window = attach_preview(&session);
    let id = session.animation_programs()[0].id;
    let target = session.document().ast().find_name("Card").unwrap();
    let window = CurveWindow {
        start: 0.0,
        end: 2.0,
    };
    let points = session
        .animation_curve_channel(id, target, "Effect", CurveChannel::BlurRadius, window)
        .unwrap();
    assert_eq!(
        points.iter().find(|point| point.time == 1.0).unwrap().value,
        8.0
    );
    assert!(points.first().unwrap().time > 0.0 && points.last().unwrap().time < 2.0);
    session.scrub_animation_program(id, 1.0).unwrap();
    let effect = session.preview.visual(target).unwrap().effect().unwrap();
    assert_eq!(effect.as_blur_effect().unwrap().radius(), 8.0);
    let draft = session.sample_keyframe(id, target, "Effect", 1.0).unwrap();
    session
        .edit_program_keyframe(id, target, "Effect", draft.edit())
        .unwrap();
    assert_eq!(session.document().source().matches("<KeyFrame ").count(), 3);
    assert!(session.document().source().contains("Value=\"blur(8)\""));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}
