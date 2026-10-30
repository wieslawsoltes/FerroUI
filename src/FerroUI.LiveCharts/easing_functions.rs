//! Predefined and custom chart easing functions. Samples use FerroUI's f64 units.

use crate::easing::*;
use std::{rc::Rc, sync::OnceLock};

pub type EasingFunction = Rc<dyn Fn(f64) -> f64>;

#[derive(Clone)]
pub struct KeyFrame {
    pub time: f64,
    pub value: f64,
    /// Easing of the interval ending at this key frame.
    pub easing_function: EasingFunction,
}
impl KeyFrame {
    pub fn new(time: f64, value: f64) -> Self {
        Self {
            time,
            value,
            easing_function: Rc::new(EasingFunctions::lineal),
        }
    }
}

pub struct EasingFunctions;
impl EasingFunctions {
    pub fn lineal(t: f64) -> f64 {
        t
    }
    pub fn back_in(t: f64) -> f64 {
        BackEasingFunction::ease_in(t, 1.70158)
    }
    pub fn back_out(t: f64) -> f64 {
        BackEasingFunction::ease_out(t, 1.70158)
    }
    pub fn back_in_out(t: f64) -> f64 {
        BackEasingFunction::ease_in_out(t, 1.70158)
    }
    pub fn bounce_in(t: f64) -> f64 {
        BounceEasingFunction::ease_in(t)
    }
    pub fn bounce_out(t: f64) -> f64 {
        BounceEasingFunction::ease_out(t)
    }
    pub fn bounce_in_out(t: f64) -> f64 {
        BounceEasingFunction::ease_in_out(t)
    }
    pub fn circle_in(t: f64) -> f64 {
        CircleEasingFunction::ease_in(t)
    }
    pub fn circle_out(t: f64) -> f64 {
        CircleEasingFunction::ease_out(t)
    }
    pub fn circle_in_out(t: f64) -> f64 {
        CircleEasingFunction::ease_in_out(t)
    }
    pub fn cubic_in(t: f64) -> f64 {
        CubicEasingFunction::ease_in(t)
    }
    pub fn cubic_out(t: f64) -> f64 {
        CubicEasingFunction::ease_out(t)
    }
    pub fn cubic_in_out(t: f64) -> f64 {
        CubicEasingFunction::ease_in_out(t)
    }
    pub fn elastic_in(t: f64) -> f64 {
        ElasticEasingFunction::ease_in(t, 1.0, 0.3)
    }
    pub fn elastic_out(t: f64) -> f64 {
        ElasticEasingFunction::ease_out(t, 1.0, 0.3)
    }
    pub fn elastic_in_out(t: f64) -> f64 {
        ElasticEasingFunction::ease_in_out(t, 1.0, 0.3)
    }
    pub fn exponential_in(t: f64) -> f64 {
        ExponentialEasingFunction::ease_in(t)
    }
    pub fn exponential_out(t: f64) -> f64 {
        ExponentialEasingFunction::ease_out(t)
    }
    pub fn exponential_in_out(t: f64) -> f64 {
        ExponentialEasingFunction::ease_in_out(t)
    }
    pub fn polinominal_in(t: f64) -> f64 {
        PolinominalEasingFunction::ease_in(t, 3.0)
    }
    pub fn polinominal_out(t: f64) -> f64 {
        PolinominalEasingFunction::ease_out(t, 3.0)
    }
    pub fn polinominal_in_out(t: f64) -> f64 {
        PolinominalEasingFunction::ease_in_out(t, 3.0)
    }
    pub fn quadratic_in(t: f64) -> f64 {
        t * t
    }
    pub fn quadratic_out(t: f64) -> f64 {
        t * (2.0 - t)
    }
    pub fn quadratic_in_out(t: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            t * t / 2.0
        } else {
            let t = t - 1.0;
            (t * (2.0 - t) + 1.0) / 2.0
        }
    }
    pub fn sin_in(t: f64) -> f64 {
        if t == 1.0 {
            1.0
        } else {
            1.0 - (t * std::f64::consts::FRAC_PI_2).cos()
        }
    }
    pub fn sin_out(t: f64) -> f64 {
        (t * std::f64::consts::FRAC_PI_2).sin()
    }
    pub fn sin_in_out(t: f64) -> f64 {
        (1.0 - (std::f64::consts::PI * t).cos()) / 2.0
    }
    pub fn ease(t: f64) -> f64 {
        static CURVE: OnceLock<CubicBezierEasingFunction> = OnceLock::new();
        CURVE
            .get_or_init(|| {
                CubicBezierEasingFunction::new(0.25, 0.1, 0.25, 1.0)
                    .expect("constant control points are valid")
            })
            .sample(t)
    }
    pub fn ease_in(t: f64) -> f64 {
        static CURVE: OnceLock<CubicBezierEasingFunction> = OnceLock::new();
        CURVE
            .get_or_init(|| {
                CubicBezierEasingFunction::new(0.42, 0.0, 1.0, 1.0)
                    .expect("constant control points are valid")
            })
            .sample(t)
    }
    pub fn ease_out(t: f64) -> f64 {
        static CURVE: OnceLock<CubicBezierEasingFunction> = OnceLock::new();
        CURVE
            .get_or_init(|| {
                CubicBezierEasingFunction::new(0.0, 0.0, 0.58, 1.0)
                    .expect("constant control points are valid")
            })
            .sample(t)
    }
    pub fn ease_in_out(t: f64) -> f64 {
        static CURVE: OnceLock<CubicBezierEasingFunction> = OnceLock::new();
        CURVE
            .get_or_init(|| {
                CubicBezierEasingFunction::new(0.42, 0.0, 0.58, 1.0)
                    .expect("constant control points are valid")
            })
            .sample(t)
    }
    pub fn build_custom_back_in(s: f64) -> EasingFunction {
        Rc::new(move |t| BackEasingFunction::ease_in(t, s))
    }
    pub fn build_custom_back_out(s: f64) -> EasingFunction {
        Rc::new(move |t| BackEasingFunction::ease_out(t, s))
    }
    pub fn build_custom_back_in_out(s: f64) -> EasingFunction {
        Rc::new(move |t| BackEasingFunction::ease_in_out(t, s))
    }
    pub fn build_custom_elastic_in(a: f64, p: f64) -> EasingFunction {
        Rc::new(move |t| ElasticEasingFunction::ease_in(t, a, p))
    }
    pub fn build_custom_elastic_out(a: f64, p: f64) -> EasingFunction {
        Rc::new(move |t| ElasticEasingFunction::ease_out(t, a, p))
    }
    pub fn build_custom_elastic_in_out(a: f64, p: f64) -> EasingFunction {
        Rc::new(move |t| ElasticEasingFunction::ease_in_out(t, a, p))
    }
    pub fn build_custom_polinominal_in(e: f64) -> EasingFunction {
        Rc::new(move |t| PolinominalEasingFunction::ease_in(t, e))
    }
    pub fn build_custom_polinominal_out(e: f64) -> EasingFunction {
        Rc::new(move |t| PolinominalEasingFunction::ease_out(t, e))
    }
    pub fn build_custom_polinominal_in_out(e: f64) -> EasingFunction {
        Rc::new(move |t| PolinominalEasingFunction::ease_in_out(t, e))
    }
    pub fn build_cubic_bezier(
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    ) -> Result<EasingFunction, EasingError> {
        let curve = CubicBezierEasingFunction::new(x1, y1, x2, y2)?;
        Ok(Rc::new(move |t| curve.sample(t)))
    }
    pub fn build_function_using_key_frames(
        mut frames: Vec<KeyFrame>,
    ) -> Result<EasingFunction, EasingError> {
        if frames.len() < 2 {
            return Err(EasingError::TooFewKeyFrames);
        }
        if frames
            .iter()
            .any(|f| !f.time.is_finite() || !(0.0..=1.0).contains(&f.time) || !f.value.is_finite())
            || frames.windows(2).any(|w| w[0].time >= w[1].time)
        {
            return Err(EasingError::InvalidKeyFrameTime);
        }
        let last = &frames[frames.len() - 1];
        if last.time < 1.0 {
            frames.push(KeyFrame::new(1.0, last.value));
        }
        Ok(Rc::new(move |t| {
            let mut i = 0;
            while i < frames.len() - 2 && frames[i + 1].time < t {
                i += 1;
            }
            let current = &frames[i];
            let next = &frames[i + 1];
            let progress = (t - current.time) / (next.time - current.time);
            current.value + (next.easing_function)(progress) * (next.value - current.value)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_predefined_easing_boundaries_match_source() {
        let functions: [fn(f64) -> f64; 32] = [
            EasingFunctions::lineal,
            EasingFunctions::back_in,
            EasingFunctions::back_out,
            EasingFunctions::back_in_out,
            EasingFunctions::bounce_in,
            EasingFunctions::bounce_out,
            EasingFunctions::bounce_in_out,
            EasingFunctions::circle_in,
            EasingFunctions::circle_out,
            EasingFunctions::circle_in_out,
            EasingFunctions::cubic_in,
            EasingFunctions::cubic_out,
            EasingFunctions::cubic_in_out,
            EasingFunctions::elastic_in,
            EasingFunctions::elastic_out,
            EasingFunctions::elastic_in_out,
            EasingFunctions::exponential_in,
            EasingFunctions::exponential_out,
            EasingFunctions::exponential_in_out,
            EasingFunctions::polinominal_in,
            EasingFunctions::polinominal_out,
            EasingFunctions::polinominal_in_out,
            EasingFunctions::quadratic_in,
            EasingFunctions::quadratic_out,
            EasingFunctions::quadratic_in_out,
            EasingFunctions::sin_in,
            EasingFunctions::sin_out,
            EasingFunctions::sin_in_out,
            EasingFunctions::ease,
            EasingFunctions::ease_in,
            EasingFunctions::ease_out,
            EasingFunctions::ease_in_out,
        ];
        for (i, f) in functions.into_iter().enumerate() {
            assert!(f(0.0).abs() < 1e-8, "function {i}");
            assert!((f(1.0) - 1.0).abs() < 1e-8, "function {i}");
        }
        assert_eq!(EasingFunctions::lineal(0.5), 0.5);
        assert_eq!(EasingFunctions::quadratic_in(0.5), 0.25);
    }
    #[test]
    fn customary_easing_curves_are_monotonic() {
        for f in [
            EasingFunctions::lineal,
            EasingFunctions::sin_out,
            EasingFunctions::cubic_out,
            EasingFunctions::circle_out,
        ] {
            let mut previous = f(0.0);
            for i in 1..=100 {
                let current = f(i as f64 / 100.0);
                assert!(current >= previous);
                previous = current;
            }
        }
    }
    #[test]
    fn custom_easing_parameters_match_source_boundaries() {
        for f in [
            EasingFunctions::build_custom_back_in(1.70158),
            EasingFunctions::build_custom_back_out(1.70158),
            EasingFunctions::build_custom_back_in_out(1.70158),
            EasingFunctions::build_custom_elastic_in(1.0, 0.3),
            EasingFunctions::build_custom_elastic_out(1.0, 0.3),
            EasingFunctions::build_custom_elastic_in_out(1.0, 0.3),
            EasingFunctions::build_custom_polinominal_in(4.0),
            EasingFunctions::build_custom_polinominal_out(4.0),
            EasingFunctions::build_custom_polinominal_in_out(4.0),
        ] {
            assert!(f(0.0).abs() < 1e-8);
            assert!((f(1.0) - 1.0).abs() < 1e-8);
        }
    }
    #[test]
    fn key_frames_interpolate_and_extend_final_value() {
        let f = EasingFunctions::build_function_using_key_frames(vec![
            KeyFrame::new(0.0, 0.0),
            KeyFrame::new(1.0, 100.0),
        ])
        .unwrap();
        assert_eq!((f(0.0), f(0.5), f(1.0)), (0.0, 50.0, 100.0));
        let f = EasingFunctions::build_function_using_key_frames(vec![
            KeyFrame::new(0.0, 0.0),
            KeyFrame::new(0.5, 50.0),
        ])
        .unwrap();
        assert_eq!(f(1.0), 50.0);
    }
    #[test]
    fn key_frames_use_destination_easing_and_reject_undefined_segments() {
        let f = EasingFunctions::build_function_using_key_frames(vec![
            KeyFrame::new(0.0, 0.0),
            KeyFrame {
                time: 1.0,
                value: 100.0,
                easing_function: Rc::new(EasingFunctions::quadratic_in),
            },
        ])
        .unwrap();
        assert_eq!(f(0.5), 25.0);
        assert!(
            EasingFunctions::build_function_using_key_frames(vec![KeyFrame::new(0.0, 0.0)])
                .is_err()
        );
        assert!(
            EasingFunctions::build_function_using_key_frames(vec![
                KeyFrame::new(0.5, 0.0),
                KeyFrame::new(0.5, 100.0)
            ])
            .is_err()
        );
    }

    #[test]
    fn source_key_frame_triangle_and_delayed_ramp() {
        let triangle = EasingFunctions::build_function_using_key_frames(vec![
            KeyFrame::new(0.0, 0.0),
            KeyFrame::new(0.5, 1.0),
            KeyFrame::new(1.0, 0.0),
        ])
        .unwrap();
        for (i, expected) in [0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 0.8, 0.6, 0.4, 0.2, 0.0]
            .into_iter()
            .enumerate()
        {
            assert!((triangle(i as f64 / 10.0) - expected).abs() < 1e-9);
        }
        let ramp = EasingFunctions::build_function_using_key_frames(vec![
            KeyFrame::new(0.0, 0.0),
            KeyFrame::new(0.3, 0.0),
            KeyFrame::new(0.8, 1.0),
            KeyFrame::new(1.0, 1.0),
        ])
        .unwrap();
        for (i, expected) in [0.0, 0.0, 0.0, 0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.0, 1.0]
            .into_iter()
            .enumerate()
        {
            assert!((ramp(i as f64 / 10.0) - expected).abs() < 1e-9);
        }
    }
}
