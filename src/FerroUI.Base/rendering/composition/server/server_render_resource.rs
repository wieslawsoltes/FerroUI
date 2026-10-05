use super::{IAnimatedServerObject, ServerCompositor, ServerObject, ServerValueChange};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Something that is notified when a render resource it depends on is
/// invalidated.
pub trait IServerRenderResourceObserver {
    fn dependency_queued_invalidate(&self, sender: &dyn IServerRenderResource);
}

/// A server-side render resource: a brush, pen, geometry, transform or
/// render data whose changes invalidate the objects that use it.
pub trait IServerRenderResource: IServerRenderResourceObserver {
    fn add_observer(&self, observer: &Rc<dyn IServerRenderResourceObserver>);
    fn remove_observer(&self, observer: &Rc<dyn IServerRenderResourceObserver>);
    fn queued_invalidate(&self);
}

/// A render resource that owns a [`ServerRenderResourceCore`].
pub trait IServerRenderResourceHost: IServerRenderResource {
    /// The compositor of the resource, while it is alive.
    fn compositor(&self) -> Option<Rc<ServerCompositor>>;

    /// The resource as a shared handle, for the invalidation queue.
    fn as_render_resource_rc(&self) -> Option<Rc<dyn IServerRenderResource>>;

    /// Called once per group of property changes, before the queued
    /// invalidation.
    fn resource_property_changed(&self) {}
}

struct ObserverEntry {
    observer: Weak<dyn IServerRenderResourceObserver>,
    count: u32,
}

/// The shared implementation of render resources: change coalescing and
/// the reference-counted observer list.
///
/// Observers are held weakly; a dropped observer is pruned the next time
/// the list is walked.
#[derive(Default)]
pub struct ServerRenderResourceCore {
    pending_invalidation: Cell<bool>,
    disposed: Cell<bool>,
    observers: RefCell<Vec<ObserverEntry>>,
}

fn same_observer(a: &Weak<dyn IServerRenderResourceObserver>, b: &Rc<dyn IServerRenderResourceObserver>) -> bool {
    std::ptr::addr_eq(a.as_ptr(), Rc::as_ptr(b))
}

