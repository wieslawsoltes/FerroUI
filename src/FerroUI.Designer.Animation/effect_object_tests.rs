use crate::{AnimationEffect, AnimationValue, CurveChannel, CurveWindow, EffectForm, Timeline};

#[test]
fn effect_objects_keep_form_and_independent_shadow_components() {
    let source = r##"<Animation Duration="0:0:2" FillMode="Both">
        <KeyFrame Cue="0%"><Setter Property="Effect"><Setter.Value>
            <DropShadowDirectionEffect Direction="30" ShadowDepth="4" BlurRadius="2" Opacity="0.25" Color="#80234567"/>
        </Setter.Value></Setter></KeyFrame>
        <KeyFrame Cue="100%"><Setter Property="Effect">
            <DropShadowDirectionEffect Direction="150" ShadowDepth="20" BlurRadius="10" Opacity="0.75" Color="#80234567"/>
        </Setter></KeyFrame>
    </Animation>"##;
    let timeline = Timeline::from_xaml(source, 1).unwrap();
    let track = &timeline.tracks[0];
    let channels = CurveChannel::for_track(track, timeline.duration, timeline.fill);
    assert_eq!(
        channels,
        [
            CurveChannel::BlurRadius,
            CurveChannel::Direction,
            CurveChannel::ShadowDepth,
            CurveChannel::ShadowOpacity,
            CurveChannel::Red,
            CurveChannel::Green,
            CurveChannel::Blue,
            CurveChannel::Alpha
        ]
    );
    let compiled = timeline.compile().unwrap();
    let window = CurveWindow {
        start: 0.0,
        end: 2.0,
    };
    for (channel, expected) in [
        (CurveChannel::Direction, 90.0),
        (CurveChannel::ShadowDepth, 12.0),
        (CurveChannel::BlurRadius, 6.0),
        (CurveChannel::ShadowOpacity, 0.5),
        (CurveChannel::Alpha, 128.0 / 255.0),
    ] {
        let points = compiled.curve(1, "Effect", channel, window).unwrap();
        assert_eq!(
            points.iter().find(|point| point.time == 1.0).unwrap().value,
            expected
        );
    }
    let output = timeline.to_xaml(1).unwrap();
    assert_eq!(output.matches("<Setter.Value>").count(), 1);
    assert_eq!(output.matches("<DropShadowDirectionEffect ").count(), 2);
    assert_eq!(Timeline::from_xaml(&output, 1).unwrap(), timeline);
    let mid =
        AnimationEffect::parse(r#"<DropShadowDirectionEffect Direction="90" ShadowDepth="12"/>"#)
            .unwrap();
    let AnimationValue::Effect(sampled) =
        track.sample_in_authored_form(1.0, AnimationValue::Effect(mid))
    else {
        panic!("effect")
    };
    assert_eq!(sampled.form(), EffectForm::PropertyElement);
}

#[test]
fn effect_objects_reject_unmapped_values_and_read_legacy_effects() {
    for source in [
        r#"<DropShadowEffect Opacity="{Binding Intensity}"/>"#,
        r#"<DropShadowEffect Opacity="NaN"/>"#,
        r#"<DropShadowDirectionEffect OffsetX="12"/>"#,
        r#"<BlurEffect xmlns="using:App.Effects" Radius="8"/>"#,
        r#"<Setter.Value><BlurEffect/><BlurEffect/></Setter.Value>"#,
        r#"<DropShadowEffect><DropShadowEffect.Color><Binding/></DropShadowEffect.Color></DropShadowEffect>"#,
    ] {
        assert!(AnimationEffect::parse(source).is_err(), "{source}");
    }
    let legacy = serde_json::json!({"kind":"drop_shadow","offset_x":3.0,"offset_y":5.0,"blur_radius":10.0,"argb":[128,0,0,0]});
    let effect: AnimationEffect = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(effect.to_xaml(), "drop-shadow(3 5 10 #80000000)");
    assert_eq!(serde_json::to_value(effect).unwrap(), legacy);
}
