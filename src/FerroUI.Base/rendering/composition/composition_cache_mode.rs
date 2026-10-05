//! `CompositionCacheMode` and `CompositionBitmapCache`.
//!
//! The only cache mode is the bitmap cache, so the abstract class is an
//! alias of it.

use super::animations::{ICompositionAnimation, ICompositionAnimationBase, ImplicitAnimationCollection};
use super::expressions::ExpressionVariant;
use super::generated::{
    CompositionBitmapCacheHooks, CompositionBitmapCacheProps, CompositionCacheModeHooks,
    ServerCompositionBitmapCacheProps,
};
use super::server::{CompositionProperty, ServerCompositionBitmapCache, ServerObjectId};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::{
    AsCompositionObject, CompositionObject, Compositor, ICompositionObject, ICompositionObjectAnimations,
    ICompositionObjectHost, ICompositorSerializable, PendingAnimations,
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

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The collection of implicit animations attached to this object.
    pub fn implicit_animations(&self) -> Option<Rc<ImplicitAnimationCollection>> {
        self.object.implicit_animations()
    }

    pub fn set_implicit_animations(&self, value: Option<Rc<ImplicitAnimationCollection>>) {
        self.object.set_implicit_animations(value)
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

    fn implicit_animation(&self, property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        self.object.implicit_animation(property_name)
    }

    fn start_animation_group(
        &self,
        grp: &Rc<dyn ICompositionAnimationBase>,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool {
        self.start_animation_group_for(&**grp, target, final_value)
    }
}

impl ICompositionObjectAnimations for CompositionBitmapCache {
    fn try_start_animation(
        &self,
        property_name: &str,
        animation: &dyn ICompositionAnimation,
        final_value: Option<ExpressionVariant>,
    ) -> bool {
        self.props.start_animation(self, property_name, animation, final_value)
    }

    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty> {
        ServerCompositionBitmapCacheProps::get_composition_property(property_name)
    }

    fn composition_object(&self) -> &CompositionObject {
        &self.object
    }
}

impl AsCompositionObject for CompositionBitmapCache {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "CompositionBitmapCache"
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
