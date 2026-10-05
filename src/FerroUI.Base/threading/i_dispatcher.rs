use super::DispatcherPriority;

/// Dispatches jobs to a thread.
pub trait IDispatcher {
    /// Determines whether the calling thread is the thread associated with
    /// this dispatcher.
    fn check_access(&self) -> bool;

    /// Panics when the calling thread is not the thread associated with this
    /// dispatcher.
    fn verify_access(&self);

    /// Posts an action that will be invoked on the dispatcher thread.
    ///
    /// The action can be posted from any thread and therefore has to be
    /// `Send`.
    fn post(&self, action: Box<dyn FnOnce() + Send>, priority: DispatcherPriority);
}
