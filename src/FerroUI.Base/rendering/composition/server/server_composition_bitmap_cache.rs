use super::{impl_animated_server_object, IAnimatedServerObject, IServerObject, ServerCompositionVisual, ServerCompositor, ServerObject};
use crate::rendering::composition::generated::{
    ServerCompositionBitmapCacheHooks, ServerCompositionBitmapCacheProps, ServerCompositionCacheModeHooks,
};
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Server-side counterpart of `CompositionBitmapCache`.
pub struct ServerCompositionBitmapCache {
    object: ServerObject,
    props: ServerCompositionBitmapCacheProps,
    attached_visuals: RefCell<Vec<Weak<ServerCompositionVisual>>>,
}

impl ServerCompositionBitmapCache {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionBitmapCache> {
        Rc::new_cyclic(|this: &Weak<ServerCompositionBitmapCache>| {
            let owner: Weak<dyn IAnimatedServerObject> = this.clone();
            ServerCompositionBitmapCache {
                object: ServerObject::new(compositor, owner),
                props: ServerCompositionBitmapCacheProps::new(),
                attached_visuals: RefCell::new(Vec::new()),
            }
        })
    }

    pub fn render_at_scale(&self) -> f64 {
        self.props.render_at_scale()
    }

    pub fn snaps_to_device_pixels(&self) -> bool {
        self.props.snaps_to_device_pixels()
    }

    pub fn enable_clear_type(&self) -> bool {
        self.props.enable_clear_type()
    }

    // --- ServerCompositionCacheMode -----------------------------------------

    pub fn subscribe(&self, visual: &Rc<ServerCompositionVisual>) {
        let mut visuals = self.attached_visuals.borrow_mut();
        visuals.retain(|v| v.strong_count() > 0);
        if !visuals.iter().any(|v| std::ptr::eq(v.as_ptr(), Rc::as_ptr(visual))) {
            visuals.push(Rc::downgrade(visual));
        }
    }

    pub fn unsubscribe(&self, visual: &Rc<ServerCompositionVisual>) {
        self.attached_visuals.borrow_mut().retain(|v| !std::ptr::eq(v.as_ptr(), Rc::as_ptr(visual)));
    }
}

impl ServerCompositionCacheModeHooks for ServerCompositionBitmapCache {}
impl ServerCompositionBitmapCacheHooks for ServerCompositionBitmapCache {}

impl_animated_server_object!(
    ServerCompositionBitmapCache,
    object,
    composition_property: ServerCompositionBitmapCacheProps::get_composition_property
);

impl IServerObject for ServerCompositionBitmapCache {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        let alive: Vec<_> = self.attached_visuals.borrow().iter().filter_map(Weak::upgrade).collect();
        for visual in alive {
            visual.on_cache_mode_state_changed();
        }
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
