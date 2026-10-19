//! `ModeDetectorScope`.

use ferroui_splat::{IModeDetector, ModeDetector};
use std::rc::Rc;

/// Saves the detector of [`ModeDetector`] and its cached result, resets them, and restores them
/// when dropped.
pub struct ModeDetectorScope {
    saved: Option<(Rc<dyn IModeDetector>, Option<bool>)>,
}

impl ModeDetectorScope {
    pub fn new() -> Self {
        let saved = ModeDetector::get_state();
        ModeDetector::reset_state();
        Self { saved: Some(saved) }
    }
}

impl Drop for ModeDetectorScope {
    fn drop(&mut self) {
        if let Some(saved) = self.saved.take() {
            ModeDetector::restore_state(saved);
        }
    }
}
