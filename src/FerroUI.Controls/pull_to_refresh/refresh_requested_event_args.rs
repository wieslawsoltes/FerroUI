use super::RefreshCompletionDeferral;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use std::rc::Rc;

/// Provides event data for RefreshRequested events.
#[derive(Clone)]
pub struct RefreshRequestedEventArgs {
    base: RoutedEventArgs,
    refresh_completion_deferral: Rc<RefreshCompletionDeferral>,
}

ferro_routed_event_args!(RefreshRequestedEventArgs: RoutedEventArgs);

impl RefreshRequestedEventArgs {
    /// Creates args whose deferral runs `deferred_action` once completed.
    pub fn new<T: ?Sized>(deferred_action: impl Fn() + 'static, routed_event: Option<&RoutedEvent<T>>) -> Self {
        Self::with_deferral(RefreshCompletionDeferral::new(deferred_action), routed_event)
    }

    /// Creates args that share an existing deferral.
    pub fn with_deferral<T: ?Sized>(
        completion_deferral: Rc<RefreshCompletionDeferral>,
        routed_event: Option<&RoutedEvent<T>>,
    ) -> Self {
        let base = match routed_event {
            Some(routed_event) => RoutedEventArgs::with_event(routed_event),
            None => RoutedEventArgs::new(),
        };
        Self { base, refresh_completion_deferral: completion_deferral }
    }

    /// Gets a deferral object for managing the work done in the
    /// RefreshRequested event handler.
    pub fn get_deferral(&self) -> Rc<RefreshCompletionDeferral> {
        self.refresh_completion_deferral.get()
    }

    pub(crate) fn increment_count(&self) {
        drop(self.refresh_completion_deferral.get());
    }

    pub(crate) fn decrement_count(&self) {
        self.refresh_completion_deferral.complete();
    }
}
