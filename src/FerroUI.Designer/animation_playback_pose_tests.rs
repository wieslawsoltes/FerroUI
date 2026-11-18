use crate::animation_integration_tests::attach_preview;
use crate::{DesignerSession, TimelinePreviewPose};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, ContentControl};

const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
  <Canvas.Styles>
    <Style Selector="Button">
      <Style.Animations>
        <Animation Duration="0:0:2" IterationCount="3" PlaybackDirection="Alternate" PlaybackBehavior="OnlyIfVisible" FillMode="Both">
          <KeyFrame Cue="0%"><Setter Property="Opacity" Value="0"/></KeyFrame>
          <KeyFrame Cue="100%"><Setter Property="Opacity" Value="1"/></KeyFrame>
        </Animation>
      </Style.Animations>
    </Style>
  </Canvas.Styles>
  <Button Name="Lead" Content="Old"/>
  <Button Name="Hidden" IsVisible="False" Canvas.Top="50"/>
</Canvas>"#;

fn opacity(session: &DesignerSession, name: &str) -> f64 {
    let target = session.document().ast().find_name(name).unwrap();
    session.preview.visual(target).unwrap().opacity()
}

#[test]
fn timeline_rebuild_preserves_elapsed_iteration_and_individual_visibility_clocks() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let mut session = DesignerSession::new(SOURCE, ContentControl::new()).unwrap();
    let _window = attach_preview(&session);
    session.play();
    session.advance(3.0).unwrap();
    session.pause();
    assert_eq!(opacity(&session, "Lead"), 0.5);
    assert_eq!(opacity(&session, "Hidden"), 0.0);
    let original = session.timeline_playback_pose().unwrap();
    assert_eq!(original.time, 1.0);
    assert_eq!(original.playback.as_ref().unwrap().elapsed, 3.0);

    let replacement = SOURCE
        .replace("Content=\"Old\"", "Content=\"Edited\"")
        .replace("IsVisible=\"False\"", "IsVisible=\"True\"")
        .replace("<Canvas.Styles>", "<Canvas.Resources><SolidColorBrush x:Key=\"Added\">Red</SolidColorBrush></Canvas.Resources><Canvas.Styles>");
    let mut rebuilt = DesignerSession::new(&replacement, ContentControl::new()).unwrap();
    let _rebuilt_window = attach_preview(&rebuilt);
    assert_ne!(rebuilt.active_animation_program(), Some(original.program));
    let encoded = serde_json::to_string(&original).unwrap();
    let incoming: TimelinePreviewPose = serde_json::from_str(&encoded).unwrap();
    let held = rebuilt
        .restore_timeline_preview_pose(&incoming)
        .unwrap()
        .unwrap();
    assert_eq!(held.time, 1.0);
    assert_eq!(held.playback.unwrap().elapsed, 3.0);
    assert!(!rebuilt.is_playing());
    assert_eq!(opacity(&rebuilt, "Lead"), 0.5);
    assert_eq!(opacity(&rebuilt, "Hidden"), 0.0);
    rebuilt.play();
    rebuilt.advance(0.5).unwrap();
    assert_eq!(opacity(&rebuilt, "Lead"), 0.25);
    assert_eq!(opacity(&rebuilt, "Hidden"), 0.25);

    assert!(!session.update_source("<Canvas>").valid);
    assert!(session.update_source(&replacement).valid);
    assert_eq!(session.time(), 3.0);
    assert!(!session.is_playing());
    assert_eq!(opacity(&session, "Hidden"), 0.0);
    session.play();
    session.advance(0.5).unwrap();
    assert_eq!(opacity(&session, "Lead"), 0.25);
    assert_eq!(opacity(&session, "Hidden"), 0.25);
}

#[test]
fn timeline_native_resource_keeps_explicit_targets_and_samples_after_rebuild() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
      <Canvas.Resources><Animation x:Key="Shadow" x:SetterTargetType="Border" Duration="0:0:2" FillMode="Forward">
        <KeyFrame Cue="0%"><Setter Property="BoxShadow" Value="0 0 0 0 Red"/></KeyFrame>
        <KeyFrame Cue="100%"><Setter Property="BoxShadow" Value="20 30 10 4 Blue"/></KeyFrame>
      </Animation></Canvas.Resources>
      <Border Name="Card" Width="100" Height="60"/><Border Name="Other"/>
    </Canvas>"#;
    let shadow = |session: &DesignerSession, name: &str| {
        let target = session.document().ast().find_name(name).unwrap();
        session
            .preview
            .visual(target)
            .unwrap()
            .cast::<Border>()
            .unwrap()
            .box_shadow()
    };
    let mut session = DesignerSession::new(source, ContentControl::new()).unwrap();
    let _window = attach_preview(&session);
    let id = session.active_animation_program().unwrap();
    let target = session.document().ast().find_name("Card").unwrap();
    assert!(session.animation_programs()[0].timeline.is_none());
    session
        .set_animation_program_preview_targets(id, Some(vec![target]))
        .unwrap();
    session.play();
    session.advance(0.75).unwrap();
    session.pause();
    assert!((shadow(&session, "Card").get(0).offset_x - 7.5).abs() < 1e-6);
    assert_eq!(shadow(&session, "Other").count(), 0);
    let pose = session.timeline_playback_pose().unwrap();
    let targets = session.timeline_target_overrides();
    let replacement = source.replace(
        "<Canvas.Resources>",
        "<TextBlock Text=\"Added\"/><Canvas.Resources>",
    );
    let mut rebuilt = DesignerSession::new(&replacement, ContentControl::new()).unwrap();
    let _window = attach_preview(&rebuilt);
    rebuilt.restore_timeline_targets(&targets);
    let held = rebuilt
        .restore_timeline_preview_pose(&pose)
        .unwrap()
        .unwrap();
    assert_eq!(held.time, 0.75);
    assert_eq!(held.playback.unwrap().elapsed, 0.75);
    assert!((shadow(&rebuilt, "Card").get(0).offset_x - 7.5).abs() < 1e-6);
    rebuilt.play();
    rebuilt.advance(0.25).unwrap();
    assert!((shadow(&rebuilt, "Card").get(0).offset_x - 10.0).abs() < 1e-6);
    rebuilt.stop();
    assert_eq!(shadow(&rebuilt, "Card").count(), 0);
    assert_eq!(rebuilt.document().source(), replacement);
}