impl ServerRenderResourceCore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    /// Assigns a property field of the resource: moves the observation of
    /// the resource from the old value to the new one and records the
    /// change. Nothing happens when the value is unchanged.
    pub fn set_value(&self, host: &dyn IServerRenderResourceHost, change: ServerValueChange<'_>) {
        if change.equal {
            return;
        }

        if self.disposed.get() {
            (change.assign)();
            return;
        }

        let observer: Option<Rc<dyn IServerRenderResourceObserver>> =
            host.as_render_resource_rc().map(|this| this as Rc<dyn IServerRenderResourceObserver>);

        if let (Some(old_child), Some(observer)) = (&change.old_resource, &observer) {
            old_child.remove_observer(observer);
        }

        (change.assign)();

        if let (Some(new_child), Some(observer)) = (&change.new_resource, &observer) {
            new_child.add_observer(observer);
        }

        self.invalidated(host);
    }

    /// Assigns a property field that holds a list of values (the
    /// `IEnumerable` branch of `SetValue`): moves the observation of the
    /// resource from the render resources of the old list to those of the
    /// new one and records the change. The lists differ by reference, which
    /// is the caller's equality test.
    pub fn set_list_value(
        &self,
        host: &dyn IServerRenderResourceHost,
        old_children: Vec<Rc<dyn IServerRenderResource>>,
        new_children: Vec<Rc<dyn IServerRenderResource>>,
        assign: &mut dyn FnMut(),
    ) {
        if self.disposed.get() {
            assign();
            return;
        }

        let observer: Option<Rc<dyn IServerRenderResourceObserver>> =
            host.as_render_resource_rc().map(|this| this as Rc<dyn IServerRenderResourceObserver>);

        if let Some(observer) = &observer {
            for child in &old_children {
                child.remove_observer(observer);
            }
        }

        assign();

        if let Some(observer) = &observer {
            for child in &new_children {
                child.add_observer(observer);
            }
        }

        self.invalidated(host);
    }

    /// Stops observing the render resource behind a property value.
    pub fn remove_observers_from_property(
        &self,
        host: &dyn IServerRenderResourceHost,
        resource: Option<&Rc<dyn IServerRenderResource>>,
    ) {
        if let (Some(resource), Some(this)) = (resource, host.as_render_resource_rc()) {
            let observer: Rc<dyn IServerRenderResourceObserver> = this;
            resource.remove_observer(&observer);
        }
    }

    /// Records that a property of the resource changed. The first change
    /// since the last queued invalidation enqueues the resource on its
    /// compositor.
    pub fn invalidated(&self, host: &dyn IServerRenderResourceHost) {
        // This is needed to avoid triggering on multiple property changes.
        if !self.pending_invalidation.get() {
            self.pending_invalidation.set(true);
            if let (Some(compositor), Some(this)) = (host.compositor(), host.as_render_resource_rc()) {
                compositor.enqueue_render_resource_for_invalidation(this);
            }
            host.resource_property_changed();
        }
    }

    pub fn dispose(&self) {
        self.disposed.set(true);
        self.observers.borrow_mut().clear();
    }

    pub fn dependency_queued_invalidate(&self, host: &dyn IServerRenderResourceHost) {
        if let (Some(compositor), Some(this)) = (host.compositor(), host.as_render_resource_rc()) {
            compositor.enqueue_render_resource_for_invalidation(this);
        }
    }

    pub fn add_observer(&self, observer: &Rc<dyn IServerRenderResourceObserver>) {
        debug_assert!(!self.disposed.get());
        if self.disposed.get() {
            return;
        }
        let mut observers = self.observers.borrow_mut();
        match observers.iter_mut().find(|e| same_observer(&e.observer, observer)) {
            Some(entry) => entry.count += 1,
            None => observers.push(ObserverEntry { observer: Rc::downgrade(observer), count: 1 }),
        }
    }

    pub fn remove_observer(&self, observer: &Rc<dyn IServerRenderResourceObserver>) {
        if self.disposed.get() {
            return;
        }
        let mut observers = self.observers.borrow_mut();
        if let Some(index) = observers.iter().position(|e| same_observer(&e.observer, observer)) {
            observers[index].count -= 1;
            if observers[index].count == 0 {
                observers.swap_remove(index);
            }
        }
    }

    pub fn queued_invalidate(&self, this: &dyn IServerRenderResource) {
        self.pending_invalidation.set(false);
        let observers: Vec<Rc<dyn IServerRenderResourceObserver>> = {
            let mut observers = self.observers.borrow_mut();
            observers.retain(|e| e.observer.strong_count() > 0);
            observers.iter().filter_map(|e| e.observer.upgrade()).collect()
        };
        for observer in observers {
            observer.dependency_queued_invalidate(this);
        }
    }

    /// The number of distinct live observers.
    pub fn observer_count(&self) -> usize {
        self.observers.borrow().iter().filter(|e| e.observer.strong_count() > 0).count()
    }
}

/// The base of the server-side render resources that are not animatable
/// (upstream `SimpleServerRenderResource`), as a part the resource classes
/// embed: the compositor, the resource core and the handle of the object as
/// a render resource.
pub struct SimpleServerRenderResource {
    compositor: Weak<ServerCompositor>,
    this: Weak<dyn IServerRenderResource>,
    core: ServerRenderResourceCore,
}

impl SimpleServerRenderResource {
    /// `this` is the object that embeds the part.
    pub fn new(compositor: &Rc<ServerCompositor>, this: Weak<dyn IServerRenderResource>) -> Self {
        Self { compositor: Rc::downgrade(compositor), this, core: ServerRenderResourceCore::new() }
    }

    pub fn compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.upgrade()
    }

    pub fn core(&self) -> &ServerRenderResourceCore {
        &self.core
    }

    pub fn is_disposed(&self) -> bool {
        self.core.is_disposed()
    }

    pub fn as_render_resource_rc(&self) -> Option<Rc<dyn IServerRenderResource>> {
        self.this.upgrade()
    }
}

