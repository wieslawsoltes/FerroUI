pub struct CircleEasingFunction;
impl CircleEasingFunction {
    pub fn ease_in(t: f64) -> f64 {
        1.0 - (1.0 - t * t).sqrt()
    }
    pub fn ease_out(t: f64) -> f64 {
        let t = t - 1.0;
        (1.0 - t * t).sqrt()
    }
    pub fn ease_in_out(t: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            (1.0 - (1.0 - t * t).sqrt()) / 2.0
        } else {
            let t = t - 2.0;
            ((1.0 - t * t).sqrt() + 1.0) / 2.0
        }
    }
}
