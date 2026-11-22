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
    session.record_effective_keyframe(target, "Effect", 1.0).unwrap();
    assert_eq!(session.document().source().matches("<KeyFrame ").count(), 3);
    assert!(session.document().source().contains("Value=\"blur(8)\""));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}

#[test]
fn studio_directional_shadow_keys_follow_native_angles_and_keep_the_value_element() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui">
      <Canvas.Styles><Style Selector="Border#Card"><Style.Animations>
        <Animation Duration="0:0:2" FillMode="Both">
          <KeyFrame Cue="0%"><Setter Property="Effect"><Setter.Value>
            <DropShadowDirectionEffect Direction="30" ShadowDepth="4" BlurRadius="2" Opacity="0.25" Color="Purple"/>
          </Setter.Value></Setter></KeyFrame>
          <KeyFrame Cue="100%"><Setter Property="Effect">
            <!-- Keep the authored note -->
            <DropShadowDirectionEffect Direction="150" ShadowDepth="20" BlurRadius="10" Opacity="0.75" Color="Purple"/>
          </Setter></KeyFrame>
        </Animation>
      </Style.Animations></Style></Canvas.Styles>
      <Border Name="Card" Width="180" Height="100" Background="White"/>
    </Canvas>"#;
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let _preview_window = attach_preview(&session);
    let program = &session.animation_programs()[0];
    assert!(program.timeline.is_some(), "{:?}", program.diagnostics);
    let id = program.id;
    let target = session.document().ast().find_name("Card").unwrap();
    let window = CurveWindow {
        start: 0.0,
        end: 2.0,
    };
    for (channel, expected) in [
        (CurveChannel::Direction, 90.0),
        (CurveChannel::ShadowDepth, 12.0),
        (CurveChannel::ShadowOpacity, 0.5),
    ] {
        let points = session
            .animation_curve_channel(id, target, "Effect", channel, window)
            .unwrap();
        assert_eq!(
            points.iter().find(|point| point.time == 1.0).unwrap().value,
            expected
        );
    }
    session.scrub_animation_program(id, 1.0).unwrap();
    let value = session.preview.visual(target).unwrap().effect().unwrap();
    let shadow = value.as_drop_shadow_effect().unwrap();
    let directional = shadow.as_direction_drop_shadow_effect().unwrap();
    assert_eq!(directional.direction(), 90.0);
    assert_eq!(directional.shadow_depth(), 12.0);
    assert_eq!(shadow.opacity(), 0.5);
    session.record_effective_keyframe(target, "Effect", 1.0).unwrap();
    let edited = session.document().source();
    assert_eq!(edited.matches("<Setter.Value>").count(), 2);
    assert_eq!(edited.matches("<DropShadowDirectionEffect ").count(), 3);
    assert!(edited.contains("Direction=\"90\" ShadowDepth=\"12\""));
    assert!(edited.contains("<!-- Keep the authored note -->"));
    session.undo().unwrap();
    assert_eq!(session.document().source(), source);
}
