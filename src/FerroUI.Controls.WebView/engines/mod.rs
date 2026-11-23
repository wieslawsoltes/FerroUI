//! The web engine of each platform.

use crate::engine::IWebViewEngineFactory;
use std::rc::Rc;

/// The engine of this platform, or `None` where the crate has none.
pub fn default_factory() -> Option<Rc<dyn IWebViewEngineFactory>> {
    None
}
