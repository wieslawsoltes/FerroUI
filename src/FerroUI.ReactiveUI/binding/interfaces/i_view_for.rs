//! `IViewFor` (ReactiveUI.Binding, `Interfaces/IViewFor.cs`).

use super::{IActivatableView, IViewForOf};
use std::any::Any;
use std::rc::Rc;

/// A view that displays a view model (`IViewFor`). A view is shared as an `Rc`.
///
/// This is the form of a view that does not name the type of its view model: the view model is an
/// `Rc<dyn Any>`. A view of a known view model also implements [`IViewForOf<VM>`], and the macro
/// [`view_for!`](crate::view_for) writes both traits (with [`IActivatableView`]) from the typed
/// accessors.
///
/// The trait is object safe, so that a view is held as `Rc<dyn IViewFor>`; it is `Any` (through
/// [`IActivatableView`]), so that `Rc<dyn IViewFor>` upcasts to `Rc<dyn Any>` and downcasts to the
/// type of the view.
pub trait IViewFor: IActivatableView {
    /// The view model displayed by the view (the getter of `ViewModel`).
    fn view_model(&self) -> Option<Rc<dyn Any>>;

    /// Sets the view model displayed by the view (the setter of `ViewModel`). A typed view panics
    /// when `view_model` is of another type than its own, as the cast of upstream's views throws.
    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>);

    /// The view as an `Rc<dyn IViewForOf<VM>>` in a `Box<dyn Any>`, for the type `VM` of view
    /// model it is typed for, or `None` for a view that names no type.
    ///
    /// Not an upstream member: upstream asks `view is IViewFor<TViewModel>` at run time, which
    /// Rust cannot ask of a trait object. [`view_for_of`] reads it; [`view_for!`](crate::view_for)
    /// writes it, and a view implementing the traits by hand returns
    /// [`typed_view_for`]`::<VM, Self>(self)`.
    // Deviation (DEVIATIONS.md, ReactiveUI): the run-time test `is IViewFor<TViewModel>` is this
    // method, which a typed view implements.
    fn as_view_for_of(self: Rc<Self>) -> Option<Box<dyn Any>> {
        None
    }
}

/// The value of [`IViewFor::as_view_for_of`] for a view typed for `VM`.
pub fn typed_view_for<VM: 'static, V: IViewForOf<VM> + 'static>(view: Rc<V>) -> Option<Box<dyn Any>> {
    Some(Box::new(view as Rc<dyn IViewForOf<VM>>))
}

/// The view as an `IViewForOf<VM>` when it is typed for `VM` (`view as IViewFor<TViewModel>`),
/// through [`IViewFor::as_view_for_of`].
pub fn view_for_of<VM: 'static>(view: Rc<dyn IViewFor>) -> Option<Rc<dyn IViewForOf<VM>>> {
    let typed = view.as_view_for_of()?;
    typed.downcast::<Rc<dyn IViewForOf<VM>>>().ok().map(|view| *view)
}

/// A view as an `Rc<dyn IViewFor>`, whether its type is a view (`Rc<MyView>`) or a trait object of
/// one (`Rc<dyn IViewForOf<VM>>`, `Rc<dyn IViewFor>`): the bound of the services a view mapping
/// takes from the service locator
/// ([`ViewMappingBuilder::map_from_service_locator`](crate::ViewMappingBuilder::map_from_service_locator)).
pub trait IntoViewFor {
    /// The view as an `Rc<dyn IViewFor>`.
    fn into_view_for(self: Rc<Self>) -> Rc<dyn IViewFor>;
}

impl<V: IViewFor> IntoViewFor for V {
    fn into_view_for(self: Rc<Self>) -> Rc<dyn IViewFor> {
        self
    }
}

impl IntoViewFor for dyn IViewFor {
    fn into_view_for(self: Rc<Self>) -> Rc<dyn IViewFor> {
        self
    }
}

impl<VM: 'static> IntoViewFor for dyn IViewForOf<VM> {
    fn into_view_for(self: Rc<Self>) -> Rc<dyn IViewFor> {
        self
    }
}
