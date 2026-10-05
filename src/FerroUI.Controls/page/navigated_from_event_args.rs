use super::{NavigationType, Page};
use ferroui_base::{BoxedValue, Ref};

/// Provides data for the `NavigatedFrom` event of a page.
#[derive(Clone, PartialEq)]
pub struct NavigatedFromEventArgs {
    destination_page: Option<Ref<Page>>,
    navigation_type: NavigationType,
    parameter: Option<BoxedValue>,
}

impl NavigatedFromEventArgs {
    /// Creates the args for the page that became active and the type of
    /// the navigation.
    pub fn new(destination_page: Option<Ref<Page>>, navigation_type: NavigationType) -> Self {
        Self { destination_page, navigation_type, parameter: None }
    }

    /// Creates the args with the parameter that was passed to the
    /// navigation.
    pub fn with_parameter(
        destination_page: Option<Ref<Page>>,
        navigation_type: NavigationType,
        parameter: Option<BoxedValue>,
    ) -> Self {
        Self { destination_page, navigation_type, parameter }
    }

    /// The page that became active after the navigation.
    pub fn destination_page(&self) -> Option<Ref<Page>> {
        self.destination_page.clone()
    }

    /// The type of navigation that triggered the event.
    pub fn navigation_type(&self) -> NavigationType {
        self.navigation_type
    }

    /// The parameter passed to the navigation.
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }
}
