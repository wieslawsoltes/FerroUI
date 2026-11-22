use crate::{AnimationValue, CurveChannel, CurveWindow, ImportOptions, Timeline, ValueKind};

#[test]
fn literal_effect_keys_round_trip_with_their_component_channels() {
    let source = r##"<Animation Duration="0:0:2" FillMode="Both">
      <KeyFrame Cue="0%"><Setter Property="Effect" Value="drop-shadow(0 2 4 #FF000000)"/></KeyFrame>
      <KeyFrame Cue="100%"><Setter Property="Effect" Value="drop-shadow(12 -4 16 #80FF8040)"/></KeyFrame>
    </Animation>"##;
    let import = |source: &str| {
        Timeline::import_xaml(source, 1, &ImportOptions::default())
            .unwrap()
            .timeline
    };
    let timeline = import(source);
    let compiled = timeline.compile().unwrap();
    let window = CurveWindow {
        start: 0.0,
        end: 2.0,
    };
    let channels = CurveChannel::for_track(&timeline.tracks[0], timeline.duration, timeline.fill);
    assert_eq!(
        channels,
        [
            CurveChannel::BlurRadius,
            CurveChannel::OffsetX,
            CurveChannel::OffsetY,
            CurveChannel::Red,
            CurveChannel::Green,
            CurveChannel::Blue,
            CurveChannel::Alpha
        ]
    );
    for (channel, expected) in [
        (CurveChannel::BlurRadius, 10.0),
        (CurveChannel::OffsetX, 6.0),
        (CurveChannel::OffsetY, -1.0),
    ] {
        let curve = compiled.curve(1, "Effect", channel, window).unwrap();
        assert_eq!(
            curve.iter().find(|point| point.time == 1.0).unwrap().value,
            expected
        );
    }
    assert_eq!(import(&timeline.to_xaml(1).unwrap()), timeline);
    let value = &timeline.tracks[0].keyframes[1].value;
    assert_eq!(
        serde_json::from_value::<AnimationValue>(serde_json::to_value(value).unwrap()).unwrap(),
        *value
    );
    for invalid in [
        "blur(NaN)",
        "blur(4) blur(8)",
        "drop-shadow(2 3 -1)",
        "drop-shadow(2 3 4 #000000000)",
    ] {
        assert!(
            AnimationValue::parse(invalid, ValueKind::Effect).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn native_effect_keys_keep_implicit_endpoints_in_the_authored_source() {
    let source = r#"<Animation Duration="0:0:2"><KeyFrame Cue="50%"><Setter Property="Effect" Value="blur(4)"/></KeyFrame></Animation>"#;
    assert!(Timeline::import_xaml(source, 1, &ImportOptions::default()).is_err());
    let mut options = ImportOptions {
        native_effect_endpoints: true,
        ..ImportOptions::default()
    };
    options
        .property_kinds
        .insert("Effect".into(), ValueKind::Effect);
    let timeline = Timeline::import_xaml(source, 1, &options).unwrap().timeline;
    let serialized = timeline.to_xaml(1).unwrap();
    assert_eq!(serialized.matches("<KeyFrame ").count(), 1);
    assert_eq!(
        Timeline::import_xaml(&serialized, 1, &options)
            .unwrap()
            .timeline,
        timeline
    );
}
