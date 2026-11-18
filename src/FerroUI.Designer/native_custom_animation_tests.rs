use crate::animation_integration_tests::attach_preview;
use crate::DesignerSession;
use ferroui_base::animation::{animators::CornerRadiusAnimator, Animation, InterpolatingAnimator};
use ferroui_base::CornerRadius;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, ContentControl};

#[derive(Default)]
struct ReviewCorners;

impl InterpolatingAnimator for ReviewCorners {
    type Value = CornerRadius;

    fn interpolate(&self, progress: f64, from: &CornerRadius, to: &CornerRadius) -> CornerRadius {
        CornerRadiusAnimator::interpolate_core(progress * progress, from, to)
    }
}

#[test]
fn timeline_uses_registered_animators_for_typed_tracks_and_native_visibility() {
    // The animator registry belongs to the UI thread, so isolate the registration
    // from other tests that use the built-in animator for the same property type.
    std::thread::spawn(|| {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        Animation::register_custom_animator::<ReviewCorners>();
        let mut studio = DesignerSession::new(
            r#"<Canvas xmlns="https://github.com/ferroui">
                <Canvas.Styles><Style Selector="Border#Review"><Style.Animations>
                    <Animation Duration="0:0:2" FillMode="Both" PlaybackBehavior="OnlyIfVisible">
                        <KeyFrame Cue="0%"><Setter Property="CornerRadius" Value="0"/></KeyFrame>
                        <KeyFrame Cue="100%"><Setter Property="CornerRadius" Value="32"/></KeyFrame>
                    </Animation>
                </Style.Animations></Style></Canvas.Styles>
                <Border Name="Review" Width="160" Height="90" CornerRadius="0"/>
            </Canvas>"#,
            ContentControl::new(),
        )
        .unwrap();
        let _window = attach_preview(&studio);
        let program = studio.animation_programs()[0].id;
        assert!(studio.animation_programs()[0].timeline.is_some());
        let node = studio.document().ast().find_name("Review").unwrap();
        let border = studio
            .preview
            .visual(node)
            .unwrap()
            .cast::<Border>()
            .unwrap();
        studio.scrub_animation_program(program, 1.0).unwrap();
        assert_eq!(border.corner_radius().top_left, 8.0);
        studio.stop();
        assert_eq!(border.corner_radius().top_left, 0.0);

        studio
            .start_animation_playback(program, Some(0.0), false)
            .unwrap();
        studio.advance(0.5).unwrap();
        assert_eq!(border.corner_radius().top_left, 2.0);
        border.set_is_visible(false);
        studio.advance(1.0).unwrap();
        assert_eq!(border.corner_radius().top_left, 2.0);
        border.set_is_visible(true);
        studio.advance(0.5).unwrap();
        assert_eq!(border.corner_radius().top_left, 8.0);
    })
    .join()
    .unwrap();
}
