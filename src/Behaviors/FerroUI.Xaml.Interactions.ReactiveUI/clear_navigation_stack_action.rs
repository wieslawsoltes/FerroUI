//! Port of `ClearNavigationStackAction.cs`.

use crate::view_model_parameter::result;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_reactiveui::RoutingState;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that resets the navigation stack.
#[repr(C)]
pub struct ClearNavigationStackAction {
    base: StyledElementAction,
}

ferro_class!(ClearNavigationStackAction: StyledElementAction);
ferro_class_info!(ClearNavigationStackAction { new: ClearNavigationStackAction::new });
ferroui_base::ferro_impl_classes!(ClearNavigationStackAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ClearNavigationStackAction {
        /// Identifies the `Router` property.
        pub fn router_property() -> StyledProperty<Option<Rc<RoutingState>>> {
            FerroProperty::register::<ClearNavigationStackAction, _>("Router", None)
        }
    }
}

impl StyledElementActionImpl for ClearNavigationStackAction {
    /// Executes the action.
    ///
    /// `sender` is the sender that triggered the action, `parameter` an optional parameter for
    /// the action. Returns `true` if navigation was requested.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return result(false);
        }

        let Some(router) = this.router() else {
            return result(false);
        };

        router.navigation_stack().clear();
        result(true)
    }
}

impl ClearNavigationStackAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates the action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the router used for navigation. This is a property of the object model.
    pub fn router(&self) -> Option<Rc<RoutingState>> {
        self.get_value(Self::router_property())
    }

    pub fn set_router(&self, value: Option<Rc<RoutingState>>) {
        self.set_value(Self::router_property(), value)
    }
}
