use super::{IAnimatedServerObject, ServerCompositor, ServerObject, ServerValueChange};
use smallvec::SmallVec;
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
        // The observers are copied out, since one may change the list while it is told; a
        // resource has one or two, so the copy stays off the heap (FerroUI addition,
        // desktop-performance.md, section 9.31).
        let observers: SmallVec<[Rc<dyn IServerRenderResourceObserver>; 4]> = {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A render resource with nothing but the core.
    #[derive(Default)]
    struct Resource {
        core: ServerRenderResourceCore,
    }

    impl IServerRenderResourceObserver for Resource {
        fn dependency_queued_invalidate(&self, _sender: &dyn IServerRenderResource) {}
    }

    impl IServerRenderResource for Resource {
        fn add_observer(&self, observer: &Rc<dyn IServerRenderResourceObserver>) {
            self.core.add_observer(observer);
        }

        fn remove_observer(&self, observer: &Rc<dyn IServerRenderResourceObserver>) {
            self.core.remove_observer(observer);
        }

        fn queued_invalidate(&self) {
            self.core.queued_invalidate(self);
        }
    }

    type Action = Box<dyn Fn(&dyn IServerRenderResource)>;

    /// Counts how often it is told, and may act on the sender while it is.
    #[derive(Default)]
    struct Observer {
        told: Cell<u32>,
        action: RefCell<Option<Action>>,
    }

    impl IServerRenderResourceObserver for Observer {
        fn dependency_queued_invalidate(&self, sender: &dyn IServerRenderResource) {
            self.told.set(self.told.get() + 1);
            if let Some(action) = &*self.action.borrow() {
                action(sender);
            }
        }
    }

    fn observer() -> (Rc<Observer>, Rc<dyn IServerRenderResourceObserver>) {
        let observer = Rc::new(Observer::default());
        (observer.clone(), observer)
    }

    fn told(observers: &[(Rc<Observer>, Rc<dyn IServerRenderResourceObserver>)]) -> Vec<u32> {
        observers.iter().map(|(observer, _)| observer.told.get()).collect()
    }

    /// The copy of the observers stays inline up to four of them and goes to
    /// the heap above (section 9.31): the count does not change who is told.
    #[test]
    fn every_observer_is_told_once_however_many_there_are() {
        for count in [0, 1, 2, 4, 5, 9] {
            let resource = Resource::default();
            let observers: Vec<_> = (0..count).map(|_| observer()).collect();
            for (_, handle) in &observers {
                resource.add_observer(handle);
            }
            assert_eq!(resource.core.observer_count(), count);

            resource.queued_invalidate();
            assert_eq!(told(&observers), vec![1; count]);
            resource.queued_invalidate();
            assert_eq!(told(&observers), vec![2; count]);
        }
    }

    #[test]
    fn an_observer_added_twice_is_told_once_and_stays_until_it_is_removed_twice() {
        let resource = Resource::default();
        let (first, handle) = observer();
        resource.add_observer(&handle);
        resource.add_observer(&handle);
        assert_eq!(resource.core.observer_count(), 1);

        resource.queued_invalidate();
        assert_eq!(first.told.get(), 1);

        resource.remove_observer(&handle);
        resource.queued_invalidate();
        assert_eq!(first.told.get(), 2);

        resource.remove_observer(&handle);
        assert_eq!(resource.core.observer_count(), 0);
        resource.queued_invalidate();
        assert_eq!(first.told.get(), 2);
    }

    /// The observers of the moment the invalidation starts are told, whatever
    /// one of them does to the list meanwhile.
    #[test]
    fn an_observer_may_remove_itself_and_others_while_it_is_told() {
        let resource = Resource::default();
        let observers: Vec<_> = (0..6).map(|_| observer()).collect();
        for (_, handle) in &observers {
            resource.add_observer(handle);
        }
        // The first removes itself and the last two.
        let removed: Vec<Weak<dyn IServerRenderResourceObserver>> =
            [0, 4, 5].iter().map(|index| Rc::downgrade(&observers[*index].1)).collect();
        *observers[0].0.action.borrow_mut() = Some(Box::new(move |sender| {
            for observer in removed.iter().filter_map(Weak::upgrade) {
                sender.remove_observer(&observer);
            }
        }));

        resource.queued_invalidate();
        assert_eq!(told(&observers), [1, 1, 1, 1, 1, 1]);
        assert_eq!(resource.core.observer_count(), 3);

        resource.queued_invalidate();
        assert_eq!(told(&observers), [1, 2, 2, 2, 1, 1]);
    }

    #[test]
    fn an_observer_added_while_the_others_are_told_is_told_the_next_time() {
        let resource = Resource::default();
        let (first, first_handle) = observer();
        let (late, late_handle) = observer();
        resource.add_observer(&first_handle);
        let added = Cell::new(false);
        *first.action.borrow_mut() = Some(Box::new(move |sender| {
            if !added.replace(true) {
                sender.add_observer(&late_handle);
            }
        }));

        resource.queued_invalidate();
        assert_eq!((first.told.get(), late.told.get()), (1, 0));
        assert_eq!(resource.core.observer_count(), 2);

        resource.queued_invalidate();
        assert_eq!((first.told.get(), late.told.get()), (2, 1));
    }

    #[test]
    fn a_dropped_observer_is_pruned_and_a_disposed_resource_tells_nobody() {
        let resource = Resource::default();
        let (kept, kept_handle) = observer();
        let (dropped, dropped_handle) = observer();
        resource.add_observer(&dropped_handle);
        resource.add_observer(&kept_handle);
        let dropped_told = Rc::downgrade(&dropped);
        drop((dropped, dropped_handle));
        assert!(dropped_told.upgrade().is_none());
        assert_eq!(resource.core.observer_count(), 1);

        resource.queued_invalidate();
        assert_eq!(kept.told.get(), 1);
        assert_eq!(resource.core.observers.borrow().len(), 1);

        resource.core.dispose();
        resource.queued_invalidate();
        assert_eq!(kept.told.get(), 1);
        assert_eq!(resource.core.observer_count(), 0);
    }
}
