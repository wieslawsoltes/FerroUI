//! `InternalLocatorScope`.

use ferroui_splat::service_location::InternalLocator;
use ferroui_splat::{AppLocator, ServiceLocationInitializationExt};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Saves the locator of [`AppLocator`], replaces it with a new one whose resolver has the
/// services of Splat, and when dropped disposes the new one and restores the saved one.
pub struct InternalLocatorScope {
    saved_locator: Rc<InternalLocator>,
    pub locator: Rc<InternalLocator>,
}

impl InternalLocatorScope {
    pub fn new() -> Self {
        let saved_locator = AppLocator::internal_locator();
        let locator = InternalLocator::new();
        locator.current_mutable().initialize_splat();
        AppLocator::set_internal_locator(locator.clone());
        Self { saved_locator, locator }
    }
}

impl Drop for InternalLocatorScope {
    fn drop(&mut self) {
        self.locator.dispose();
        AppLocator::set_internal_locator(self.saved_locator.clone());
    }
}
