pub struct CubicEasingFunction;
impl CubicEasingFunction {
    pub fn ease_in(t: f64) -> f64 {
        t * t * t
    }
    pub fn ease_out(t: f64) -> f64 {
        let t = t - 1.0;
        t * t * t + 1.0
    }
    pub fn ease_in_out(t: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            t * t * t / 2.0
        } else {
            let t = t - 2.0;
            (t * t * t + 2.0) / 2.0
        }
    }
}
