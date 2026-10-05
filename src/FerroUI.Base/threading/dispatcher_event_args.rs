use std::sync::Arc;

use super::Dispatcher;

/// Base contract for all event arguments associated with a [`Dispatcher`].
pub trait DispatcherEventArgs {
    /// The dispatcher associated with this event.
    fn dispatcher(&self) -> &Arc<Dispatcher>;
}
