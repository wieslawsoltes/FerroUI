//! Port of `NavigateBackAction.cs`.

use crate::view_model_parameter::result;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::RoutingState;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that navigates back in the [`RoutingState`] stack.
#[repr(C)]
pub struct NavigateBackAction {
    base: StyledElementAction,
}

ferro_class!(NavigateBackAction: StyledElementAction);
ferro_class_info!(NavigateBackAction { new: NavigateBackAction::new });
ferroui_base::ferro_impl_classes!(NavigateBackAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl NavigateBackAction {
        /// Identifies the `Router` property.
        pub fn router_property() -> StyledProperty<Option<Rc<RoutingState>>> {
            FerroProperty::register::<NavigateBackAction, _>("Router", None)
        }
    }
}

impl StyledElementActionImpl for NavigateBackAction {
    /// Executes the action.
    ///
    /// `sender` is the sender that triggered the action, `parameter` an optional parameter for
    /// the action. Returns `true` if navigation back was requested.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return result(false);
        }

        let Some(router) = this.router() else {
            return result(false);
        };

        if router.navigation_stack().count() == 0 {
            return result(false);
        }

        let _ = router.navigate_back().execute_default().subscribe_ignore();
        result(true)
    }
}

impl NavigateBackAction {
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
