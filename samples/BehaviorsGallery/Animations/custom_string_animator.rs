//! Port of `Animations/CustomStringAnimator.cs`.

use ferroui_base::animation::InterpolatingAnimator;

/// Animates a string by progressively revealing characters.
///
/// Deviation (DEVIATIONS.md, Behaviors gallery): the animated property is the text of a text block,
/// whose value type is an optional string; the animator interpolates that type (an absent text is
/// the empty string of the reference).
#[derive(Default)]
pub struct CustomStringAnimator;

impl InterpolatingAnimator for CustomStringAnimator {
    type Value = Option<String>;

    /// `Interpolate(double, string, string)`.
    fn interpolate(
        &self,
        progress: f64,
        _old_value: &Option<String>,
        new_value: &Option<String>,
    ) -> Option<String> {
        let new_value = match new_value.as_deref() {
            Some(value) if !value.is_empty() => value,
            _ => return Some(String::new()),
        };

        let characters: Vec<char> = new_value.chars().collect();
        let step = 1.0 / characters.len() as f64;
        let length = (progress / step) as i64;
        let count = (length + 1).clamp(0, characters.len() as i64) as usize;
        Some(characters[..count].iter().collect())
    }
}
