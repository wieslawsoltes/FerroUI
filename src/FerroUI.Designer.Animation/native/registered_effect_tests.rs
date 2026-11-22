use super::{sample_curve_track, sample_typed_keyframe};
use crate::{
    AnimationEffect, AnimationValue, CurveWindow, ImportOptions, Timeline, TrackCurveChannel,
};
use ferroui_base::animation::{Animation, Cue, FillMode, KeyFrame, SignedDuration};
use ferroui_base::media::Effect;
use ferroui_base::styling::Setter;
use ferroui_base::Visual;

#[test]
fn named_parameters_sample_native_values_across_effect_endpoints() {
    let source = r#"<Animation Duration="0:0:2">
       <KeyFrame Cue="25%"><Setter Property="Effect" Value="spotlight(strength=0.2, color=#80102030)"/></KeyFrame>
       <KeyFrame Cue="75%"><Setter Property="Effect" Value="spotlight(strength=0.8, color=#80102030)"/></KeyFrame>
    </Animation>"#;
    let options = ImportOptions {
        native_effect_endpoints: true,
        effect_literal: Some(AnimationEffect::parse_native),
        property_kinds: [("Effect".into(), crate::ValueKind::Effect)].into(),
        ..Default::default()
    };
    let timeline = Timeline::import_xaml(source, 1, &options).unwrap().timeline;
    let track = timeline.track(1, "Effect").unwrap();
    let channel: TrackCurveChannel = "effect_parameter/spotlight/strength/value".parse().unwrap();
    assert_eq!(channel.label(), "Strength");
    assert_eq!(
        serde_json::to_string(&channel).unwrap(),
        "\"effect_parameter/spotlight/strength/value\""
    );
    assert!(TrackCurveChannel::for_track(track, 2.0, timeline.fill).contains(&channel));

    let visual = Visual::new();
    visual.set_effect(Some(Effect::parse("blur(2)").unwrap()));
    let animation = Animation::new();
    animation.set_duration(SignedDuration::from_seconds(2.0));
    animation.set_fill_mode(FillMode::None);
    for key in &track.keyframes {
        let effect = Effect::parse(&key.value.to_xaml()).unwrap();
        animation.children().add(KeyFrame::with_cue(
            Cue::new(key.time / 2.0),
            [Setter::new(Visual::effect_property(), Some(effect)) as _],
        ));
    }
    let plot = sample_curve_track(
        &animation,
        &visual,
        "Effect",
        channel,
        CurveWindow {
            start: 0.0,
            end: 2.0,
        },
    )
    .unwrap();
    assert!(plot.first().unwrap().time > 0.0 && plot.last().unwrap().time < 2.0);
    assert_eq!(
        plot.iter().find(|point| point.time == 1.0).unwrap().value,
        0.5
    );
    let sample = sample_typed_keyframe(&animation, &visual, "Effect", 1.0).unwrap();
    let AnimationValue::Effect(registered @ AnimationEffect::Registered { .. }) = sample else {
        panic!("Expected an authored effect literal");
    };
    let parsed = registered.to_native().unwrap();
    assert_eq!(
        parsed
            .as_custom_effect()
            .unwrap()
            .parameter("strength")
            .unwrap()
            .as_float(),
        Some(0.5)
    );
    assert!(visual.effect().unwrap().as_blur_effect().is_some());
    assert!(AnimationEffect::parse_native("spotlight(strength=NaN)").is_err());
    assert!(AnimationEffect::parse_native("unregistered-preview-effect(strength=0.5)").is_err());
}
