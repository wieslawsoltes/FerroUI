use super::{sample_curve_track, sample_typed_keyframe};
use crate::{
    AnimationEffect, AnimationValue, CurveWindow, ImportOptions, Timeline, TrackCurveChannel,
};
use ferroui_base::animation::{Animation, Cue, FillMode, KeyFrame, SignedDuration};
use ferroui_base::media::Effect;
use ferroui_base::styling::Setter;
use ferroui_base::Visual;

#[test]
fn value_only_registered_effect_patch_keeps_xml_spelling_and_shared_cue() {
    use crate::{animation_fragment, registered_key_value_patch, AnimationCodecRegistry, AnimationPrograms};
    use ferroui_designer_core::{Document, TextPatch};

    let source = r#"<f:Window xmlns:f="https://github.com/ferroui">
    <f:Window.Styles><f:Style Selector="Border"><f:Style.Animations>
        <f:Animation Duration="0:0:2" FillMode="Both">
            <!-- Keep this cue and both setters 🧪 -->
            <f:KeyFrame Cue='50%'><f:Setter Property="Opacity" Value="0.8"/>
                <f:Setter Property="Effect" Value='{}Spotlight ( Color = &#x23;8021356C ; Strength = 0.42 )'/>
            </f:KeyFrame>
        </f:Animation>
    </f:Style.Animations></f:Style></f:Window.Styles>
</f:Window>"#.replace('\n', "\r\n");
    let document = Document::new(source.as_str()).unwrap();
    let mut program = AnimationPrograms::scan(&document).pop().unwrap();
    let fragment = animation_fragment(&document, program.id).unwrap();
    let options = ImportOptions {
        native_effect_endpoints: true,
        effect_literal: Some(AnimationEffect::parse_native),
        property_kinds: [("Effect".into(), crate::ValueKind::Effect)].into(),
        ..Default::default()
    };
    AnimationCodecRegistry::default().decode(&mut program, &fragment, &options);
    let target = program.primary_target();
    let mut draft = program.timeline.as_ref().unwrap().clone();
    let key = &mut draft.track_mut(target, "Effect").unwrap().keyframes[0];
    let channel: TrackCurveChannel = "effect_parameter/spotlight/strength/value".parse().unwrap();
    key.value = channel.replace_key_component(&key.value, 0.65).unwrap();
    let patch = registered_key_value_patch(&document, &program, &draft).unwrap().unwrap();
    assert_eq!(&source[patch.span.range()], "42");
    assert_eq!(TextPatch::apply_all(&source, vec![patch]).unwrap(), source.replace("0.42", "0.65"));
    let retired = Document::new(source.replace("0.42", "0.5")).unwrap();
    assert!(registered_key_value_patch(&retired, &program, &draft).is_err());
    draft.track_mut(target, "Effect").unwrap().keyframes[0].time = 1.25;
    assert!(registered_key_value_patch(&document, &program, &draft).unwrap().is_none());
}

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
