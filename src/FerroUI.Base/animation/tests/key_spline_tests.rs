use super::*;
use crate::animation::easings::*;
use crate::animation::{Animation, Cue, IterationCount, KeyFrame, KeySpline, PlaybackDirection};
use crate::media::{ITransform, RotateTransform, TranslateTransform};
use crate::styling::Setter;
use crate::threading::CancellationToken;

#[test]
// The type converter of the reference class is `KeySpline::parse`.
fn can_parse_key_spline_via_type_converter() {
    for input in ["1,2 3,4", "1 2 3 4", "1 2,3 4", "1,2,3,4"] {
        let spline = KeySpline::parse(input).unwrap();
        assert_eq!(spline.control_point_x1(), 1.0);
        assert_eq!(spline.control_point_y1(), 2.0);
        assert_eq!(spline.control_point_x2(), 3.0);
        assert_eq!(spline.control_point_y2(), 4.0);
    }
}

#[test]
fn can_handle_invalid_string_key_spline_via_type_converter() {
    for input in ["1,2F,3,4", "Foo,Bar,Fee,Buzz"] {
        assert!(KeySpline::parse(input).is_err());
    }
}

#[test]
fn key_spline_x_values_in_range_do_not_throw() {
    for input in [0.00, 0.50, 1.00] {
        let key_spline = KeySpline::new();
        key_spline.set_control_point_x1(input);
        key_spline.set_control_point_x2(input);
    }
}

#[test]
fn key_spline_x_values_cannot_be_out_of_range() {
    for input in [-0.01, 1.01] {
        let key_spline = KeySpline::new();
        assert_panics(|| key_spline.set_control_point_x1(input));
        assert_panics(|| key_spline.set_control_point_x2(input));
    }
}

#[test]
fn spline_easing_can_be_mutated() {
    let easing = SplineEasing::new();

    assert_eq!(easing.ease(0.0), 0.0);
    assert_eq!(easing.ease(1.0), 1.0);

    easing.set_x1(0.25);
    easing.set_y1(0.5);
    easing.set_x2(0.75);
    easing.set_y2(1.0);

    assert_ne!(easing.ease(0.5), 0.5);
}

#[test]
fn spline_easing_constructor_assigns_all_control_points() {
    let easing = SplineEasing::with_points(0.2, 0.8, 0.4, 0.3);

    assert_eq!(easing.x1(), 0.2);
    assert_eq!(easing.y1(), 0.8);
    assert_eq!(easing.x2(), 0.4);
    assert_eq!(easing.y2(), 0.3);

    let expected = SplineEasing::with_key_spline(KeySpline::with_points(0.2, 0.8, 0.4, 0.3));
    assert_eq!(easing.ease(0.5), expected.ease(0.5));
}

fn rotate_animation(key_spline: Option<Ref<KeySpline>>, easing: Option<Easing>) -> (Ref<Animation>, Ref<RotateTransform>, Ref<Border>) {
    let keyframe1 = KeyFrame::with_key_time(seconds(0.0), [Setter::new(RotateTransform::angle_property(), -2.5) as _]);
    let keyframe2 = KeyFrame::with_key_time(seconds(5.0), [Setter::new(RotateTransform::angle_property(), 2.5) as _]);
    keyframe2.set_key_spline(key_spline);

    let animation = Animation::new();
    animation.set_duration(seconds(5.0));
    animation.children().add(keyframe1);
    animation.children().add(keyframe2);
    animation.set_iteration_count(IterationCount::new(5));
    animation.set_playback_direction(PlaybackDirection::Alternate);
    if let Some(easing) = easing {
        animation.set_easing(easing);
    }

    let rotate_transform = RotateTransform::with_angle(-2.5);
    let rect = Border::new();
    let handle: Rc<dyn ITransform> = (&rotate_transform).into();
    rect.set_render_transform(Some(handle));
    (animation, rotate_transform, rect)
}

