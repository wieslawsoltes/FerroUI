//! `AppBuilderScope`.

use ferroui_splat::AppBuilder;

/// Saves the state of the builder of Splat, resets it, and restores it when dropped.
pub struct AppBuilderScope {
    saved_has_been_built: bool,
    saved_using_builder: bool,
}

impl AppBuilderScope {
    pub fn new() -> Self {
        let (saved_has_been_built, saved_using_builder) = AppBuilder::get_state();
        AppBuilder::reset_state();
        Self { saved_has_been_built, saved_using_builder }
    }
}

impl Drop for AppBuilderScope {
    fn drop(&mut self) {
        AppBuilder::restore_state(self.saved_has_been_built, self.saved_using_builder);
    }
}
