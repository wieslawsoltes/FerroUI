use super::*;
use crate::animation::easings::{IEasing, SpringEasing};
use crate::animation::{Animation, IterationCount, KeyFrame, PlaybackDirection, Spring};
use crate::media::{ITransform, RotateTransform};
use crate::styling::Setter;
use crate::threading::CancellationToken;

#[test]
fn can_parse_spring() {
    let spring = Spring::parse("1,2 3,4").unwrap();
    assert_eq!(spring.mass(), 1.0);
    assert_eq!(spring.stiffness(), 2.0);
    assert_eq!(spring.damping(), 3.0);
    assert_eq!(spring.initial_velocity(), 4.0);
}

#[test]
fn can_handle_invalid_string() {
    for input in ["1,2F,3,4", "Foo,Bar,Fee,Buzz"] {
        assert!(Spring::parse(input).is_err());
    }
}

#[test]
fn spring_easing_can_be_mutated() {
    let easing = SpringEasing::with_values(1.0, 1.0, 1.0, 0.0);

    assert_eq!(easing.ease(0.0), 0.0);
    assert_eq!(easing.ease(1.0), 0.34029984660829826);

    easing.set_mass(2.0);
    easing.set_stiffness(2.0);
    easing.set_damping(2.0);
    easing.set_initial_velocity(1.0);

    assert_ne!(easing.ease(0.5), 0.05136985716812037);
}

#[test]
fn check_spring_easing_handled_properly() {
    let keyframe1 = KeyFrame::with_key_time(seconds(0.0), [Setter::new(RotateTransform::angle_property(), -2.5) as _]);
    let keyframe2 = KeyFrame::with_key_time(seconds(5.0), [Setter::new(RotateTransform::angle_property(), 2.5) as _]);

    let animation = Animation::new();
    animation.set_duration(seconds(5.0));
    animation.children().add(keyframe1);
    animation.children().add(keyframe2);
    animation.set_iteration_count(IterationCount::new(5));
    animation.set_playback_direction(PlaybackDirection::Alternate);
    animation.set_easing(SpringEasing::with_values(1.0, 10.0, 1.0, 0.0));

    let rotate_transform = RotateTransform::with_angle(-2.5);
    let rect = Border::new();
    let handle: Rc<dyn ITransform> = (&rotate_transform).into();
    rect.set_render_transform(Some(handle));

    let clock = TestClock::new();
    let _task = animation.run_async_with_clock(&rect, Some(clock.as_clock()), CancellationToken::none());

    clock.step(TimeSpan::ZERO);
    assert_eq!(rotate_transform.angle(), -2.5);
    clock.step(seconds(5.0));
    assert_eq!(rotate_transform.angle(), 5.522828945000075);

    let tolerance = 0.01;
    for (time, expected) in [
        ("00:00:10.0153932", -2.499763294237805),
        ("00:00:11.2655407", -1.1011448950348934),
        ("00:00:12.6158773", 2.1264981706749007),
        ("00:00:14.6495256", 5.4337608446234782),
    ] {
        clock.step(TimeSpan::parse(time).unwrap());
        assert_close(rotate_transform.angle(), expected, tolerance);
    }
}
