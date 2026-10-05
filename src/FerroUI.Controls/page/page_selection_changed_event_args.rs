use super::Page;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{ferro_routed_event_args, Ref};

/// Provides data for a page selection-changed event.
#[derive(Clone)]
pub struct PageSelectionChangedEventArgs {
    base: RoutedEventArgs,
    previous_page: Option<Ref<Page>>,
    current_page: Option<Ref<Page>>,
}

ferro_routed_event_args!(PageSelectionChangedEventArgs: RoutedEventArgs);

impl PartialEq for PageSelectionChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base
    }
}

impl PageSelectionChangedEventArgs {
    /// Initializes a new instance of the [`PageSelectionChangedEventArgs`]
    /// class.
    ///
    /// `routed_event` is the routed event associated with this event args
    /// instance, `previous_page` the page that was selected before the
    /// change (`None` if no page was selected) and `current_page` the page
    /// that is now selected (`None` if the selection was cleared).
    pub fn new<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        previous_page: Option<Ref<Page>>,
        current_page: Option<Ref<Page>>,
    ) -> Self {
        let base = match routed_event {
            Some(routed_event) => RoutedEventArgs::with_event(routed_event),
            None => RoutedEventArgs::new(),
        };
        Self { base, previous_page, current_page }
    }

    /// Gets the page that was selected before the change.
    pub fn previous_page(&self) -> Option<Ref<Page>> {
        self.previous_page.clone()
    }

    /// Gets the page that is now selected.
    pub fn current_page(&self) -> Option<Ref<Page>> {
        self.current_page.clone()
    }
}
