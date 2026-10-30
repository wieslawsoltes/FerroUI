//! Registered-property fixtures from GradientPaintAnimationTests.cs. Native
//! brush interpolation and gradient-space resolution are verified in paint.rs.

use super::{Animatable, Animation};
use crate::EasingFunctions;
use ferroui_base::{Point, media::Color};

#[test]
fn source_gradient_stops_interpolate_per_channel_and_complete() {
    let from = vec![Color::from_rgb(0, 0, 0); 2];
    let to = vec![
        Color::from_rgb(200, 100, 50),
        Color::from_rgb(100, 200, 150),
    ];
    let mut paint = Animatable::new();
    let stops = paint
        .register_motion("GradientStops", from.clone())
        .unwrap();
    paint
        .set_transition(
            Some(Animation::new(1000.0, EasingFunctions::lineal)),
            &["GradientStops"],
        )
        .unwrap();
    paint.set(&stops, from, 0.0).unwrap();
    paint.complete_transition(&["GradientStops"]).unwrap();
    paint.set(&stops, to.clone(), 0.0).unwrap();
    paint.begin_frame();
    assert_eq!(
        paint.get(&stops, 500.0).unwrap(),
        [Color::from_rgb(100, 50, 25), Color::from_rgb(50, 100, 75)]
    );
    assert!(!paint.is_valid);
    paint.begin_frame();
    assert_eq!(paint.get(&stops, 2000.0).unwrap(), to);
    assert!(paint.motion(&stops).unwrap().unwrap().is_completed());
    assert!(paint.is_valid);
}

#[test]
fn source_gradient_stops_mismatched_length_snaps_to_target() {
    let from = vec![Color::from_rgb(0, 0, 0); 2];
    let to = vec![
        Color::from_rgb(255, 0, 0),
        Color::from_rgb(0, 128, 0),
        Color::from_rgb(0, 0, 255),
    ];
    let mut paint = Animatable::new();
    let stops = paint
        .register_motion("GradientStops", from.clone())
        .unwrap();
    paint
        .set_transition(
            Some(Animation::new(1000.0, EasingFunctions::lineal)),
            &["GradientStops"],
        )
        .unwrap();
    paint.set(&stops, from, 0.0).unwrap();
    paint.complete_transition(&["GradientStops"]).unwrap();
    paint.set(&stops, to.clone(), 0.0).unwrap();
    paint.begin_frame();
    assert_eq!(paint.get(&stops, 500.0).unwrap(), to);
    assert!(paint.motion(&stops).unwrap().unwrap().is_completed());
    assert!(paint.is_valid);
}

#[test]
fn source_linear_start_and_radial_center_interpolate() {
    for name in ["StartPoint", "Center"] {
        let from = Point::new(0.0, 0.0);
        let to = Point::new(1.0, 1.0);
        let mut paint = Animatable::new();
        let point = paint.register_motion(name, from).unwrap();
        paint
            .set_transition(
                Some(Animation::new(1000.0, EasingFunctions::lineal)),
                &[name],
            )
            .unwrap();
        paint.set(&point, from, 0.0).unwrap();
        paint.complete_transition(&[name]).unwrap();
        paint.set(&point, to, 0.0).unwrap();
        paint.begin_frame();
        assert_eq!(paint.get(&point, 500.0).unwrap(), Point::new(0.5, 0.5));
        assert!(!paint.is_valid);
    }
}

#[test]
fn source_color_positions_interpolate_then_null_snaps() {
    let from = Some(vec![0.0_f32, 1.0_f32]);
    let to = Some(vec![0.2_f32, 0.8_f32]);
    let mut paint = Animatable::new();
    let positions = paint.register_motion("ColorPos", from.clone()).unwrap();
    paint
        .set_transition(
            Some(Animation::new(1000.0, EasingFunctions::lineal)),
            &["ColorPos"],
        )
        .unwrap();
    paint.set(&positions, from, 0.0).unwrap();
    paint.complete_transition(&["ColorPos"]).unwrap();
    paint.set(&positions, to, 0.0).unwrap();
    paint.begin_frame();
    let mid = paint.get(&positions, 500.0).unwrap().unwrap();
    assert!((mid[0] - 0.1).abs() < 0.001);
    assert!((mid[1] - 0.9).abs() < 0.001);
    paint.set(&positions, None, 500.0).unwrap();
    paint.begin_frame();
    assert!(paint.get(&positions, 600.0).unwrap().is_none());
    assert!(paint.is_valid);
}
