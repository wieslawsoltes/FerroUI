//! Not from the reference: the routed event handlers a behavior adds to its associated object and
//! removes again.
//!
//! The reference removes a handler by the delegate it added (`RemoveHandler(event, handler)`); a
//! Rust closure has no identity to remove it by, so the token of each handler is kept and removed
//! with it.

use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent, RoutingStrategies};
use ferroui_base::reactive::{Disposable, IDisposable};
use std::cell::RefCell;
use std::rc::Rc;

/// The handlers a behavior added, removed together by [`clear`](Self::clear).
#[derive(Default)]
pub(crate) struct RoutedHandlers {
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl RoutedHandlers {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// `element.AddHandler(routedEvent, handler, routes, handledEventsToo)`.
    pub(crate) fn add<TEventArgs: IRoutedEventArgs + 'static>(
        &self,
        element: &Interactive,
        routed_event: &'static RoutedEvent<TEventArgs>,
        handler: impl Fn(&Interactive, &TEventArgs) + 'static,
        routes: RoutingStrategies,
        handled_events_too: bool,
    ) {
        let token = element.add_handler_with(routed_event, handler, routes, handled_events_too);
        let element = element.to_ref().downgrade();
        self.subscriptions.borrow_mut().push(Disposable::create(move || {
            if let Some(element) = element.upgrade() {
                element.remove_handler(routed_event, token);
            }
        }));
    }

    /// `element.RemoveHandler(..)` of every handler added.
    pub(crate) fn clear(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}
