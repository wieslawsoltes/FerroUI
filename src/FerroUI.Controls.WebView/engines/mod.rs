//! The web engine of each platform.

use crate::engine::IWebViewEngineFactory;
use std::rc::Rc;

#[cfg(any(target_vendor = "apple", windows, target_os = "android", test))]
pub(crate) mod bridge;

#[cfg(target_vendor = "apple")]
mod apple;
#[cfg(windows)]
mod windows;
#[cfg(target_os = "android")]
mod android;

/// The engine of this platform, or `None` where the crate has none.
pub fn default_factory() -> Option<Rc<dyn IWebViewEngineFactory>> {
    #[cfg(target_vendor = "apple")]
    {
        Some(Rc::new(apple::AppleEngineFactory))
    }
    #[cfg(windows)]
    {
        Some(Rc::new(windows::WindowsEngineFactory))
    }
    #[cfg(target_os = "android")]
    {
        Some(Rc::new(android::AndroidEngineFactory))
    }
    #[cfg(not(any(target_vendor = "apple", windows, target_os = "android")))]
    {
        None
    }
}