#[test]
fn check_key_spline_handled_properly() {
    let (animation, rotate_transform, rect) = rotate_animation(
        Some(KeySpline::with_points(0.1123555056179775, 0.657303370786517, 0.8370786516853934, 0.499999999999999999)),
        None,
    );

    let clock = TestClock::new();
    let _task = animation.run_async_with_clock(&rect, Some(clock.as_clock()), CancellationToken::none());

    // position is what you'd expect at end and beginning
    clock.step(TimeSpan::ZERO);
    assert_eq!(rotate_transform.angle(), -2.5);
    clock.step(seconds(5.0));
    assert_eq!(rotate_transform.angle(), 2.5);

    // test some points in between end and beginning
    let tolerance = 0.01;
    for (time, expected) in [
        ("00:00:10.0153932", -2.4122350198982545),
        ("00:00:11.2655407", -0.37153223002125113),
        ("00:00:12.6158773", 0.3967885416786294),
        ("00:00:14.6495256", 1.8016358493761722),
    ] {
        clock.step(TimeSpan::parse(time).unwrap());
        assert_close(rotate_transform.angle(), expected, tolerance);
    }
}

#[test]
fn check_key_spline_parsing_is_correct() {
    let (animation, rotate_transform, rect) =
        rotate_animation(Some(KeySpline::parse("0.1123555056179775,0.657303370786517,0.8370786516853934,0.499999999999999999").unwrap()), None);

    let clock = TestClock::new();
    let _task = animation.run_async_with_clock(&rect, Some(clock.as_clock()), CancellationToken::none());

    clock.step(TimeSpan::ZERO);
    assert_eq!(rotate_transform.angle(), -2.5);
    clock.step(seconds(5.0));
    assert_eq!(rotate_transform.angle(), 2.5);

    let tolerance = 0.01;
    for (time, expected) in [
        ("00:00:10.0153932", -2.4122350198982545),
        ("00:00:11.2655407", -0.37153223002125113),
        ("00:00:12.6158773", 0.3967885416786294),
        ("00:00:14.6495256", 1.8016358493761722),
    ] {
        clock.step(TimeSpan::parse(time).unwrap());
        assert_close(rotate_transform.angle(), expected, tolerance);
    }
}

#[test]
fn easing_parse_finds_every_easing_by_name() {
    for name in [
        "BackEaseIn", "BackEaseInOut", "BackEaseOut", "BounceEaseIn", "BounceEaseInOut", "BounceEaseOut",
        "CircularEaseIn", "CircularEaseInOut", "CircularEaseOut", "CubicEaseIn", "CubicEaseInOut", "CubicEaseOut",
        "ElasticEaseIn", "ElasticEaseInOut", "ElasticEaseOut", "ExponentialEaseIn", "ExponentialEaseInOut",
        "ExponentialEaseOut", "LinearEasing", "QuadraticEaseIn", "QuadraticEaseInOut", "QuadraticEaseOut",
        "QuarticEaseIn", "QuarticEaseInOut", "QuarticEaseOut", "QuinticEaseIn", "QuinticEaseInOut",
        "QuinticEaseOut", "SineEaseIn", "SineEaseInOut", "SineEaseOut", "SplineEasing", "SpringEasing",
    ] {
        let easing = Easing::parse(name).unwrap_or_else(|_| panic!("{name}"));
        if name != "SpringEasing" {
            assert_close(easing.ease(0.0), 0.0, 1e-3);
            assert_close(easing.ease(1.0), 1.0, 1e-3);
        }
    }
    assert!(Easing::parse("NoSuchEasing").is_err());
    assert!(Easing::parse("linearEasing").is_err());

    let spline = Easing::parse("0.25,0.5,0.75,1.0").unwrap();
    assert_ne!(spline.ease(0.5), 0.5);
    assert!(Easing::parse("0.25,0.5,x,1.0").is_err());
}

