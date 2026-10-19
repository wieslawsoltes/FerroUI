//! `IViewFor` and `IViewFor<T>`.

use std::any::Any;
use std::rc::Rc;

/// A view of a view model of any type (`IViewFor`).
pub trait IViewFor {
    /// The view model (`ViewModel { get; }`).
    fn view_model_any(&self) -> Option<Rc<dyn Any>>;

    /// Sets the view model; one of another type sets none (`ViewModel { set; }`).
    fn set_view_model_any(&self, view_model: Option<Rc<dyn Any>>);
}

/// A view of a view model of the type `T` (`IViewFor<T>`).
pub trait IViewForOf<T>: IViewFor {
    /// The view model (`ViewModel { get; }`).
    fn view_model(&self) -> Option<Rc<T>>;

    /// Sets the view model (`ViewModel { set; }`).
    fn set_view_model(&self, view_model: Option<Rc<T>>);
}
