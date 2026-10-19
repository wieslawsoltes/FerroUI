//! `IViewLocator` (ReactiveUI.Binding, `Interfaces/IViewLocator.cs`).

use super::{IViewFor, IViewForOf};
use ferroui_splat::{AppLocator, IEnableLogger, ReadonlyDependencyResolverExt};
use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;

/// Resolves the views of view models (`IViewLocator`).
///
/// A view model held as an `Rc<dyn Any>` is resolved by [`resolve_view`](Self::resolve_view),
/// which only asks what was registered for its type without the service locator, and by
/// [`resolve_view_unsafe`](Self::resolve_view_unsafe), which also asks the service locator. A
/// view model whose type is known is resolved by [`resolve_view_typed`](Self::resolve_view_typed),
/// which the generic [`resolve_view_of`](super::ViewLocatorMixins::resolve_view_of) calls.
///
/// The trait is object safe (the locator is a service, `Rc<dyn IViewLocator>`); it is `Any`, so
/// that [`ViewLocatorMixins`](super::ViewLocatorMixins) finds the
/// [`DefaultViewLocator`](crate::DefaultViewLocator) behind it.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the generic member `ResolveView<TViewModel>(TViewModel,
/// string?)` cannot be part of a trait object; it is [`resolve_view_typed`](Self::resolve_view_typed),
/// which takes the view model type-erased with a [`ViewModelType`] that stands for its
/// compile-time type.
pub trait IViewLocator: IEnableLogger + Any {
    /// Resolves the view of `view_model` under `contract` (`None` for the default view) through
    /// its compile-time type `view_model_type`, or `None` if there is no view
    /// (`ResolveView<TViewModel>(TViewModel, string?)`).
    fn resolve_view_typed(&self, view_model: Rc<dyn Any>, view_model_type: &ViewModelType, contract: Option<&str>) -> Option<Rc<dyn IViewFor>>;

    /// Resolves the view of `view_model` by its run-time type, without the service locator, or
    /// `None` if there is no view or no view model (`ResolveView(object?, string?)`).
    fn resolve_view(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>>;

    /// Resolves the view of `view_model` by its run-time type as
    /// [`resolve_view`](Self::resolve_view) does, then through the service locator
    /// (`ResolveViewUnsafe(object?, string?)`).
    fn resolve_view_unsafe(&self, view_model: Option<Rc<dyn Any>>, contract: Option<&str>) -> Option<Rc<dyn IViewFor>>;
}

/// The compile-time type of a view model, as a value (what upstream's `TViewModel` gives a generic
/// resolution): its `TypeId`, its name, and how to ask the service locator for its view
/// (`IViewFor<TViewModel>`, here `dyn IViewForOf<VM>`).
///
/// Not an upstream type: Rust cannot close `IViewFor<>` over a type known only at run time
/// (`MakeGenericType`), so the lookup is made where the type is known, by [`of`](Self::of).
#[derive(Clone, Copy)]
pub struct ViewModelType {
    type_id: TypeId,
    type_name: &'static str,
    lookup: fn(Option<&str>) -> Option<Rc<dyn IViewFor>>,
}

impl ViewModelType {
    /// The type `VM`.
    pub fn of<VM: 'static>() -> Self {
        Self { type_id: TypeId::of::<VM>(), type_name: std::any::type_name::<VM>(), lookup: service_view_for::<VM> }
    }

    /// The `TypeId` of the type.
    pub fn id(&self) -> TypeId {
        self.type_id
    }

    /// The name of the type (`Type.Name`, here the path of the type).
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    /// The last view registered in the service locator as `dyn IViewForOf<VM>` under `contract`
    /// (`None` and an empty contract are the default registration), or `None`
    /// (`AppLocator.Current.GetService<IViewFor<TViewModel>>(contract)`).
    pub fn service_view(&self, contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        (self.lookup)(contract.filter(|contract| !contract.is_empty()))
    }
}

impl fmt::Debug for ViewModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ViewModelType").field("type_name", &self.type_name).finish()
    }
}

impl PartialEq for ViewModelType {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}

impl Eq for ViewModelType {}

/// `AppLocator.Current.GetService<IViewFor<VM>>(contract)`, as a view.
fn service_view_for<VM: 'static>(contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
    AppLocator::current()
        .get_service_with_contract::<dyn IViewForOf<VM>>(contract)
        .map(|view| view as Rc<dyn IViewFor>)
}
