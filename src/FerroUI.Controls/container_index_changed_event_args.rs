use crate::Control;
use ferroui_base::Ref;

/// Provides data for the `ItemsControl::container_index_changed` event.
///
/// The args are immutable: copies compare equal when they describe the same
/// container (and indexes).
#[derive(Clone, PartialEq)]
pub struct ContainerIndexChangedEventArgs {
    container: Ref<Control>,
    old_index: i32,
    new_index: i32,
}

impl ContainerIndexChangedEventArgs {
    /// Initializes a new instance of the [`ContainerIndexChangedEventArgs`] class.
    pub fn new(container: Ref<Control>, old_index: i32, new_index: i32) -> Self {
        Self { container, old_index, new_index }
    }

    /// Get the container for which the index changed.
    pub fn container(&self) -> &Ref<Control> {
        &self.container
    }

    /// Gets the index of the container after the change.
    pub fn new_index(&self) -> i32 {
        self.new_index
    }

    /// Gets the index of the container before the change.
    pub fn old_index(&self) -> i32 {
        self.old_index
    }
}