/// Implements the render resource contracts of a class that embeds a
/// [`SimpleServerRenderResource`] in the field `$base`: the members
/// `SimpleServerRenderResource` gives its subclasses upstream.
///
/// `SetValue` goes through the resource core, so a property that holds a
/// render resource is observed and every change invalidates the resource.
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_simple_server_render_resource {
    ($ty:ty, $base:ident) => {
        impl $crate::rendering::composition::server::IServerRenderResourceObserver for $ty {
            fn dependency_queued_invalidate(
                &self,
                _sender: &dyn $crate::rendering::composition::server::IServerRenderResource,
            ) {
                self.$base.core().dependency_queued_invalidate(self);
            }
        }

        impl $crate::rendering::composition::server::IServerRenderResource for $ty {
            fn add_observer(
                &self,
                observer: &::std::rc::Rc<dyn $crate::rendering::composition::server::IServerRenderResourceObserver>,
            ) {
                self.$base.core().add_observer(observer);
            }

            fn remove_observer(
                &self,
                observer: &::std::rc::Rc<dyn $crate::rendering::composition::server::IServerRenderResourceObserver>,
            ) {
                self.$base.core().remove_observer(observer);
            }

            fn queued_invalidate(&self) {
                self.$base.core().queued_invalidate(self);
            }
        }

        impl $crate::rendering::composition::server::IServerRenderResourceHost for $ty {
            fn compositor(&self) -> Option<::std::rc::Rc<$crate::rendering::composition::server::ServerCompositor>> {
                self.$base.compositor()
            }

            fn as_render_resource_rc(
                &self,
            ) -> Option<::std::rc::Rc<dyn $crate::rendering::composition::server::IServerRenderResource>> {
                self.$base.as_render_resource_rc()
            }
        }

        impl $crate::rendering::composition::server::IServerPropertyHost for $ty {
            fn server_compositor(
                &self,
            ) -> Option<::std::rc::Rc<$crate::rendering::composition::server::ServerCompositor>> {
                self.$base.compositor()
            }

            fn set_value(
                &self,
                _property: &'static $crate::rendering::composition::server::CompositionProperty,
                change: $crate::rendering::composition::server::ServerValueChange<'_>,
            ) {
                self.$base.core().set_value(self, change);
            }
        }
    };
}

pub use crate::__impl_simple_server_render_resource as impl_simple_server_render_resource;

/// The base of the animatable server-side render resources (upstream
/// `ServerRenderResource`, a `ServerObject`), as a part the resource classes
/// embed: the animation support of the server object, the resource core and
/// the handle of the object as a render resource.
pub struct ServerRenderResource {
    object: ServerObject,
    this: Weak<dyn IServerRenderResource>,
    core: ServerRenderResourceCore,
}

impl ServerRenderResource {
    /// `owner` and `this` are the object that embeds the part.
    pub fn new(
        compositor: &Rc<ServerCompositor>,
        owner: Weak<dyn IAnimatedServerObject>,
        this: Weak<dyn IServerRenderResource>,
    ) -> Self {
        Self { object: ServerObject::new(compositor, owner), this, core: ServerRenderResourceCore::new() }
    }

    /// The embedded `ServerObject`.
    pub fn object(&self) -> &ServerObject {
        &self.object
    }

    pub fn compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.object.compositor()
    }

    pub fn core(&self) -> &ServerRenderResourceCore {
        &self.core
    }

    pub fn is_disposed(&self) -> bool {
        self.core.is_disposed()
    }

    pub fn as_render_resource_rc(&self) -> Option<Rc<dyn IServerRenderResource>> {
        self.this.upgrade()
    }
}

