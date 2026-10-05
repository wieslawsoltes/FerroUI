use crate::Control;
use ferroui_base::Ref;

/// Provides data for the `ItemsControl::container_prepared` event.
///
/// The args are immutable: copies compare equal when they describe the same
/// container (and indexes).
#[derive(Clone, PartialEq)]
pub struct ContainerPreparedEventArgs {
    container: Ref<Control>,
    index: i32,
}

impl ContainerPreparedEventArgs {
    /// Initializes a new instance of the [`ContainerPreparedEventArgs`] class.
    pub fn new(container: Ref<Control>, index: i32) -> Self {
        Self { container, index }
    }

    /// Gets the prepared container.
    pub fn container(&self) -> &Ref<Control> {
        &self.container
    }

    /// Gets the index of the item the container was prepared for.
    pub fn index(&self) -> i32 {
        self.index
    }
}
