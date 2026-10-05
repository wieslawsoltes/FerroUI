use super::{NavigationType, Page};
use ferroui_base::{BoxedValue, Ref};
use std::cell::Cell;
use std::rc::Rc;

/// Provides data for the `Navigating` event of a page.
///
/// The cancel flag is shared: a clone is another handle to the same args.
#[derive(Clone)]
pub struct NavigatingFromEventArgs {
    destination_page: Option<Ref<Page>>,
    navigation_type: NavigationType,
    cancel: Rc<Cell<bool>>,
    parameter: Option<BoxedValue>,
}

impl PartialEq for NavigatingFromEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cancel, &other.cancel)
    }
}

impl NavigatingFromEventArgs {
    /// Creates the args for the page that will become active and the type
    /// of the navigation.
    pub fn new(destination_page: Option<Ref<Page>>, navigation_type: NavigationType) -> Self {
        Self::with_parameter(destination_page, navigation_type, None)
    }

    /// Creates the args with the parameter that was passed to the
    /// navigation.
    pub fn with_parameter(
        destination_page: Option<Ref<Page>>,
        navigation_type: NavigationType,
        parameter: Option<BoxedValue>,
    ) -> Self {
        Self { destination_page, navigation_type, cancel: Rc::new(Cell::new(false)), parameter }
    }

    /// The page that will become active after the navigation.
    pub fn destination_page(&self) -> Option<Ref<Page>> {
        self.destination_page.clone()
    }

    /// The type of navigation that triggered the event.
    pub fn navigation_type(&self) -> NavigationType {
        self.navigation_type
    }

    /// Whether the navigation should be cancelled.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value)
    }

    /// The parameter passed to the navigation.
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }
}
