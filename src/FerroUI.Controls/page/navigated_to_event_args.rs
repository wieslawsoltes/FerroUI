use super::{NavigationType, Page};
use ferroui_base::{BoxedValue, Ref};

/// Provides data for the `NavigatedTo` event of a page.
#[derive(Clone, PartialEq)]
pub struct NavigatedToEventArgs {
    previous_page: Option<Ref<Page>>,
    navigation_type: NavigationType,
    parameter: Option<BoxedValue>,
}

impl NavigatedToEventArgs {
    /// Creates the args for the page that was active before and the type
    /// of the navigation.
    pub fn new(previous_page: Option<Ref<Page>>, navigation_type: NavigationType) -> Self {
        Self { previous_page, navigation_type, parameter: None }
    }

    /// Creates the args with the parameter that was passed to the
    /// navigation.
    pub fn with_parameter(
        previous_page: Option<Ref<Page>>,
        navigation_type: NavigationType,
        parameter: Option<BoxedValue>,
    ) -> Self {
        Self { previous_page, navigation_type, parameter }
    }

    /// The page that was active before the navigation.
    pub fn previous_page(&self) -> Option<Ref<Page>> {
        self.previous_page.clone()
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
