use crate::media::{CacheMode, CacheModeImpl};
use crate::rendering::composition::{CompositionBitmapCache, CompositionCacheMode, Compositor};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Creates and caches a bitmap representation of a visual.
#[repr(C)]
pub struct BitmapCache {
    base: CacheMode,
    current: RefCell<Option<Rc<CompositionBitmapCache>>>,
}

ferro_class!(BitmapCache: CacheMode);
crate::ferro_class_info!(BitmapCache { new: BitmapCache::new });
ferro_impl_classes!(BitmapCache: StyledElementImpl);

impl FerroObjectImpl for BitmapCache {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        let current = this.current.borrow().clone();
        if let (true, Some(current)) = (change.is_effective_value_change(), current) {
            if change.property() == Self::render_at_scale_property().as_property() {
                current.set_render_at_scale(this.render_at_scale());
            } else if change.property() == Self::snaps_to_device_pixels_property().as_property() {
                current.set_snaps_to_device_pixels(this.snaps_to_device_pixels());
            } else if change.property() == Self::enable_clear_type_property().as_property() {
                current.set_enable_clear_type(this.enable_clear_type());
            }
        }

        Self::parent_on_property_changed(this, change);
    }
}

impl CacheModeImpl for BitmapCache {
    // We currently only allow visual to be attached to one compositor at a time, so keep it simple for now
    fn get_for_compositor(this: &Self, c: &Rc<Compositor>) -> Rc<CompositionCacheMode> {
        let current = this.current.borrow().clone();
        match current {
            Some(current) if Rc::ptr_eq(current.compositor(), c) => current,
            _ => {
                let current = CompositionBitmapCache::new(c);
                current.set_enable_clear_type(this.enable_clear_type());
                current.set_render_at_scale(this.render_at_scale());
                current.set_snaps_to_device_pixels(this.snaps_to_device_pixels());
                *this.current.borrow_mut() = Some(current.clone());
                current
            }
        }
    }
}

crate::ferro_properties! { impl BitmapCache {
    ferro_property!(
        /// Defines the `RenderAtScale` property.
        pub fn render_at_scale_property() -> StyledProperty<f64> {
            FerroProperty::register::<BitmapCache, _>("RenderAtScale", 1.0)
        }
    );

    ferro_property!(
        /// Defines the `SnapsToDevicePixels` property.
        pub fn snaps_to_device_pixels_property() -> StyledProperty<bool> {
            FerroProperty::register::<BitmapCache, _>("SnapsToDevicePixels", false)
        }
    );

    ferro_property!(
        /// Defines the `EnableClearType` property.
        pub fn enable_clear_type_property() -> StyledProperty<bool> {
            FerroProperty::register::<BitmapCache, _>("EnableClearType", false)
        }
    );
} }

impl BitmapCache {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: CacheMode::construct(), current: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The scale that is applied to the bitmap.
    pub fn render_at_scale(&self) -> f64 {
        self.get_value(Self::render_at_scale_property())
    }

    pub fn set_render_at_scale(&self, value: f64) {
        self.set_value(Self::render_at_scale_property(), value)
    }

    /// Whether the bitmap is rendered with pixel snapping.
    pub fn snaps_to_device_pixels(&self) -> bool {
        self.get_value(Self::snaps_to_device_pixels_property())
    }

    pub fn set_snaps_to_device_pixels(&self, value: bool) {
        self.set_value(Self::snaps_to_device_pixels_property(), value)
    }

    /// Whether the bitmap is rendered with ClearType activated.
    pub fn enable_clear_type(&self) -> bool {
        self.get_value(Self::enable_clear_type_property())
    }

    pub fn set_enable_clear_type(&self, value: bool) {
        self.set_value(Self::enable_clear_type_property(), value)
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn defaults_and_parse() {
        let cache = BitmapCache::new();
        assert_eq!(1.0, cache.render_at_scale());
        assert!(!cache.snaps_to_device_pixels());
        assert!(!cache.enable_clear_type());
        cache.set_render_at_scale(2.0);
        assert_eq!(2.0, cache.render_at_scale());

        assert!(CacheMode::parse("BitmapCache").unwrap().is::<BitmapCache>());
        assert!(CacheMode::parse("bitmapcache").is_err());
        assert!(CacheMode::parse("").is_err());
    }
}
