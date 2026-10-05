use crate::media::BitmapCache;
use crate::utilities::FormatError;
use crate::rendering::composition::{CompositionCacheMode, Compositor};
use crate::{ferro_class, ferro_impl_classes, FerroObjectImpl, Ref, StyledElement, StyledElementImpl};
use std::rc::Rc;

/// Represents cached content modes for graphics acceleration features.
#[repr(C)]
pub struct CacheMode {
    base: StyledElement,
}

ferro_class! {
    CacheMode: StyledElement, virtuals CacheModeImpl: StyledElementImpl {
        /// The composition cache mode that mirrors this cache mode on
        /// compositor `c`.
        // We currently only allow visual to be attached to one compositor at a time, so keep it simple for now
        fn get_for_compositor(this, c: &Rc<Compositor>) -> Rc<CompositionCacheMode>;
    }
}
ferro_impl_classes!(CacheMode: FerroObjectImpl, StyledElementImpl);

impl CacheModeImpl for CacheMode {
    fn get_for_compositor(_this: &Self, _c: &Rc<Compositor>) -> Rc<CompositionCacheMode> {
        panic!("the cache mode class is abstract")
    }
}

impl CacheMode {
    /// Creates the class data; see [`crate::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElement::construct() }
    }

    /// Parses a cache mode: the only known mode is `BitmapCache`.
    pub fn parse(s: &str) -> Result<Ref<CacheMode>, FormatError> {
        if s == "BitmapCache" {
            return Ok(BitmapCache::new().upcast());
        }
        Err(FormatError::from_string(format!("Unknown CacheMode: {s}")))
    }
}
