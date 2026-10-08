use super::{
    IResourceHost, IResourceProvider, ResourceHostRef, ResourceKey, ResourceValue, ResourcesChangedEventArgs,
    WeakResourceHost,
};
use crate::reactive::{Disposable, IDisposable, IObservable, IObserver};
use crate::styling::ThemeVariant;
use crate::{BoxedValue, FerroProperty};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A function applied to a resource value before it is published by a
/// resource observable.
pub type ResourceConverter = Rc<dyn Fn(Option<BoxedValue>) -> Option<BoxedValue>>;

impl dyn IResourceHost + '_ {
    /// Finds the specified resource by searching up the logical tree and then
    /// global styles.
    ///
    /// Returns the resource, or the unset value marker if not found.
    pub fn find_resource(&self, key: &ResourceKey) -> ResourceValue {
        match self.try_find_resource(key, None) {
            Some(value) => value,
            None => Some(FerroProperty::unset_value()),
        }
    }

    /// Finds the specified resource for a theme variant by searching up the
    /// logical tree and then global styles.
    ///
    /// Returns the resource, or the unset value marker if not found.
    pub fn find_resource_for_theme(&self, theme: Option<&ThemeVariant>, key: &ResourceKey) -> ResourceValue {
        match self.try_find_resource(key, theme) {
            Some(value) => value,
            None => Some(FerroProperty::unset_value()),
        }
    }

    /// Tries to find the specified resource by searching up the logical tree
    /// and then global styles.
    pub fn try_find_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        crate::perf_count!(ResourceLookups);
        crate::perf_count!(ResourceHostsProbed);
        if let Some(value) = self.try_get_resource(key, theme) {
            return Some(value);
        }

        let mut current = self.as_style_host().and_then(|h| h.styling_parent());
        while let Some(host) = current {
            crate::perf_count!(ResourceHostsProbed);
            if let Some(value) = host.try_get_resource(key, theme) {
                return Some(value);
            }
            current = host.styling_parent();
        }

        None
    }
}

impl ResourceHostRef {
    /// Gets an observable for the resource with the specified key.
    ///
    /// The observable produces the current value on subscription and a new
    /// value whenever the resources visible to the host, or its theme
    /// variant, change. It holds the host weakly; while it has observers the
    /// host keeps it alive.
    pub fn resource_observable(
        &self,
        key: impl Into<ResourceKey>,
        converter: Option<ResourceConverter>,
    ) -> Rc<dyn IObservable<Option<BoxedValue>>> {
        ResourceObservable::create(Target::Host(self.downgrade()), key.into(), None, converter)
    }
}

/// Gets an observable for the resource with the specified key as seen by the
/// owner of a resource provider.
///
/// The observable produces a value whenever the provider has an owner and the
/// resources visible to that owner, the owner itself or (unless
/// `default_theme_variant` is given) its theme variant change.
///
/// While it has observers the observable and the provider keep each other
/// alive; dispose the subscriptions to release them.
pub fn get_floating_resource_observable(
    resource_provider: Rc<dyn IResourceProvider>,
    key: impl Into<ResourceKey>,
    default_theme_variant: Option<ThemeVariant>,
    converter: Option<ResourceConverter>,
) -> Rc<dyn IObservable<Option<BoxedValue>>> {
    ResourceObservable::create(Target::Provider(resource_provider), key.into(), default_theme_variant, converter)
}

type ResourceObserver = Rc<dyn IObserver<Option<BoxedValue>>>;

enum Target {
    Host(WeakResourceHost),
    Provider(Rc<dyn IResourceProvider>),
}

