use super::{NavigationType, Page};
use ferroui_base::{BoxedValue, Ref};

/// Provides data for navigation events such as `Pushed`, `Popped` and
/// `PoppedToRoot`.
#[derive(Clone, PartialEq)]
pub struct NavigationEventArgs {
    page: Ref<Page>,
    navigation_type: NavigationType,
    parameter: Option<BoxedValue>,
}

impl NavigationEventArgs {
    /// Creates the args for the page involved in the navigation and the
    /// type of the navigation.
    pub fn new(page: Ref<Page>, navigation_type: NavigationType) -> Self {
        Self { page, navigation_type, parameter: None }
    }

    /// Creates the args with the parameter that was passed to the
    /// navigation.
    pub fn with_parameter(page: Ref<Page>, navigation_type: NavigationType, parameter: Option<BoxedValue>) -> Self {
        Self { page, navigation_type, parameter }
    }

    /// The page involved in the navigation.
    pub fn page(&self) -> Ref<Page> {
        self.page.clone()
    }

    /// The type of navigation.
    pub fn navigation_type(&self) -> NavigationType {
        self.navigation_type
    }

    /// The parameter passed to the navigation.
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }
}