/// Implements the render resource and animation contracts of a class that
/// embeds a [`ServerRenderResource`] in the field `$base`: the members
/// `ServerRenderResource` gives its subclasses upstream.
///
/// As upstream, `SetValue` goes through the resource core only: the
/// `SetValue` of the render resource hides the one of `ServerObject`, so a
/// direct set does not invalidate the animations that depend on the
/// property. `$get` resolves a composition property by name.
///
/// The class implements `IServerObject` itself, with `values_invalidated`
/// invalidating the resource (`self.$base.core().invalidated(self)`).
#[macro_export]
#[doc(hidden)]
macro_rules! __impl_server_render_resource {
    ($ty:ty, $base:ident, composition_property: $get:expr) => {
        $crate::rendering::composition::server::impl_server_render_resource!(@resource $ty, $base);

        impl $crate::rendering::composition::server::IServerAnimatedPropertyHost for $ty {
            fn set_animated_value(
                &self,
                property: &'static $crate::rendering::composition::server::CompositionProperty,
                current_value: $crate::rendering::composition::expressions::ExpressionVariant,
                committed_at: ::std::time::Duration,
                animation: ::std::rc::Rc<dyn $crate::rendering::composition::animations::IAnimationInstance>,
            ) {
                self.$base.object().set_animated_value(property, current_value, committed_at, animation);
            }

            fn remove_animation_for_property(
                &self,
                property: &'static $crate::rendering::composition::server::CompositionProperty,
            ) {
                self.$base.object().remove_animation_for_property(property);
            }

            fn notify_animated_value_changed(
                &self,
                _property: &'static $crate::rendering::composition::server::CompositionProperty,
            ) {
                $crate::rendering::composition::server::IServerObject::values_invalidated(self);
            }
        }

        impl $crate::rendering::composition::server::IAnimatedServerObject for $ty {
            fn server_object(&self) -> &$crate::rendering::composition::server::ServerObject {
                self.$base.object()
            }

            fn get_composition_property(
                &self,
                field_name: &str,
            ) -> Option<&'static $crate::rendering::composition::server::CompositionProperty> {
                ($get)(field_name)
            }

            fn as_server_object_dyn(&self) -> &dyn $crate::rendering::composition::server::IServerObject {
                self
            }
        }
    };
    (@resource $ty:ty, $base:ident) => {
        impl $crate::rendering::composition::server::IServerRenderResourceObserver for $ty {
            fn dependency_queued_invalidate(
                &self,
                _sender: &dyn $crate::rendering::composition::server::IServerRenderResource,
            ) {
                self.$base.core().dependency_queued_invalidate(self);
            }
        }

        impl $crate::rendering::composition::server::IServerRenderResource for $ty {
            fn add_observer(
                &self,
                observer: &::std::rc::Rc<dyn $crate::rendering::composition::server::IServerRenderResourceObserver>,
            ) {
                self.$base.core().add_observer(observer);
            }

            fn remove_observer(
                &self,
                observer: &::std::rc::Rc<dyn $crate::rendering::composition::server::IServerRenderResourceObserver>,
            ) {
                self.$base.core().remove_observer(observer);
            }

            fn queued_invalidate(&self) {
                self.$base.core().queued_invalidate(self);
            }
        }

        impl $crate::rendering::composition::server::IServerRenderResourceHost for $ty {
            fn compositor(&self) -> Option<::std::rc::Rc<$crate::rendering::composition::server::ServerCompositor>> {
                self.$base.compositor()
            }

            fn as_render_resource_rc(
                &self,
            ) -> Option<::std::rc::Rc<dyn $crate::rendering::composition::server::IServerRenderResource>> {
                self.$base.as_render_resource_rc()
            }
        }

        impl $crate::rendering::composition::server::IServerPropertyHost for $ty {
            fn server_compositor(
                &self,
            ) -> Option<::std::rc::Rc<$crate::rendering::composition::server::ServerCompositor>> {
                self.$base.compositor()
            }

            fn set_value(
                &self,
                _property: &'static $crate::rendering::composition::server::CompositionProperty,
                change: $crate::rendering::composition::server::ServerValueChange<'_>,
            ) {
                self.$base.core().set_value(self, change);
            }
        }
    };
}

pub use crate::__impl_server_render_resource as impl_server_render_resource;
