use super::{Animation, Discrete, Interpolate, Repeat};

/// A retargetable value on a host-owned animation timeline.
///
/// The timeline has no global clock, platform timer or thread dependency. A
/// compositor/dispatcher supplies one timestamp to every property in a frame.
#[derive(Clone, Debug)]
pub struct MotionProperty<T: Interpolate> {
    pub from_value: T,
    pub to_value: T,
    pub default_value: T,
    pub animation: Option<Animation>,
    start_time: f64,
    end_time: f64,
    repeat_count: u32,
    is_completed: bool,
    force_finish: bool,
}

impl<T: Interpolate> MotionProperty<T> {
    pub fn new(default_value: T) -> Self {
        Self {
            from_value: default_value.clone(),
            to_value: default_value.clone(),
            default_value,
            animation: None,
            start_time: 0.0,
            end_time: 0.0,
            repeat_count: 0,
            is_completed: true,
            force_finish: false,
        }
    }
    pub fn is_completed(&self) -> bool {
        self.is_completed
    }
    pub fn repeat_count(&self) -> u32 {
        self.repeat_count
    }

    pub fn set_movement(&mut self, value: T, now_ms: f64) {
        self.from_value = self.get_movement(now_ms);
        self.to_value = value;
        self.force_finish = false;
        if let Some(animation) = &self.animation {
            self.is_completed = false;
            self.start_time = now_ms;
            self.end_time = now_ms + animation.duration;
            self.repeat_count = 0;
        } else {
            self.is_completed = true;
        }
    }

    pub fn get_movement(&mut self, now_ms: f64) -> T {
        let Some(animation) = &self.animation else {
            self.is_completed = true;
            return self.to_value.clone();
        };
        let Some(easing) = &animation.easing_function else {
            self.is_completed = true;
            return self.to_value.clone();
        };
        if self.force_finish
            || !now_ms.is_finite()
            || !animation.duration.is_finite()
            || !T::can_interpolate(&self.from_value, &self.to_value)
        {
            self.is_completed = true;
            return self.to_value.clone();
        }
        let mut delta = self.end_time - self.start_time;
        let mut progress = (now_ms - self.start_time) / delta;
        let remaining = match animation.repeat_times {
            Repeat::None => 0,
            Repeat::Count(count) => count.saturating_sub(self.repeat_count),
            Repeat::Forever => u32::MAX,
        };
        // A stalled frame advances by whole cycles, preserving the fractional
        // phase, and cannot run past the finite remaining repeat budget.
        if delta > 0.0 && progress > 1.0 && remaining > 0 {
            let cycles = progress.floor();
            let cycles = if animation.repeat_times == Repeat::Forever {
                cycles
            } else {
                cycles.min(f64::from(remaining))
            };
            if animation.repeat_times != Repeat::Forever {
                self.repeat_count = self.repeat_count.saturating_add(cycles as u32);
            }
            self.start_time += cycles * delta;
            self.end_time = self.start_time + animation.duration;
            delta = self.end_time - self.start_time;
            progress = (now_ms - self.start_time) / delta;
        }
        if delta <= 0.0 || progress > 1.0 || !progress.is_finite() {
            self.is_completed = true;
            return self.to_value.clone();
        }
        if progress >= 1.0 {
            progress = 1.0;
            let repeat = match animation.repeat_times {
                Repeat::Forever => true,
                Repeat::Count(count) => {
                    self.repeat_count = self.repeat_count.saturating_add(1);
                    count >= self.repeat_count
                }
                Repeat::None => false,
            };
            if repeat {
                self.start_time = now_ms;
                self.end_time = now_ms + animation.duration;
            }
        }
        T::interpolate(&self.from_value, &self.to_value, easing(progress))
    }

    pub fn finish(&mut self) {
        self.force_finish = true;
    }
    pub fn copy_from(&mut self, source: &Self) {
        *self = source.clone();
    }
}

