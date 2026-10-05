use super::PipsPager;
use ferroui_base::ferro_routed_event_args;
use ferroui_base::interactivity::RoutedEventArgs;

/// Provides data for the `PipsPager::selected_index_changed` event.
#[derive(Clone, Default)]
pub struct PipsPagerSelectedIndexChangedEventArgs {
    base: RoutedEventArgs,
    old_index: i32,
    new_index: i32,
}

ferro_routed_event_args!(PipsPagerSelectedIndexChangedEventArgs: RoutedEventArgs);

impl PipsPagerSelectedIndexChangedEventArgs {
    /// Creates the args of a change of the selected index from `old_index`
    /// to `new_index`.
    pub fn new(old_index: i32, new_index: i32) -> Self {
        Self { base: RoutedEventArgs::with_event(PipsPager::selected_index_changed_event()), old_index, new_index }
    }

    /// Gets the previous selected index.
    pub fn old_index(&self) -> i32 {
        self.old_index
    }

    /// Gets the new selected index.
    pub fn new_index(&self) -> i32 {
        self.new_index
    }
}
