use super::*;

fn location(target: u64, property: &str, time: f64) -> KeyframeLocation {
    KeyframeLocation { target, property: property.into(), time }
}

fn radius(value: f64) -> AnimationValue {
    AnimationValue::CornerRadius { top_left: value, top_right: value, bottom_right: value, bottom_left: value }
}

#[test]
fn pasted_tracks_keep_types_relative_time_and_reversed_segment_easing() {
    let mut source = Timeline::new(2.0).unwrap();
    source.record(1, "Opacity", 0.25, AnimationValue::Number(0.25)).unwrap();
    source.record(1, "Opacity", 0.75, AnimationValue::Number(0.75)).unwrap();
    source.record(2, "CornerRadius", 0.5, radius(8.0)).unwrap();
    source.record(2, "CornerRadius", 1.0, radius(16.0)).unwrap();
    let ease = Easing::CubicBezier { x1: 0.125, y1: -0.25, x2: 0.75, y2: 1.5 };
    source.track_mut(1, "Opacity").unwrap().set_easing(0.75, ease.clone()).unwrap();
    let clipboard = source.copy_keyframes(&[
        location(2, "CornerRadius", 1.0), location(1, "Opacity", 0.75),
        location(2, "CornerRadius", 0.5), location(1, "Opacity", 0.25),
    ]).unwrap();
    let serialized = clipboard.to_text().unwrap();
    assert!(!serialized.contains("target"));
    assert_eq!(KeyframeClipboard::from_text(&serialized).unwrap(), clipboard);
    assert!(KeyframeClipboard::from_text(&serialized.replace("keyframes.v1", "keyframes.v2")).is_err());

    let mut target = Timeline::new(3.0).unwrap();
    target.record(10, "Width", 0.0, AnimationValue::Number(100.0)).unwrap();
    target.record(20, "CornerRadius", 0.0, radius(1.0)).unwrap();
    let destinations = vec![KeyframeDestination { target: 10, property: "Width".into() },
        KeyframeDestination { target: 20, property: "CornerRadius".into() }];
    let mut paste = KeyframePaste { clipboard, destinations, time: 1.0, reverse: false };
    let (forward, keys) = target.with_pasted_keyframes(&paste).unwrap();
    assert_eq!(keys, [location(10, "Width", 1.0), location(10, "Width", 1.5),
        location(20, "CornerRadius", 1.25), location(20, "CornerRadius", 1.75)]);
    assert_eq!(forward.track(10, "Width").unwrap().keyframes[2].easing, ease);

    paste.reverse = true;
    let (reversed, _) = target.with_pasted_keyframes(&paste).unwrap();
    let width = &reversed.track(10, "Width").unwrap().keyframes;
    assert_eq!(width[1], Keyframe::new(1.25, AnimationValue::Number(0.75)));
    assert_eq!(width[2].time, 1.75);
    assert_eq!(width[2].value, AnimationValue::Number(0.25));
    assert_eq!(width[2].easing, Easing::CubicBezier { x1: 0.25, y1: -0.5, x2: 0.875, y2: 1.25 });
    assert_eq!(reversed.track(20, "CornerRadius").unwrap().keyframes[1], Keyframe::new(1.0, radius(16.0)));

    paste.destinations.swap(0, 1);
    assert!(target.with_pasted_keyframes(&paste).unwrap_err().0.contains("cannot receive"));
    paste.destinations.swap(0, 1);
    paste.time = 1.000_000_03;
    assert!(reversed.with_pasted_keyframes(&paste).unwrap_err().0.contains("occupies"));
    paste.time = 2.5;
    assert!(target.with_pasted_keyframes(&paste).unwrap_err().0.contains("duration"));
    assert_eq!(target.tracks[0].keyframes.len(), 1);
}

#[test]
fn mirrored_bezier_samples_follow_the_reflection_and_leave_other_keys_in_place() {
    let mut source = Timeline::new(3.0).unwrap();
    for (time, value) in [(0.25, 0.2), (0.75, 0.7), (2.75, 1.0)] {
        source.record(1, "Opacity", time, AnimationValue::Number(value)).unwrap();
    }
    source.track_mut(1, "Opacity").unwrap().set_easing(0.75,
        Easing::CubicBezier { x1: 0.125, y1: -0.25, x2: 0.75, y2: 1.5 }).unwrap();
    let keys = [location(1, "Opacity", 0.25), location(1, "Opacity", 0.75)];
    let (mirrored, locations) = source.with_mirrored_keyframes(&keys, 1.125).unwrap();
    assert_eq!(locations, [location(1, "Opacity", 2.0), location(1, "Opacity", 1.5)]);
    for time in [0.25, 0.375, 0.5, 0.625, 0.75] {
        let AnimationValue::Number(before) = source.sample(time).unwrap()[0].value else { panic!("expected a number") };
        let AnimationValue::Number(after) = mirrored.sample(2.25 - time).unwrap()[0].value else { panic!("expected a number") };
        assert!((before - after).abs() < 1e-10);
    }
    assert_eq!(source.tracks[0].keyframes.last(), mirrored.tracks[0].keyframes.last());
    assert!(source.with_mirrored_keyframes(&keys, 1.5).unwrap_err().0.contains("occupies"));
    assert!(source.with_mirrored_keyframes(&keys, 0.25).unwrap_err().0.contains("duration"));
}
