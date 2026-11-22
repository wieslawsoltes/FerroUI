use super::{sample_curve_track, sample_typed_keyframe, CurveChannel, CurveWindow};
use crate::native::NativePlayback;
use crate::{AnimationEffect, AnimationValue};
use ferroui_base::animation::{Animation, Cue, FillMode, KeyFrame, SignedDuration};
use ferroui_base::media::Effect;
use ferroui_base::styling::Setter;
use ferroui_base::Visual;

#[test]
fn effect_curves_sample_the_same_animator_as_native_playback() {
    let visual = Visual::new();
    let mut playback = NativePlayback::default();
    let window = CurveWindow {
        start: 0.0,
        end: 2.0,
    };
    for (from, to, expected, sample_channel, midpoint) in [
        (
            "blur(0)",
            "blur(12)",
            "blur(6)",
            CurveChannel::BlurRadius,
            6.0,
        ),
        (
            "drop-shadow(0 2 4 Black)",
            "drop-shadow(12 -4 16 rgba(255, 128, 64, 50%))",
            "drop-shadow(6 -1 10 #C0BC5C2C)",
            CurveChannel::OffsetX,
            6.0,
        ),
    ] {
        let animation = Animation::new();
        animation.set_duration(SignedDuration::from_seconds(2.0));
        animation.set_fill_mode(FillMode::Both);
        for (cue, text) in [(0.0, from), (1.0, to)] {
            animation.children().add(KeyFrame::with_cue(
                Cue::new(cue),
                [Setter::new(
                    Visual::effect_property(),
                    Some(Effect::parse(text).unwrap()),
                ) as _],
            ));
        }
        let curve =
            sample_curve_track(&animation, &visual, "Effect", sample_channel, window).unwrap();
        assert_eq!(
            curve.iter().find(|point| point.time == 1.0).unwrap().value,
            midpoint
        );
        let sampled = sample_typed_keyframe(&animation, &visual, "Effect", 1.0).unwrap();
        assert_eq!(
            sampled,
            AnimationValue::Effect(AnimationEffect::parse_native(expected).unwrap())
        );
        assert!(
            visual.effect().is_none(),
            "Reading the curve leaves the native control alone"
        );
        playback
            .seek(vec![(animation, visual.clone().upcast())], 1.0)
            .unwrap();
        let actual = visual.effect().unwrap();
        assert_eq!(
            AnimationValue::Effect(AnimationEffect::from_native(&*actual).unwrap()),
            sampled
        );
        playback.stop();
    }
}