struct ResourceObservable {
    this: Weak<ResourceObservable>,
    target: Target,
    key: ResourceKey,
    override_theme_variant: Option<ThemeVariant>,
    converter: Option<ResourceConverter>,
    observers: RefCell<Vec<(u64, ResourceObserver)>>,
    next_id: Cell<u64>,
    owner_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    host_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl ResourceObservable {
    fn create(
        target: Target,
        key: ResourceKey,
        override_theme_variant: Option<ThemeVariant>,
        converter: Option<ResourceConverter>,
    ) -> Rc<dyn IObservable<Option<BoxedValue>>> {
        Rc::new_cyclic(|this| ResourceObservable {
            this: this.clone(),
            target,
            key,
            override_theme_variant,
            converter,
            observers: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
            owner_changed_subscription: RefCell::new(None),
            host_subscriptions: RefCell::new(Vec::new()),
        })
    }

    fn host(&self) -> Option<ResourceHostRef> {
        match &self.target {
            Target::Host(host) => host.upgrade(),
            Target::Provider(provider) => provider.owner(),
        }
    }

    fn has_host(&self) -> bool {
        match &self.target {
            Target::Host(_) => true,
            Target::Provider(provider) => provider.owner().is_some(),
        }
    }

    fn initialize(&self) {
        if let Target::Provider(provider) = &self.target {
            // The subscriptions keep the observable alive, so that it keeps
            // publishing while it has observers even if nothing else holds
            // it. They are released when the last observer unsubscribes.
            let Some(this) = self.this.upgrade() else { return };
            let subscription = provider.owner_changed(Rc::new(move || {
                this.unsubscribe_host();
                this.subscribe_host();
                this.publish_next();
            }));
            *self.owner_changed_subscription.borrow_mut() = Some(subscription);
        }
        self.subscribe_host();
    }

    fn deinitialize(&self) {
        let subscription = self.owner_changed_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.unsubscribe_host();
    }

    fn subscribe_host(&self) {
        let Some(host) = self.host() else { return };
        let mut subscriptions = Vec::with_capacity(2);

        let Some(this) = self.this.upgrade() else { return };
        let observable = this.clone();
        subscriptions.push(host.resources_changed(Rc::new(move |_: &ResourcesChangedEventArgs| {
            observable.publish_next();
        })));

        if self.override_theme_variant.is_none() {
            if let Some(theme_host) = host.as_theme_variant_host() {
                subscriptions.push(theme_host.actual_theme_variant_changed(Rc::new(move || {
                    this.publish_next();
                })));
            }
        }

        *self.host_subscriptions.borrow_mut() = subscriptions;
    }

    fn unsubscribe_host(&self) {
        let subscriptions = std::mem::take(&mut *self.host_subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }

    fn publish_next(&self) {
        if !self.has_host() {
            return;
        }
        let value = self.get_value();
        let observers: Vec<_> = self.observers.borrow().iter().map(|(_, o)| o.clone()).collect();
        for observer in observers {
            observer.on_next(value.clone());
        }
    }

    fn get_value(&self) -> Option<BoxedValue> {
        let value = match self.host() {
            Some(host) => {
                let theme = self
                    .override_theme_variant
                    .clone()
                    .or_else(|| host.as_theme_variant_host().and_then(|h| h.actual_theme_variant()));
                host.find_resource_for_theme(theme.as_ref(), &self.key)
            }
            None => Some(FerroProperty::unset_value()),
        };
        // A null resource is published as the unset value, as is a missing one.
        let value = value.or_else(|| Some(FerroProperty::unset_value()));
        match &self.converter {
            Some(converter) => converter(value.clone()).or(value),
            None => value,
        }
    }
}

impl IObservable<Option<BoxedValue>> for ResourceObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<Option<BoxedValue>>>) -> Rc<dyn IDisposable> {
        let first = self.observers.borrow().is_empty();
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.observers.borrow_mut().push((id, observer.clone()));

        if first {
            self.initialize();
        }

        if self.has_host() {
            observer.on_next(self.get_value());
        }

        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                let empty = {
                    let mut observers = this.observers.borrow_mut();
                    observers.retain(|(i, _)| *i != id);
                    observers.is_empty()
                };
                if empty {
                    this.deinitialize();
                }
            }
        })
    }
}
