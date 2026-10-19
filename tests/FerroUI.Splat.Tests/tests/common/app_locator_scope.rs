//! `AppLocatorScope`.

use ferroui_splat::{
    AppLocator, IDependencyResolver, InstanceGenericFirstDependencyResolver,
    ServiceLocationInitializationExt,
};
use std::rc::Rc;

/// Saves the resolver of [`AppLocator`], makes a new [`InstanceGenericFirstDependencyResolver`]
/// with the services of Splat the current one, and restores the saved resolver when dropped.
pub struct AppLocatorScope {
    saved_resolver: Rc<dyn IDependencyResolver>,
}

impl AppLocatorScope {
    pub fn new() -> Self {
        let saved_resolver = AppLocator::get_locator();
        let new_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());
        new_resolver.initialize_splat();
        AppLocator::set_locator(new_resolver);
        Self { saved_resolver }
    }
}

impl Drop for AppLocatorScope {
    fn drop(&mut self) {
        AppLocator::set_locator(self.saved_resolver.clone());
    }
}
