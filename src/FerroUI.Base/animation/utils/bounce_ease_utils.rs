/// Helper function for the bounce easings.
pub(crate) struct BounceEaseUtils;

impl BounceEaseUtils {
    /// Returns the consequent value of the simulated bounce function.
    pub(crate) fn bounce(progress: f64) -> f64 {
        let p = progress;
        if p < 4.0 / 11.0 {
            (121.0 * p * p) / 16.0
        } else if p < 8.0 / 11.0 {
            (363.0 / 40.0 * p * p) - (99.0 / 10.0 * p) + 17.0 / 5.0
        } else if p < 9.0 / 10.0 {
            (4356.0 / 361.0 * p * p) - (35442.0 / 1805.0 * p) + 16061.0 / 1805.0
        } else {
            (54.0 / 5.0 * p * p) - (513.0 / 25.0 * p) + 268.0 / 25.0
        }
    }
}
