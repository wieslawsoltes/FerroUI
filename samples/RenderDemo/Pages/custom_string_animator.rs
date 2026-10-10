//! Port of `Pages/CustomStringAnimator.cs`.
//!
//! One deviation: the number of characters the animator takes is limited to the length of the
//! text (see `interpolate`).

use ferroui_base::animation::{ICustomAnimator, InterpolatingAnimator};
use ferroui_base::ferro_markup_type;
use std::rc::{Rc, Weak};

pub struct CustomStringAnimator {
    this: Weak<CustomStringAnimator>,
}

impl PartialEq for CustomStringAnimator {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CustomStringAnimator {
    pub fn new() -> Rc<CustomStringAnimator> {
        Rc::new_cyclic(|this| Self { this: this.clone() })
    }

    /// The animator as the custom animator contract.
    pub fn as_custom_animator(&self) -> Rc<dyn ICustomAnimator> {
        self.this.upgrade().expect("the animator is alive while it is used")
    }
}

impl InterpolatingAnimator for CustomStringAnimator {
    /// The value type of the text properties the animator animates (`string`).
    type Value = Option<String>;

    fn interpolate(&self, progress: f64, _old_value: &Option<String>, new_value: &Option<String>) -> Option<String> {
        // The lengths and the indices of a string are counted in UTF-16 code units.
        let new_value: Vec<u16> = new_value.as_deref().unwrap_or_default().encode_utf16().collect();
        if new_value.is_empty() {
            return Some(String::new());
        }
        let step = 1.0 / new_value.len() as f64;
        let length = (progress / step) as i32;
        // Deviation (DEVIATIONS.md, RenderDemo sample; GAPS.md, R002): `Substring(0, length + 1)`
        // of the managed original throws at a progress of one, where `length + 1` is one more
        // than the text has, and the framework asks for the value at a progress of one when it
        // ends the animation as its element leaves the tree. The port takes the whole text
        // there.
        let end = usize::try_from(length.saturating_add(1)).unwrap_or(0).min(new_value.len());
        let result = String::from_utf16_lossy(&new_value[..end]);
        Some(result)
    }
}

ferro_markup_type!(class CustomStringAnimator {
    this: Rc<CustomStringAnimator>,
    handles: [CustomStringAnimator, Rc<CustomStringAnimator>, Option<Rc<CustomStringAnimator>>],
    interfaces: [Rc<dyn ICustomAnimator>],
    constructors: [() => CustomStringAnimator::new],
});
