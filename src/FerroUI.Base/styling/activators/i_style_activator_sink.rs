/// Receives notifications from an [`IStyleActivator`](super::IStyleActivator).
pub trait IStyleActivatorSink {
    /// Called when the subscribed activator value changes.
    fn on_next(&self, value: bool);
}
