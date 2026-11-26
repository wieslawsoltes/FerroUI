use crate::animation_integration_tests::attach_preview;
use crate::DesignerSession;
use ferroui_base::animation::{animators::CornerRadiusAnimator, Animation, InterpolatingAnimator};
use ferroui_base::CornerRadius;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, ContentControl};
use ferroui_designer_animation::{
    AnimationCodec, AnimationError, AnimationImport, AnimationValue, CurveWindow, ImportOptions, KeyframeEdit,
    NativeAnimationCodec, Timeline, ValueKind,
};

#[derive(Default)]
struct ReviewOpacity;

impl InterpolatingAnimator for ReviewOpacity {
    type Value = f64;

    fn interpolate(&self, progress: f64, from: &f64, to: &f64) -> f64 {
        from + (to - from) * progress * progress
    }
}

#[test]
fn registered_scalar_curve_matches_native_pose_without_changing_it() {
    std::thread::spawn(|| {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        Animation::register_custom_animator::<ReviewOpacity>();
        let source = r#"<Canvas xmlns="https://github.com/ferroui">
            <Canvas.Styles><Style Selector="Border#Review"><Style.Animations>
                <Animation Duration="0:0:2" FillMode="Both">
                    <KeyFrame Cue="0%"><Setter Property="Opacity" Value="0.2"/></KeyFrame>
                    <KeyFrame Cue="100%"><Setter Property="Opacity" Value="1"/></KeyFrame>
                </Animation>
            </Style.Animations></Style></Canvas.Styles>
            <Border Name="Review" Width="160" Height="90" Opacity="0.8"/>
        </Canvas>"#;
        let mut studio = DesignerSession::new(source, ContentControl::new()).unwrap();
        let _window = attach_preview(&studio);
        let program = &studio.animation_programs()[0];
        assert!(studio.requires_native_animation(program));
        let (id, target) = (program.id, program.primary_target());
        let visual = studio.preview.visual(target).unwrap();
        studio.scrub_animation_program(id, 0.5).unwrap();
        let opacity = visual.opacity();
        let curve = studio.animation_curve(id, target, "Opacity", CurveWindow { start: 0.0, end: 2.0 }).unwrap();
        let midpoint = curve.iter().find(|sample| sample.time == 1.0).unwrap().value;
        assert!((midpoint - 0.4).abs() < 1e-12);
        assert_eq!(studio.time(), 0.5);
        assert_eq!(visual.opacity(), opacity);
        assert_eq!(studio.document().source(), source);
        studio.scrub_animation_program(id, 1.0).unwrap();
        assert_eq!(visual.opacity(), midpoint);
    }).join().unwrap();
}

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

struct ProjectedCorners;

impl AnimationCodec for ProjectedCorners {
    fn id(&self) -> &str {
        "project.corners"
    }

    fn can_decode(&self, source: &str) -> bool {
        source.contains("<!--project-corners-->") && NativeAnimationCodec.can_decode(source)
    }

    fn decode(
        &self,
        source: &str,
        target: u64,
        options: &ImportOptions,
    ) -> ferroui_designer_animation::Result<AnimationImport> {
        let mut options = options.clone();
        options
            .property_kinds
            .insert("CornerRadius".into(), ValueKind::Discrete);
        options.base_values.remove("CornerRadius");
        options.literal_values.clear();
        NativeAnimationCodec.decode(source, target, &options)
    }

    fn encode(
        &self,
        original: &str,
        timeline: &Timeline,
        target: u64,
    ) -> ferroui_designer_animation::Result<String> {
        if timeline.tracks.iter().flat_map(|track| &track.keyframes).any(|key| {
            !matches!(&key.value, AnimationValue::Discrete(value) if value.parse::<u32>().is_ok())
        }) {
            return Err(AnimationError("Enter a nonnegative radius".into()));
        }
        NativeAnimationCodec.encode(original, timeline, target)
    }
}

#[test]
fn studio_keyframe_codec_edits_keep_native_interpolation_and_undo() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Canvas xmlns="https://github.com/ferroui">
        <Canvas.Styles><Style Selector="Border#Review"><Style.Animations>
            <Animation Duration="0:0:2" FillMode="Both"><!--project-corners-->
                <KeyFrame Cue="0%"><Setter Property="CornerRadius" Value="0"/></KeyFrame>
                <KeyFrame Cue="100%"><Setter Property="CornerRadius" Value="32"/></KeyFrame>
            </Animation>
        </Style.Animations></Style></Canvas.Styles>
        <!-- Preserve the card declaration -->
        <Border Name="Review" Width="160" Height="90" CornerRadius="0"/>
    </Canvas>"#;
    let mut studio = DesignerSession::new(source, ContentControl::new()).unwrap();
    let _window = attach_preview(&studio);
    studio.register_animation_codec(Box::new(ProjectedCorners));
    let program = &studio.animation_programs()[0];
    let (id, target) = (program.id, program.primary_target());
    assert_eq!(program.codec.as_deref(), Some("project.corners"));
    assert_eq!(
        program.timeline.as_ref().unwrap().tracks[0].keyframes[1]
            .value
            .kind(),
        ValueKind::Discrete
    );

    let edit = |value| KeyframeEdit::Upsert {
        previous: Some(2.0),
        time: 2.0,
        value,
        easing: "LinearEasing",
    };
    assert!(studio
        .edit_program_keyframe(id, target, "CornerRadius", edit("broken"))
        .is_err());
    assert_eq!(studio.document().source(), source);
    studio
        .edit_program_keyframe(id, target, "CornerRadius", edit("64"))
        .unwrap();
    let changed = studio.document().source();
    assert!(
        changed.contains("<!--project-corners-->")
            && changed.contains("<!-- Preserve the card declaration -->")
    );
    assert!(changed.contains("Value=\"64\""));
    let id = studio.animation_programs()[0].id;
    studio.scrub_animation_program(id, 1.0).unwrap();
    let border = studio
        .preview
        .visual(target)
        .unwrap()
        .cast::<Border>()
        .unwrap();
    assert_eq!(border.corner_radius().top_left, 32.0);
    studio.undo().unwrap();
    assert_eq!(studio.document().source(), source);
}
