use super::*;
fn numeric() -> Timeline {
    let mut timeline = Timeline::new(2.0).unwrap();
    timeline
        .record(1, "Opacity", 0.0, AnimationValue::Number(0.0))
        .unwrap();
    timeline
        .record(1, "Opacity", 2.0, AnimationValue::Number(1.0))
        .unwrap();
    timeline
}
fn number(timeline: &Timeline, time: f64) -> f64 {
    match timeline.sample(time).unwrap()[0].value {
        AnimationValue::Number(value) => value,
        _ => panic!("number"),
    }
}
#[test]
fn samples_independently_of_frame_rate_and_repeat_direction() {
    let mut timeline = numeric();
    assert_eq!(number(&timeline, 0.5), 0.25);
    timeline.repeat = Repeat::Count(2);
    timeline.playback_direction = PlaybackDirection::Alternate;
    assert_eq!(number(&timeline, 2.0), 1.0);
    assert_eq!(number(&timeline, 3.0), 0.5);
    assert_eq!(number(&timeline, 4.0), 0.0);
    timeline.playback_direction = PlaybackDirection::AlternateReverse;
    assert_eq!(number(&timeline, 0.0), 1.0);
    assert_eq!(number(&timeline, 4.0), 1.0);
}
#[test]
fn delay_speed_and_fill_are_respected() {
    let mut timeline = numeric();
    timeline.delay = 1.0;
    timeline.speed_ratio = 2.0;
    timeline.fill = FillMode::None;
    assert!(timeline.sample(0.1).unwrap().is_empty());
    assert_eq!(number(&timeline, 1.0), 0.5);
    assert!(timeline.sample(1.5).unwrap().is_empty());
    timeline.fill = FillMode::Both;
    assert_eq!(number(&timeline, 0.1), 0.0);
    assert_eq!(number(&timeline, 3.0), 1.0);
}
#[test]
fn bezier_solves_x_and_rejects_invalid_controls() {
    let curve = Easing::CubicBezier {
        x1: 0.8,
        y1: 0.0,
        x2: 0.9,
        y2: 1.0,
    };
    assert!(curve.sample(0.5).unwrap() < 0.2);
    assert_eq!(curve.sample(0.0).unwrap(), 0.0);
    assert!(Easing::CubicBezier {
        x1: -1.0,
        y1: 0.0,
        x2: 1.0,
        y2: 1.0
    }
    .validate()
    .is_err());
    assert!((Easing::Named("BackEaseIn".into()).sample(0.5).unwrap() + 0.375).abs() < 1e-12);
    assert!((Easing::Named("BounceEaseOut".into()).sample(0.5).unwrap() - 0.71875).abs() < 1e-12);
}
#[test]
fn color_interpolation_matches_linear_light_runtime() {
    let black = AnimationValue::parse("#FF000000", ValueKind::Color).unwrap();
    let white = AnimationValue::parse("#FFFFFFFF", ValueKind::Color).unwrap();
    assert_eq!(
        black.interpolate(&white, 0.5).unwrap().to_xaml(),
        "#FFBCBCBC"
    );
    assert_eq!(
        AnimationValue::parse("1,2", ValueKind::Thickness)
            .unwrap()
            .to_xaml(),
        "1,2,1,2"
    );
    assert!(black
        .interpolate(&AnimationValue::Number(1.0), 0.5)
        .is_err());
}
#[test]
fn edits_validate_and_preserve_existing_key_easing() {
    let mut timeline = numeric();
    let track = timeline.track_mut(1, "Opacity").unwrap();
    let easing = Easing::CubicBezier {
        x1: 0.2,
        y1: 0.0,
        x2: 0.7,
        y2: 1.0,
    };
    track.set_easing(2.0, easing.clone()).unwrap();
    track.record(2.0, AnimationValue::Number(0.8)).unwrap();
    assert_eq!(track.keyframes[1].easing, easing);
    assert!(track.move_keyframe(2.0, 0.0).is_err());
    track.move_keyframe(2.0, 1.0).unwrap();
    assert_eq!(track.keyframes[1].time, 1.0);
    assert!(timeline
        .record(1, "Opacity", f64::NAN, AnimationValue::Number(0.2))
        .is_err());
    assert!(timeline
        .record(1, "Opacity", 1.0, AnimationValue::Discrete("x".into()))
        .is_err());
}
#[test]
fn player_pauses_seeks_and_clamps_to_finite_end() {
    let timeline = numeric();
    let mut player = Player::new();
    player.play();
    player.advance(0.5, &timeline).unwrap();
    player.pause();
    player.advance(1.0, &timeline).unwrap();
    assert_eq!(player.time(), 0.5);
    player.seek(1.5).unwrap();
    player.play();
    player.advance(10.0, &timeline).unwrap();
    assert_eq!(player.time(), 2.0);
    assert!(!player.is_playing());
    assert!(player.advance(f64::NAN, &timeline).is_err());
    assert_eq!(player.time(), 2.0);
    player.stop();
    assert_eq!(player.time(), 0.0);
}
#[test]
fn native_xaml_roundtrip_preserves_curves_properties_and_timing() {
    let mut timeline = numeric();
    timeline.delay = 0.25;
    timeline.repeat = Repeat::Count(3);
    timeline.playback_direction = PlaybackDirection::Alternate;
    timeline
        .track_mut(1, "Opacity")
        .unwrap()
        .set_easing(
            2.0,
            Easing::CubicBezier {
                x1: 0.25,
                y1: 0.1,
                x2: 0.25,
                y2: 1.0,
            },
        )
        .unwrap();
    timeline
        .record(
            1,
            "Margin",
            0.0,
            AnimationValue::parse("1,2", ValueKind::Thickness).unwrap(),
        )
        .unwrap();
    timeline
        .record(
            1,
            "Margin",
            2.0,
            AnimationValue::parse("10", ValueKind::Thickness).unwrap(),
        )
        .unwrap();
    let source = timeline.to_xaml(1).unwrap();
    assert!(source.contains("KeySpline="));
    let imported = Timeline::from_xaml(&source, 1).unwrap();
    for time in [0.0, 0.3, 0.8, 1.5, 2.5, 6.25] {
        let mut a = timeline.sample(time).unwrap();
        let mut b = imported.sample(time).unwrap();
        a.sort_by(|a, b| a.property.cmp(&b.property));
        b.sort_by(|a, b| a.property.cmp(&b.property));
        assert_eq!(a, b);
    }
}
#[test]
fn implicit_endpoints_require_underlying_value_and_unknown_markup_is_not_discarded() {
    let source = r#"<Animation Duration="0:0:2"><KeyFrame Cue="50%"><Setter Property="Opacity" Value="1" /></KeyFrame></Animation>"#;
    assert!(Timeline::from_xaml(source, 1).is_err());
    let mut options = ImportOptions::default();
    options
        .base_values
        .insert("Opacity".into(), AnimationValue::Number(0.0));
    let timeline = Timeline::import_xaml(source, 1, &options).unwrap().timeline;
    assert_eq!(number(&timeline, 0.0), 0.0);
    assert_eq!(number(&timeline, 0.5), 0.5);
    assert_eq!(number(&timeline, 1.5), 0.5);
    assert!(Timeline::from_xaml(r#"<Animation Duration="1" CustomBehavior="x"/>"#, 1).is_err());
}
#[test]
fn compiled_sampler_equals_checked_sampler_and_discrete_values_switch_at_key() {
    let timeline = numeric();
    let compiled = timeline.compile().unwrap();
    for i in 0..201 {
        let time = i as f64 / 100.0;
        assert_eq!(
            compiled.sample(time).unwrap(),
            timeline.sample(time).unwrap()
        );
        assert_eq!(compiled.scrub(time).unwrap(), timeline.scrub(time).unwrap());
    }
    let start = AnimationValue::Discrete("Hidden".into());
    let end = AnimationValue::Discrete("Visible".into());
    assert_eq!(start.interpolate(&end, 0.99).unwrap(), start);
    assert_eq!(start.interpolate(&end, 1.0).unwrap(), end);
}
#[test]
fn native_time_export_quantizes_ticks_and_rejects_colliding_keys() {
    let mut timeline = Timeline::new(90000.0).unwrap();
    timeline
        .record(1, "Opacity", 0.0, AnimationValue::Number(0.0))
        .unwrap();
    timeline
        .record(1, "Opacity", 0.1 + 0.2, AnimationValue::Number(0.5))
        .unwrap();
    timeline
        .record(1, "Opacity", 90000.0, AnimationValue::Number(1.0))
        .unwrap();
    let source = timeline.to_xaml(1).unwrap();
    assert!(source.contains("Duration=\"1.1:0:0\""));
    assert!(source.contains("0:0:0.3000000"));
    Timeline::from_xaml(&source, 1).unwrap();
    timeline
        .record(1, "Opacity", 0.3000000001, AnimationValue::Number(0.6))
        .unwrap();
    assert!(timeline.to_xaml(1).is_err());
}
#[test]
fn finite_extreme_values_do_not_generate_nonfinite_preview_samples() {
    assert_eq!(
        AnimationValue::Number(-1e308)
            .interpolate(&AnimationValue::Number(1e308), 0.5)
            .unwrap(),
        AnimationValue::Number(0.0)
    );
    assert!(AnimationValue::Number(-1e308)
        .interpolate(&AnimationValue::Number(1e308), 10.0)
        .is_err());
}
#[test]
fn literal_braces_and_xml_characters_roundtrip_without_becoming_bindings() {
    let mut timeline = Timeline::new(1.0).unwrap();
    let value = AnimationValue::Discrete("{literal & <text>}".into());
    timeline.record(1, "Tag", 0.0, value.clone()).unwrap();
    timeline.record(1, "Tag", 1.0, value.clone()).unwrap();
    let source = timeline.to_xaml(1).unwrap();
    let imported = Timeline::from_xaml(&source, 1).unwrap();
    assert_eq!(imported.tracks[0].keyframes[0].value, value);
    assert!(AnimationValue::Discrete("bad\u{0001}".into())
        .validate()
        .is_err());
}

#[test]
fn native_timespan_import_uses_days_and_strict_fraction_grammar() {
    let source = r#"<Animation Duration="1"><KeyFrame Cue="0%"><Setter Property="Opacity" Value="0"/></KeyFrame><KeyFrame Cue="100%"><Setter Property="Opacity" Value="1"/></KeyFrame></Animation>"#;
    let timeline = Timeline::from_xaml(source, 1).unwrap();
    assert_eq!(timeline.duration, 86400.0);
    for value in ["0.5", "0:0:1.12345678", "0:0:1e2", "+1", "0:0:1.", "24:00"] {
        assert!(
            Timeline::from_xaml(
                &source.replace("Duration=\"1\"", &format!("Duration=\"{value}\"")),
                1
            )
            .is_err(),
            "{value}"
        );
    }
    let fraction = Timeline::from_xaml(
        &source.replace("Duration=\"1\"", "Duration=\"0:0:.075\""),
        1,
    )
    .unwrap();
    assert_eq!(fraction.duration, 0.075);
}

#[test]
fn implicit_native_endpoints_respect_fill_and_inherit_trailing_spline() {
    let source = r#"<Animation Duration="0:0:2" FillMode="Both"><KeyFrame Cue="25%"><Setter Property="Opacity" Value="0.25"/></KeyFrame><KeyFrame Cue="75%" KeySpline="0.8,0,0.9,1"><Setter Property="Opacity" Value="0.75"/></KeyFrame></Animation>"#;
    let filled = Timeline::from_xaml(source, 1).unwrap();
    assert_eq!(number(&filled, 0.0), 0.25);
    assert_eq!(number(&filled, 2.0), 0.75);
    let mut options = ImportOptions::default();
    options
        .base_values
        .insert("Opacity".into(), AnimationValue::Number(0.0));
    let neutral = Timeline::import_xaml(
        &source.replace("FillMode=\"Both\"", "FillMode=\"None\""),
        1,
        &options,
    )
    .unwrap()
    .timeline;
    let spline = Easing::CubicBezier {
        x1: 0.8,
        y1: 0.0,
        x2: 0.9,
        y2: 1.0,
    };
    let expected = 0.75 * (1.0 - spline.sample(0.5).unwrap());
    assert!((number(&neutral, 1.75) - expected).abs() < 1e-12);
    let exported = neutral.to_xaml(1).unwrap();
    let roundtrip = Timeline::import_xaml(&exported, 1, &options)
        .unwrap()
        .timeline;
    assert!((number(&roundtrip, 1.75) - expected).abs() < 1e-12);
}

#[test]
fn native_initial_delay_uses_actual_edge_without_global_easing() {
    let mut timeline = numeric();
    timeline.delay = 1.0;
    timeline.fill = FillMode::Backward;
    timeline.easing = Easing::Named("ExponentialEaseInOut".into());
    assert_eq!(number(&timeline, 0.5), 0.0);
    assert_eq!(number(&timeline, 1.0), 0.0);
    timeline.playback_direction = PlaybackDirection::Reverse;
    assert_eq!(number(&timeline, 0.5), 1.0);
}

#[test]
fn recording_canonicalizes_signed_zero_and_checks_underlying_type() {
    let mut track = Track::new(1, "Opacity");
    track.record(-0.0, AnimationValue::Number(0.0)).unwrap();
    track.record(0.0, AnimationValue::Number(0.5)).unwrap();
    assert_eq!(track.keyframes.len(), 1);
    track.validate(1.0).unwrap();
    assert_eq!(
        track.remove_keyframe(-0.0).unwrap().value,
        AnimationValue::Number(0.5)
    );
    track.base_value = Some(AnimationValue::Number(1.0));
    assert!(track
        .record(0.0, AnimationValue::Discrete("text".into()))
        .is_err());
    assert!(track.keyframes.is_empty());
}

#[test]
fn native_repeat_and_effective_clock_ranges_are_validated() {
    let mut timeline = numeric();
    timeline.repeat = Repeat::Count(i64::MAX as u64 + 1);
    assert!(timeline.to_xaml(1).is_err());
    timeline.repeat = Repeat::Count(1);
    timeline.speed_ratio = f64::MIN_POSITIVE / 100.0;
    assert!(timeline.validate().is_err());
    timeline.repeat = Repeat::Infinite;
    assert!(timeline.validate().is_err());
}

#[test]
fn xml_noncharacters_and_tab_separated_splines_are_handled() {
    assert!(AnimationValue::Discrete("invalid\u{FFFF}".into())
        .validate()
        .is_err());
    let curve = Easing::parse("0\t0\t1\t1").unwrap();
    assert!((curve.sample(0.5).unwrap() - 0.5).abs() < 1e-12);
    let mut timeline = numeric();
    timeline.tracks[0].property = "bad\u{FFFE}".into();
    assert!(timeline.to_xaml(1).is_err());
}

#[test]
fn reverse_delay_fill_preserves_implicit_endpoints_on_export() {
    let source = r#"<Animation Duration="0:0:2" Delay="0:0:1" FillMode="Backward" PlaybackDirection="Reverse"><KeyFrame Cue="25%"><Setter Property="Opacity" Value="0.25"/></KeyFrame><KeyFrame Cue="75%"><Setter Property="Opacity" Value="0.75"/></KeyFrame></Animation>"#;
    let mut options = ImportOptions::default();
    options
        .base_values
        .insert("Opacity".into(), AnimationValue::Number(0.0));
    let original = Timeline::import_xaml(source, 1, &options).unwrap().timeline;
    assert_eq!(number(&original, 0.5), 0.75);
    let exported = original.to_xaml(1).unwrap();
    let reparsed = Timeline::import_xaml(&exported, 1, &options)
        .unwrap()
        .timeline;
    assert_eq!(number(&reparsed, 0.5), 0.75);
    for time in [0.0, 0.5, 1.0, 1.2, 1.5, 2.0, 2.5] {
        assert_eq!(
            original.sample(time).unwrap(),
            reparsed.sample(time).unwrap()
        );
    }
}
