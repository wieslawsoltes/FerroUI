use crate::Control;
use ferroui_base::Ref;

/// Provides data for the `ItemsControl::container_clearing` event.
///
/// The args are immutable: copies compare equal when they describe the same
/// container (and indexes).
#[derive(Clone, PartialEq)]
pub struct ContainerClearingEventArgs {
    container: Ref<Control>,
}

impl ContainerClearingEventArgs {
    /// Initializes a new instance of the [`ContainerClearingEventArgs`] class.
    pub fn new(container: Ref<Control>) -> Self {
        Self { container }
    }

    /// Gets the container being cleared.
    pub fn container(&self) -> &Ref<Control> {
        &self.container
    }
}
