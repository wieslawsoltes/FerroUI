use super::{IStyleActivator, IStyleActivatorSink};
use std::cell::{Cell, RefCell};
use std::rc::Weak;

/// The state shared by all style activators.
#[derive(Default)]
pub(crate) struct StyleActivatorBase {
    sink: RefCell<Option<Weak<dyn IStyleActivatorSink>>>,
    value: Cell<Option<bool>>,
}

impl StyleActivatorBase {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn is_subscribed(&self) -> bool {
        self.sink.borrow().is_some()
    }
}

/// Base class for implementations of [`IStyleActivator`].
pub(crate) trait StyleActivator: 'static {
    fn base(&self) -> &StyleActivatorBase;

    /// Evaluates the activation state.
    ///
    /// This method should read directly from its inputs and not rely on any
    /// subscriptions to fire in order to be up-to-date.
    fn evaluate_is_active(&self) -> bool;

    /// Called in response to a subscription to allow the activator to
    /// subscribe to its inputs.
    fn initialize(&self);

    /// Called in response to an unsubscription to allow the activator to
    /// unsubscribe from its inputs.
    fn deinitialize(&self);

    /// Evaluates the activation state, recording it as the last known value
    /// if there is none yet.
    fn evaluate_and_record(&self) -> bool {
        let value = self.evaluate_is_active();
        let base = self.base();
        if base.value.get().is_none() {
            base.value.set(Some(value));
        }
        value
    }

    /// Called from a derived class when the activation state should be
    /// re-evaluated and the subscriber notified of any change. Returns the
    /// evaluated active state.
    fn reevaluate_is_active(&self) -> bool {
        let value = self.evaluate_and_record();
        let base = self.base();
        if Some(value) != base.value.get() {
            base.value.set(Some(value));
            let sink = base.sink.borrow().clone();
            if let Some(sink) = sink.and_then(|s| s.upgrade()) {
                sink.on_next(value);
            }
        }
        value
    }
}

impl<T: StyleActivator> IStyleActivator for T {
    #[inline]
    fn is_subscribed(&self) -> bool {
        self.base().is_subscribed()
    }

    fn get_is_active(&self) -> bool {
        self.evaluate_and_record()
    }

    fn subscribe(&self, sink: Weak<dyn IStyleActivatorSink>) {
        if self.base().is_subscribed() {
            panic!("StyleActivator is already subscribed.");
        }
        self.initialize();
        *self.base().sink.borrow_mut() = Some(sink);
    }

    fn unsubscribe(&self, sink: &Weak<dyn IStyleActivatorSink>) {
        {
            let current = self.base().sink.borrow();
            let Some(current) = &*current else { return };
            if !std::ptr::addr_eq(current.as_ptr(), sink.as_ptr()) {
                panic!("StyleActivatorSink is not subscribed.");
            }
        }
        *self.base().sink.borrow_mut() = None;
        self.deinitialize();
    }

    fn dispose(&self) {
        *self.base().sink.borrow_mut() = None;
        self.deinitialize();
    }
}
