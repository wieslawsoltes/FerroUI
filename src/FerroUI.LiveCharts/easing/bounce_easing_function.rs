pub struct BounceEasingFunction;
impl BounceEasingFunction {
    pub fn ease_in(t: f64) -> f64 {
        1.0 - Self::ease_out(1.0 - t)
    }
    pub fn ease_out(t: f64) -> f64 {
        let b0 = 121.0 / 16.0;
        if t < 4.0 / 11.0 {
            b0 * t * t
        } else if t < 8.0 / 11.0 {
            let t = t - 6.0 / 11.0;
            b0 * t * t + 3.0 / 4.0
        } else if t < 10.0 / 11.0 {
            let t = t - 9.0 / 11.0;
            b0 * t * t + 15.0 / 16.0
        } else {
            let t = t - 21.0 / 22.0;
            b0 * t * t + 63.0 / 64.0
        }
    }
    pub fn ease_in_out(t: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            (1.0 - Self::ease_out(1.0 - t)) / 2.0
        } else {
            (Self::ease_out(t - 1.0) + 1.0) / 2.0
        }
    }
}
