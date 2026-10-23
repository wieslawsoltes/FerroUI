//! What the generated view locator of ReactiveUI names (`generate_view_locator = true`), with the
//! feature `reactiveui`.
//!
//! The reference generates `ReactiveUI.IViewLocator` with its four `ResolveView` overloads and
//! assigns `IViewFor.ViewModel` of the view it creates. A view of ReactiveUI in FerroUI is a
//! shared trait object (`Rc<dyn IViewFor>`) and a control is one through an adapter: a control
//! class that states its view model implements [`IControlViewFor`], and [`ControlViewFor`] shows it
//! as a view. The generator discovers the classes that implement `IControlViewFor` (the automatic
//! `IViewFor<TViewModel>` mappings of the reference) and writes, for each of their mappings, a
//! factory of the view ([`ViewForFactory`]) beside the factory of the control.

pub use ferroui_reactiveui::{view_for_of, IViewFor, IViewForOf, IViewLocator, ViewModelType};
pub use ferroui_splat::IEnableLogger;
pub use ferroui_reactiveui_controls::{view_control, view_model_of_value, ControlViewFor, IControlViewFor};

use crate::{RuntimeType, TypeTable};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::Control;
use std::any::Any;
use std::rc::Rc;

/// Creates the view of a mapping as a view of ReactiveUI: the control of a class that implements
/// [`IControlViewFor`], shown by a [`ControlViewFor`] that keeps it alive.
pub type ViewForFactory = fn() -> Rc<dyn IViewFor>;

/// The views of ReactiveUI of the view model types whose view implements [`IControlViewFor`].
pub type ViewForTable = TypeTable<ViewForFactory>;

/// The view of ReactiveUI of a new control of the class `V` (`new View()` as `IViewFor`).
pub fn view_for<V: IControlViewFor>(control: Ref<V>) -> Rc<dyn IViewFor> {
    ControlViewFor::new(control)
}

/// The view model of ReactiveUI held by the value of a property (`object` as a view model).
pub fn view_model_of(value: &BoxedValue) -> Rc<dyn Any> {
    view_model_of_value(value)
}

/// The run-time type of a view model of ReactiveUI.
pub fn runtime_type_of(view_model: &Rc<dyn Any>) -> RuntimeType {
    RuntimeType::of_any(&**view_model)
}

/// Sets `view_model` as the view model of `view` (`view.ViewModel = instance`).
pub fn set_view_model(view: &Rc<dyn IViewFor>, view_model: Rc<dyn Any>) {
    view.set_view_model(Some(view_model));
}

/// The control of a view created by a [`ViewForFactory`].
pub fn control_of(view: &Rc<dyn IViewFor>) -> Option<Ref<Control>> {
    view_control(&**view)
}

/// Whether the contract `contract` is the default one: the generated map has no contracts, so a
/// resolution under any other contract finds no view (`contract is not null` returns `null`).
/// An empty contract is the default one, as in the view locator of ReactiveUI.
pub fn is_default_contract(contract: Option<&str>) -> bool {
    contract.is_none_or(str::is_empty)
}
