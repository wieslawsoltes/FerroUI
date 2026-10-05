use super::IStyleActivatorSink;
use std::rc::Weak;

/// Defines a style activator.
///
/// A style activator is very similar to an observable of booleans, but is
/// optimized for the particular use-case of activating a style according to a
/// selector. Instead of producing an initial value on subscription, the
/// current value is read with [`get_is_active`](Self::get_is_active), and
/// changes are reported to a single sink.
///
/// The activator holds its sink weakly: the sink (a style instance or a
/// composite activator) owns the activator.
pub trait IStyleActivator {
    /// Whether the activator is subscribed.
    fn is_subscribed(&self) -> bool;

    /// Gets the current activation state.
    ///
    /// This method reads directly from its inputs and does not rely on any
    /// subscriptions to fire in order to be up-to-date.
    fn get_is_active(&self) -> bool;

    /// Subscribes to the activator. Panics if it is already subscribed.
    fn subscribe(&self, sink: Weak<dyn IStyleActivatorSink>);

    /// Unsubscribes from the activator.
    fn unsubscribe(&self, sink: &Weak<dyn IStyleActivatorSink>);

    /// Releases the activator's subscriptions.
    fn dispose(&self);
}
