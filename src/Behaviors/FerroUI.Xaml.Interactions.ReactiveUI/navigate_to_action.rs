//! Port of `NavigateToAction.cs` (`NavigateToAction<TViewModel>`).

use crate::view_model_parameter::{result, routable_view_model_of};
use ferroui_base::{
    ferro_class, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    StyledProperty,
};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{IRoutableViewModel, RoutableViewModel, RoutingState};
use ferroui_splat::{Locator, ReadonlyDependencyResolverExt};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// The view model type an action resolves and navigates to (the `TViewModel` of
/// [`NavigateToAction`] and [`NavigateToAndResetAction`](crate::NavigateToAndResetAction)).
///
/// Deviation (DEVIATIONS.md, Behaviors): the classes of the object model are not generic, so the
/// type argument of the reference is this value, made with [`of`](Self::of) for a Rust type that
/// implements [`IRoutableViewModel`]. It casts a parameter to the type (`parameter as
/// TViewModel`) and resolves an instance of the type from the service locator
/// (`Locator.Current.GetService<TViewModel>()`).
#[derive(Clone, Copy)]
pub struct RoutableViewModelType {
    name: &'static str,
    cast: fn(&BoxedValue) -> Option<RoutableViewModel>,
    resolve: fn() -> Option<RoutableViewModel>,
}

impl RoutableViewModelType {
    /// The view model type `VM`.
    pub fn of<VM: IRoutableViewModel + 'static>() -> Self {
        Self { name: std::any::type_name::<VM>(), cast: cast_view_model::<VM>, resolve: resolve_view_model::<VM> }
    }

    /// The Rust name of the type.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The value as a view model of the type (`value as TViewModel`): a shared view model of the
    /// type (`Rc<VM>`, the object itself, or a routable view model whose object is of the type).
    /// `None` for anything else.
    pub fn cast(&self, value: &BoxedValue) -> Option<RoutableViewModel> {
        (self.cast)(value)
    }

    /// An instance of the type from the service locator, or `None` if it cannot be created
    /// (`Locator.Current.GetService<TViewModel>()`).
    pub fn resolve(&self) -> Option<RoutableViewModel> {
        (self.resolve)()
    }
}

impl fmt::Debug for RoutableViewModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RoutableViewModelType").field(&self.name).finish()
    }
}

fn cast_view_model<VM: IRoutableViewModel + 'static>(value: &BoxedValue) -> Option<RoutableViewModel> {
    if let Some(view_model) = value.downcast_ref::<Rc<VM>>() {
        return Some(view_model.clone() as RoutableViewModel);
    }
    if let Some(view_model) = value.downcast_ref::<Option<Rc<VM>>>() {
        return view_model.clone().map(|view_model| view_model as RoutableViewModel);
    }
    if value.is::<VM>() {
        // A view model of a reference type, boxed as the object itself.
        let any: Rc<dyn Any> = value.clone();
        return any.downcast::<VM>().ok().map(|view_model| view_model as RoutableViewModel);
    }
    let view_model = routable_view_model_of(Some(value))?;
    let any: Rc<dyn Any> = view_model.clone();
    any.is::<VM>().then_some(view_model)
}

fn resolve_view_model<VM: IRoutableViewModel + 'static>() -> Option<RoutableViewModel> {
    Locator::current().get_service::<VM>().map(|view_model| view_model as RoutableViewModel)
}

/// An action that resolves and navigates to a view model of a type (the `TViewModel` of the
/// reference, a [`RoutableViewModelType`]).
///
/// Deviation (DEVIATIONS.md, Behaviors): the class is not generic; the view model type is given
/// when the action is created ([`new`](Self::new)), and markup does not create the action (it
/// has no type arguments for it).
#[repr(C)]
pub struct NavigateToAction {
    base: StyledElementAction,
    view_model_type: RoutableViewModelType,
}

ferro_class! {
    NavigateToAction: StyledElementAction, virtuals NavigateToActionImpl: StyledElementActionImpl {
        /// Resolves an instance of the view model type from the service locator.
        ///
        /// Returns the resolved view model instance or `None` if it cannot be created.
        fn resolve_view_model(this) -> Option<RoutableViewModel>;
    }
}

ferroui_base::ferro_impl_classes!(NavigateToAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl NavigateToAction {
        /// Identifies the `Router` property.
        pub fn router_property() -> StyledProperty<Option<Rc<RoutingState>>> {
            FerroProperty::register::<NavigateToAction, _>("Router", None)
        }
    }
}

impl NavigateToActionImpl for NavigateToAction {
    fn resolve_view_model(this: &Self) -> Option<RoutableViewModel> {
        this.view_model_type.resolve()
    }
}

impl StyledElementActionImpl for NavigateToAction {
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

        let _ = router.navigate().execute(Some(vm)).subscribe_ignore();
        result(true)
    }
}

impl NavigateToAction {
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
