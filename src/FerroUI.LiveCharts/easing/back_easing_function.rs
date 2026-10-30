pub struct BackEasingFunction;
impl BackEasingFunction {
    pub fn ease_in(t: f64, s: f64) -> f64 {
        t * t * (s * (t - 1.0) + t)
    }
    pub fn ease_out(t: f64, s: f64) -> f64 {
        let t = t - 1.0;
        t * t * ((t + 1.0) * s + t) + 1.0
    }
    pub fn ease_in_out(t: f64, s: f64) -> f64 {
        let t = t * 2.0;
        if t < 1.0 {
            t * t * ((s + 1.0) * t - s) / 2.0
        } else {
            let t = t - 2.0;
            (t * t * ((s + 1.0) * t + s) + 2.0) / 2.0
        }
    }
}
