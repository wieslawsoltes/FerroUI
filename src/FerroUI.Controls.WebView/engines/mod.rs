//! The web engine of each platform.

use crate::engine::IWebViewEngineFactory;
use std::rc::Rc;

#[cfg(any(target_vendor = "apple", test))]
pub(crate) mod bridge;

#[cfg(target_vendor = "apple")]
mod apple;

/// The engine of this platform, or `None` where the crate has none.
pub fn default_factory() -> Option<Rc<dyn IWebViewEngineFactory>> {
    #[cfg(target_vendor = "apple")]
    {
        Some(Rc::new(apple::AppleEngineFactory))
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        None
    }
}
