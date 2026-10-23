pub struct ExponentialEasingFunction;
impl ExponentialEasingFunction {
    fn tpmt(x: f64) -> f64 {
        (2.0_f64.powf(-10.0 * x) - 0.0009765625) * 1.0009775171065494
    }
    pub fn ease_in(t: f64) -> f64 {
        Self::tpmt(1.0 - t)
    }
    pub fn ease_out(t: f64) -> f64 {
        1.0 - Self::tpmt(t)
    }
    pub fn ease_in_out(t: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            Self::tpmt(1.0 - t) / 2.0
        } else {
            (2.0 - Self::tpmt(t - 1.0)) / 2.0
        }
    }
}
