//! Port of `NavigateAction.cs`.

use crate::view_model_parameter::{result, routable_view_model_of};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{RoutableViewModel, RoutingState};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that navigates to a specified routable view model
/// ([`IRoutableViewModel`](ferroui_reactiveui::IRoutableViewModel)).
#[repr(C)]
pub struct NavigateAction {
    base: StyledElementAction,
}

ferro_class!(NavigateAction: StyledElementAction);
ferro_class_info!(NavigateAction { new: NavigateAction::new });
ferroui_base::ferro_impl_classes!(NavigateAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl NavigateAction {
        /// Identifies the `Router` property.
        pub fn router_property() -> StyledProperty<Option<Rc<RoutingState>>> {
            FerroProperty::register::<NavigateAction, _>("Router", None)
        }

        /// Identifies the `ViewModel` property.
        pub fn view_model_property() -> StyledProperty<Option<RoutableViewModel>> {
            FerroProperty::register::<NavigateAction, _>("ViewModel", None)
        }
    }
}

impl StyledElementActionImpl for NavigateAction {
    /// Executes the action.
    ///
    /// `sender` is the sender that triggered the action, `parameter` an optional parameter for
    /// the action: the view model to navigate to when [`view_model`](NavigateAction::view_model)
    /// is not set. Returns `true` if navigation was requested.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return result(false);
        }

        let Some(router) = this.router() else {
            return result(false);
        };

        let Some(vm) = this.view_model().or_else(|| routable_view_model_of(parameter)) else {
            return result(false);
        };

        let _ = router.navigate().execute(Some(vm)).subscribe_ignore();
        result(true)
    }
}

impl NavigateAction {
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

    /// Gets or sets the view model to navigate to. This is a property of the object model.
    pub fn view_model(&self) -> Option<RoutableViewModel> {
        self.get_value(Self::view_model_property())
    }

    pub fn set_view_model(&self, value: Option<RoutableViewModel>) {
        self.set_value(Self::view_model_property(), value)
    }
}