#[test]
fn cue_parse() {
    assert_eq!(Cue::parse("50%").unwrap(), Cue::new(0.5));
    assert_eq!(Cue::parse("100").unwrap(), Cue::new(1.0));
    assert_eq!(Cue::parse("0%").unwrap().cue_value(), 0.0);
    assert!(Cue::parse("abc").is_err());
    assert!(Cue::parse("150%").is_err());
    assert_panics(|| {
        Cue::new(1.5);
    });
}

#[test]
fn iteration_count_parse() {
    assert_eq!(IterationCount::parse("Infinite").unwrap(), IterationCount::INFINITE);
    assert_eq!(IterationCount::parse(" infinite ").unwrap(), IterationCount::INFINITE);
    assert_eq!(IterationCount::parse("3").unwrap(), IterationCount::new(3));
    assert!(IterationCount::parse("-1").is_err());
    assert!(IterationCount::parse("x").is_err());
    assert_eq!(IterationCount::INFINITE.to_string(), "Infinite");
    assert_eq!(IterationCount::new(7).to_string(), "7");
}

const OVERSHOOTING_EASINGS: [&str; 6] =
    ["BackEaseIn", "BackEaseOut", "BackEaseInOut", "ElasticEaseIn", "ElasticEaseOut", "ElasticEaseInOut"];

fn translate_animation(easing_type: &str, key_frames: &[(f64, f64)]) -> (Ref<Animation>, Ref<TranslateTransform>, Ref<Border>) {
    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    for (cue, value) in key_frames {
        animation
            .children()
            .add(KeyFrame::with_cue(Cue::new(*cue), [Setter::new(TranslateTransform::y_property(), *value) as _]));
    }
    animation.set_iteration_count(IterationCount::new(5));
    animation.set_playback_direction(PlaybackDirection::Alternate);
    animation.set_easing(Easing::parse(easing_type).unwrap());

    let transform = TranslateTransform::with_offset(0.0, 50.0);
    let rect = Border::new();
    let handle: Rc<dyn ITransform> = (&transform).into();
    rect.set_render_transform(Some(handle));
    (animation, transform, rect)
}

#[test]
fn key_spline_progress_less_than_zero_or_greater_than_one_works() {
    for easing_type in OVERSHOOTING_EASINGS {
        let (animation, transform, rect) = translate_animation(easing_type, &[(0.0, 10.0), (1.0, 20.0)]);

        let clock = TestClock::new();
        let _run = animation.run_async_with_clock(&rect, Some(clock.as_clock()), CancellationToken::none());

        clock.step(TimeSpan::ZERO);
        assert_close(transform.y(), 10.0, 0.0001);

        let mut time = seconds(0.1);
        while time < animation.duration() {
            clock.step(time);
            assert!(transform.y().is_finite());
            assert_ne!(transform.y(), 10.0);
            assert_ne!(transform.y(), 20.0);
            time += seconds(0.1);
        }

        clock.step(animation.duration());
        assert_close(transform.y(), 20.0, 0.0001);
    }
}

#[test]
fn key_spline_progress_less_than_zero_or_greater_than_one_works_with_single_key_frame() {
    for easing_type in OVERSHOOTING_EASINGS {
        let (animation, transform, rect) = translate_animation(easing_type, &[(1.0, 10.0)]);

        let clock = TestClock::new();
        let _run = animation.run_async_with_clock(&rect, Some(clock.as_clock()), CancellationToken::none());

        clock.step(TimeSpan::ZERO);
        assert_close(transform.y(), 50.0, 0.0001);

        let mut time = seconds(0.1);
        while time < animation.duration() {
            clock.step(time);
            assert!(transform.y().is_finite());
            assert_ne!(transform.y(), 50.0);
            assert_ne!(transform.y(), 10.0);
            time += seconds(0.1);
        }

        clock.step(animation.duration());
        assert_close(transform.y(), 10.0, 0.0001);
    }
}
