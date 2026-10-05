//! Access to the platform render interface.

use crate::platform::IPlatformRenderInterface;
use crate::{FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// The platform render interface registered by the rendering backend.
///
/// Panics if no rendering backend has been registered.
pub fn render_interface() -> Rc<dyn IPlatformRenderInterface> {
    FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>()
}
