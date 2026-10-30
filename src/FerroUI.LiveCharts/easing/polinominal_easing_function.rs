/// The source's public spelling is retained for API traceability.
pub struct PolinominalEasingFunction;
impl PolinominalEasingFunction {
    pub fn ease_in(t: f64, exponent: f64) -> f64 {
        t.powf(exponent)
    }
    pub fn ease_out(t: f64, exponent: f64) -> f64 {
        1.0 - (1.0 - t).powf(exponent)
    }
    pub fn ease_in_out(t: f64, exponent: f64) -> f64 {
        let t = t * 2.0;
        if t <= 1.0 {
            t.powf(exponent) / 2.0
        } else {
            (2.0 - (2.0 - t).powf(exponent)) / 2.0
        }
    }
}
