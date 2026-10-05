//! `CompositionCacheMode` and `CompositionBitmapCache`.
//!
//! The only cache mode is the bitmap cache, so the abstract class is an
//! alias of it.

use super::animations::ICompositionAnimationBase;
use super::expressions::ExpressionVariant;
use super::generated::{CompositionBitmapCacheHooks, CompositionBitmapCacheProps, CompositionCacheModeHooks};
use super::server::{ServerCompositionBitmapCache, ServerObjectId};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::{
    CompositionObject, Compositor, ICompositionObject, ICompositionObjectHost, ICompositorSerializable,
    PendingAnimations,
};
use std::any::Any;
use std::rc::{Rc, Weak};

/// The cache mode of a composition visual.
pub type CompositionCacheMode = CompositionBitmapCache;

/// Caches the subtree of a visual in a bitmap.
pub struct CompositionBitmapCache {
    this: Weak<CompositionBitmapCache>,
    object: CompositionObject,
    props: CompositionBitmapCacheProps,
}

impl CompositionBitmapCache {
    pub fn new(compositor: &Rc<Compositor>) -> Rc<CompositionBitmapCache> {
        let server = compositor.create_server_object(|compositor, _| ServerCompositionBitmapCache::new(compositor));
        let cache = Rc::new_cyclic(|this: &Weak<CompositionBitmapCache>| CompositionBitmapCache {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            props: CompositionBitmapCacheProps::new(),
        });
        cache.props.initialize_defaults(&*cache);
        cache
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    pub fn render_at_scale(&self) -> f64 {
        self.props.render_at_scale()
    }

    pub fn set_render_at_scale(&self, value: f64) {
        self.props.set_render_at_scale(self, value)
    }

    pub fn snaps_to_device_pixels(&self) -> bool {
        self.props.snaps_to_device_pixels()
    }

    pub fn set_snaps_to_device_pixels(&self, value: bool) {
        self.props.set_snaps_to_device_pixels(self, value)
    }

    pub fn enable_clear_type(&self) -> bool {
        self.props.enable_clear_type()
    }

    pub fn set_enable_clear_type(&self, value: bool) {
        self.props.set_enable_clear_type(self, value)
    }
}

impl CompositionCacheModeHooks for CompositionBitmapCache {}
impl CompositionBitmapCacheHooks for CompositionBitmapCache {}

impl IRegisterForSerialization for CompositionBitmapCache {
    fn register_for_serialization(&self) {
        self.object
            .register_for_serialization(|| self.this.upgrade().map(|this| this as Rc<dyn ICompositorSerializable>));
    }
}

impl ICompositionObjectHost for CompositionBitmapCache {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn pending_animations(&self) -> &PendingAnimations {
        self.object.pending_animations()
    }

    fn implicit_animation(&self, _property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        None
    }

    fn start_animation_group(
        &self,
        _grp: &Rc<dyn ICompositionAnimationBase>,
        _target: &str,
        _final_value: ExpressionVariant,
    ) -> bool {
        false
    }
}

impl ICompositionObject for CompositionBitmapCache {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl ICompositorSerializable for CompositionBitmapCache {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.object.try_get_server(c)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.object.begin_serialize_changes(c);
        self.props.serialize_changes_core(self, writer);
    }
}
