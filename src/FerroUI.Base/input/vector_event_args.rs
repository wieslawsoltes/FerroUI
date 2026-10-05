use crate::interactivity::RoutedEventArgs;
use crate::{ferro_routed_event_args, Vector};

/// Provides a vector with a routed event.
#[derive(Clone, Default)]
pub struct VectorEventArgs {
    base: RoutedEventArgs,

    /// The vector of the event.
    pub vector: Vector,
}

ferro_routed_event_args!(VectorEventArgs: RoutedEventArgs);

impl VectorEventArgs {
    /// Creates args with a zero vector and no routed event.
    pub fn new() -> Self {
        Self::default()
    }
}