impl<T: Interpolate + Default> Default for MotionProperty<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[derive(Clone, Debug)]
pub struct ByReferenceMotionProperty<T: Clone>(MotionProperty<Discrete<T>>);
impl<T: Clone> ByReferenceMotionProperty<T> {
    pub fn new(value: T) -> Self {
        Self(MotionProperty::new(Discrete(value)))
    }
    pub fn set_movement(&mut self, value: T, now_ms: f64) {
        self.0.set_movement(Discrete(value), now_ms);
    }
    pub fn get_movement(&mut self, now_ms: f64) -> T {
        self.0.get_movement(now_ms).0
    }
    pub fn is_completed(&self) -> bool {
        self.0.is_completed()
    }
    pub fn set_animation(&mut self, animation: Option<Animation>) {
        self.0.animation = animation;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing_functions::EasingFunctions;
    use ferroui_base::media::Color;

    fn linear() -> MotionProperty<f64> {
        let mut p = MotionProperty::new(0.0);
        p.animation = Some(Animation::new(100.0, EasingFunctions::lineal));
        p
    }
    #[test]
    fn retarget_preserves_current_interpolated_value() {
        let mut p = linear();
        p.set_movement(100.0, 0.0);
        assert_eq!(p.get_movement(25.0), 25.0);
        p.set_movement(200.0, 50.0);
        assert_eq!(p.from_value, 50.0);
        assert_eq!(p.get_movement(100.0), 125.0);
        assert_eq!(p.get_movement(150.0), 200.0);
        assert!(!p.is_completed());
        assert_eq!(p.get_movement(151.0), 200.0);
        assert!(p.is_completed());
    }
    #[test]
    fn repeating_animation_overshoot_preserves_phase() {
        let mut p = linear();
        p.animation.as_mut().unwrap().repeat_times = Repeat::Forever;
        p.set_movement(100.0, 0.0);
        assert!((p.get_movement(335.0) - 35.0).abs() < 1e-9);
        assert!(!p.is_completed());
        assert!((p.get_movement(10_000_025.0) - 25.0).abs() < 1e-9);
    }
    #[test]
    fn finite_repeat_budget_survives_long_stall() {
        let mut p = linear();
        p.animation.as_mut().unwrap().repeat_times = Repeat::Count(2);
        p.set_movement(100.0, 0.0);
        assert_eq!(p.get_movement(250.0), 50.0);
        assert_eq!(p.repeat_count(), 2);
        assert_eq!(p.get_movement(1000.0), 100.0);
        assert!(p.is_completed());
    }
    #[test]
    fn repeat_exact_boundary_returns_end_before_next_cycle() {
        let mut p = linear();
        p.animation.as_mut().unwrap().repeat_times = Repeat::Count(1);
        p.set_movement(100.0, 0.0);
        assert_eq!(p.get_movement(100.0), 100.0);
        assert!(!p.is_completed());
        assert_eq!(p.get_movement(125.0), 25.0);
        assert_eq!(p.get_movement(200.0), 100.0);
        assert!(!p.is_completed());
        assert_eq!(p.get_movement(201.0), 100.0);
        assert!(p.is_completed());
    }
    #[test]
    fn zero_duration_finish_and_disabled_animation_snap() {
        let mut p = linear();
        p.animation.as_mut().unwrap().duration = 0.0;
        p.set_movement(7.0, 0.0);
        assert_eq!(p.get_movement(0.0), 7.0);
        assert!(p.is_completed());
        let mut p = linear();
        p.set_movement(9.0, 0.0);
        p.finish();
        assert_eq!(p.get_movement(20.0), 9.0);
        p.animation = None;
        p.set_movement(3.0, 25.0);
        assert_eq!(p.get_movement(25.0), 3.0);
    }
    #[test]
    fn nullable_array_and_reference_values_complete_without_transition() {
        let mut p = MotionProperty::new(None::<f64>);
        p.animation = Some(Animation::new(100.0, EasingFunctions::lineal));
        p.set_movement(Some(10.0), 0.0);
        assert_eq!(p.get_movement(10.0), Some(10.0));
        assert!(p.is_completed());
        let mut p = MotionProperty::new(Some(vec![0.0_f32]));
        p.animation = Some(Animation::new(100.0, EasingFunctions::lineal));
        p.set_movement(Some(vec![2.0, 3.0]), 0.0);
        assert_eq!(p.get_movement(10.0), Some(vec![2.0, 3.0]));
        assert!(p.is_completed());
        let mut p = ByReferenceMotionProperty::new("before");
        p.set_animation(Some(Animation::new(100.0, EasingFunctions::lineal)));
        p.set_movement("after", 0.0);
        assert_eq!(p.get_movement(10.0), "after");
        assert!(p.is_completed());
    }
    #[test]
    fn color_interpolation_truncates_source_channels() {
        let a = Color::from_argb(0, 0, 20, 255);
        let b = Color::from_argb(255, 255, 40, 0);
        assert_eq!(
            Color::interpolate(&a, &b, 0.5),
            Color::from_argb(127, 127, 30, 127)
        );
    }

    #[test]
    fn source_animation_complete_observes_next_frame_after_exact_endpoint() {
        let mut p = MotionProperty::new(0.0);
        p.animation = Some(Animation::new(1000.0, EasingFunctions::lineal));
        p.set_movement(100.0, 0.0);
        assert_eq!(p.get_movement(500.0), 50.0);
        assert!(!p.is_completed());
        assert_eq!(p.get_movement(1000.0), 100.0);
        assert!(!p.is_completed());
        assert_eq!(p.get_movement(1500.0), 100.0);
        assert!(p.is_completed());
    }

    #[test]
    fn source_infinite_animation_exact_frames() {
        let mut p = MotionProperty::new(50.0);
        p.animation =
            Some(Animation::new(1000.0, EasingFunctions::lineal).with_repeat(Repeat::Forever));
        p.set_movement(100.0, 50.0);
        for (time, expected) in [
            (50.0, 50.0),
            (550.0, 75.0),
            (1050.0, 100.0),
            (1550.0, 75.0),
            (2050.0, 100.0),
        ] {
            assert_eq!(p.get_movement(time), expected);
            assert!(!p.is_completed());
        }
    }

    #[test]
    fn source_not_finished_animation_retargets_at_eighty_milliseconds() {
        let mut p = linear();
        p.animation.as_mut().unwrap().repeat_times = Repeat::Forever;
        p.set_movement(100.0, 0.0);
        for i in 0..=10 {
            let time = f64::from(i * 10);
            if i == 8 {
                p.set_movement(200.0, time);
            }
            let expected = if i < 8 {
                time
            } else {
                80.0 + (time - 80.0) / 100.0 * 120.0
            };
            assert!((p.get_movement(time) - expected).abs() < 1e-9);
        }
    }
}
