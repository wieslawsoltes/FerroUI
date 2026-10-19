//! `InternalLocator`: the current resolver and the notifications of its changes.

use super::app_locator::AppLocator;
use super::dependency_resolver::{
    IDependencyResolver, IMutableDependencyResolver, IReadonlyDependencyResolver,
};
use super::instance_generic_first::InstanceGenericFirstDependencyResolver;
use crate::disposables::ActionDisposable;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A resolver that replaces the current one for a scope (`ResolverOverride`).
struct ResolverOverride {
    resolver: RefCell<Rc<dyn IDependencyResolver>>,
    notifications_suppressed: bool,
}

/// The holder of the current resolver (`InternalLocator`): the process-wide resolver, the
/// resolver of a [`with_resolver`](Self::with_resolver) scope that overrides it, and the callbacks
/// that run when it changes.
// It is public for the tests of the crate only, as upstream's is internal and visible to its tests.
// Deviation (DEVIATIONS.md, Splat): upstream's override flows with the asynchronous context
// (`AsyncLocal`); here it is the override of the thread, which is the flow of single-threaded code.
#[doc(hidden)]
pub struct InternalLocator {
    resolver_changed: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    resolver_changed_notification: RefCell<Option<Rc<dyn IDisposable>>>,
    flow_override: RefCell<Option<Rc<ResolverOverride>>>,
    resolver_changed_notification_suspend_count: Cell<usize>,
    process_wide: RefCell<Rc<dyn IDependencyResolver>>,
    disposed: Cell<bool>,
}

impl InternalLocator {
    /// A locator with an empty [`InstanceGenericFirstDependencyResolver`], whose changes run the
    /// resolver initializer of [`AppLocator`] on the new resolver; the initializer runs at once.
    pub fn new() -> Rc<Self> {
        let locator = Rc::new(Self {
            resolver_changed: Rc::new(RefCell::new(Vec::with_capacity(4))),
            resolver_changed_notification: RefCell::new(None),
            flow_override: RefCell::new(None),
            resolver_changed_notification_suspend_count: Cell::new(0),
            process_wide: RefCell::new(Rc::new(InstanceGenericFirstDependencyResolver::new())),
            disposed: Cell::new(false),
        });
        let weak: Weak<Self> = Rc::downgrade(&locator);
        let notification = locator.register_resolver_callback_changed(Rc::new(move || {
            if let Some(locator) = weak.upgrade() {
                AppLocator::re_init(&*locator.current_mutable());
            }
        }));
        *locator.resolver_changed_notification.borrow_mut() = Some(notification);
        locator
    }

    /// `Current`.
    pub fn current(&self) -> Rc<dyn IReadonlyDependencyResolver> {
        self.internal()
    }

    /// `CurrentMutable`.
    pub fn current_mutable(&self) -> Rc<dyn IMutableDependencyResolver> {
        self.internal()
    }

    /// `Internal`: the resolver of the scope that overrides the process-wide one, or that one.
    pub fn internal(&self) -> Rc<dyn IDependencyResolver> {
        if let Some(flow_override) = self.flow_override.borrow().as_ref() {
            return flow_override.resolver.borrow().clone();
        }
        self.process_wide.borrow().clone()
    }

    /// The setter of `Internal`: replaces the process-wide resolver without notifications.
    pub fn set_internal(&self, resolver: Rc<dyn IDependencyResolver>) {
        *self.process_wide.borrow_mut() = resolver;
    }

    /// `SetLocator`: replaces the resolver of the current scope, or the process-wide one, and
    /// notifies.
    pub fn set_locator(&self, resolver: Rc<dyn IDependencyResolver>) {
        let flow_override = self.flow_override.borrow().clone();
        match flow_override {
            None => *self.process_wide.borrow_mut() = resolver,
            Some(flow_override) => *flow_override.resolver.borrow_mut() = resolver,
        }
        self.notify_resolver_changed();
    }

    /// `WithResolver`: makes `resolver` the current resolver until the returned disposable is
    /// disposed.
    pub fn with_resolver(
        self: &Rc<Self>,
        resolver: Rc<dyn IDependencyResolver>,
        suppress_resolver_callback: bool,
    ) -> Rc<dyn IDisposable> {
        let previous = self.flow_override.borrow().clone();
        let notifications_suppressed = suppress_resolver_callback
            || previous.as_ref().is_some_and(|previous| previous.notifications_suppressed);

        *self.flow_override.borrow_mut() = Some(Rc::new(ResolverOverride {
            resolver: RefCell::new(resolver),
            notifications_suppressed,
        }));

        if !suppress_resolver_callback {
            self.notify_resolver_changed();
        }

        let locator = self.clone();
        ActionDisposable::new(move || {
            *locator.flow_override.borrow_mut() = previous;
            if suppress_resolver_callback {
                return;
            }
            locator.notify_resolver_changed();
        })
    }

    /// `RegisterResolverCallbackChanged`: adds a callback for the changes of the resolver, and
    /// runs it at once while notifications are enabled.
    pub fn register_resolver_callback_changed(&self, callback: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.resolver_changed.borrow_mut().push(callback.clone());

        // We always invoke the callback at once, to set up the current resolver with whatever we
        // have.
        if self.are_resolver_callback_changed_notifications_enabled() {
            callback();
        }

        let callbacks = Rc::downgrade(&self.resolver_changed);
        ActionDisposable::new(move || {
            if let Some(callbacks) = callbacks.upgrade() {
                let mut callbacks = callbacks.borrow_mut();
                if let Some(index) = callbacks
                    .iter()
                    .position(|c| std::ptr::addr_eq(Rc::as_ptr(c), Rc::as_ptr(&callback)))
                {
                    callbacks.remove(index);
                }
            }
        })
    }

    /// `SuppressResolverCallbackChangedNotifications`: suppresses the notifications until the
    /// returned disposable is disposed; suppressions nest.
    pub fn suppress_resolver_callback_changed_notifications(self: &Rc<Self>) -> Rc<dyn IDisposable> {
        let count = &self.resolver_changed_notification_suspend_count;
        count.set(count.get() + 1);
        let locator = self.clone();
        ActionDisposable::new(move || {
            let count = &locator.resolver_changed_notification_suspend_count;
            count.set(count.get().saturating_sub(1));
        })
    }

    /// `AreResolverCallbackChangedNotificationsEnabled`.
    pub fn are_resolver_callback_changed_notifications_enabled(&self) -> bool {
        self.resolver_changed_notification_suspend_count.get() == 0
            && !self
                .flow_override
                .borrow()
                .as_ref()
                .is_some_and(|flow_override| flow_override.notifications_suppressed)
    }

    /// Whether the locator was disposed.
    pub fn is_disposed(&self) -> bool {
        self.disposed.get()
    }

    fn notify_resolver_changed(&self) {
        if !self.are_resolver_callback_changed_notifications_enabled() {
            return;
        }
        // A copy, so that a callback may register or set the resolver again.
        let callbacks: Vec<Rc<dyn Fn()>> = self.resolver_changed.borrow().clone();
        for callback in callbacks {
            callback();
        }
    }
}

impl IDisposable for InternalLocator {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let process_wide = self.process_wide.borrow().clone();
        process_wide.dispose();
        let notification = self.resolver_changed_notification.borrow_mut().take();
        if let Some(notification) = notification {
            notification.dispose();
        }
    }
}
