//! Port of `NavigateToAndResetAction.cs` (`NavigateToAndResetAction<TViewModel>`).

use crate::view_model_parameter::result;
use crate::RoutableViewModelType;
use ferroui_base::{
    ferro_class, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    StyledProperty,
};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{IRoutableViewModel, RoutableViewModel, RoutingState};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that resolves and navigates to a view model of a type (the `TViewModel` of the
/// reference, a [`RoutableViewModelType`]) and clears the navigation stack.
///
/// Deviation (DEVIATIONS.md, Behaviors): the class is not generic; the view model type is given
/// when the action is created ([`new`](Self::new)), and markup does not create the action (it
/// has no type arguments for it).
#[repr(C)]
pub struct NavigateToAndResetAction {
    base: StyledElementAction,
    view_model_type: RoutableViewModelType,
}

ferro_class! {
    NavigateToAndResetAction: StyledElementAction, virtuals NavigateToAndResetActionImpl: StyledElementActionImpl {
        /// Resolves an instance of the view model type from the service locator.
        ///
        /// Returns the resolved view model instance or `None` if it cannot be created.
        fn resolve_view_model(this) -> Option<RoutableViewModel>;
    }
}

ferroui_base::ferro_impl_classes!(NavigateToAndResetAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl NavigateToAndResetAction {
        /// Identifies the `Router` property.
        pub fn router_property() -> StyledProperty<Option<Rc<RoutingState>>> {
            FerroProperty::register::<NavigateToAndResetAction, _>("Router", None)
        }
    }
}

impl NavigateToAndResetActionImpl for NavigateToAndResetAction {
    fn resolve_view_model(this: &Self) -> Option<RoutableViewModel> {
        this.view_model_type.resolve()
    }
}

impl StyledElementActionImpl for NavigateToAndResetAction {
    fn execute(this: &Self, _sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return result(false);
        }

        let Some(router) = this.router() else {
            return result(false);
        };

        let vm = parameter
            .and_then(|parameter| this.view_model_type.cast(parameter))
            .or_else(|| this.resolve_view_model());
        let Some(vm) = vm else {
            return result(false);
        };

        let _ = router.navigate_and_reset().execute(Some(vm)).subscribe_ignore();
        result(true)
    }
}

impl NavigateToAndResetAction {
    /// Creates the class data for an action that navigates to view models of `view_model_type`
    /// (the `TViewModel` of the reference); see [`ferroui_base::FerroObject::construct`].
    pub fn construct(view_model_type: RoutableViewModelType) -> Self {
        Self { base: StyledElementAction::construct(), view_model_type }
    }

    /// Creates an action that navigates to view models of type `VM`.
    pub fn new<VM: IRoutableViewModel + 'static>() -> Ref<Self> {
        instantiate(Self::construct(RoutableViewModelType::of::<VM>()))
    }

    /// The view model type the action navigates to (the `TViewModel` of the reference).
    pub fn view_model_type(&self) -> RoutableViewModelType {
        self.view_model_type
    }

    /// Gets or sets the router used for navigation. This is a property of the object model.
    pub fn router(&self) -> Option<Rc<RoutingState>> {
        self.get_value(Self::router_property())
    }

    pub fn set_router(&self, value: Option<Rc<RoutingState>>) {
        self.set_value(Self::router_property(), value)
    }
}
