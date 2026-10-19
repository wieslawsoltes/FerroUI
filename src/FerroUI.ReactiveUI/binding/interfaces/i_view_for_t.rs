//! `IViewFor<T>` (ReactiveUI.Binding, `Interfaces/IViewFor{T}.cs`), and the macro
//! [`view_for!`](crate::view_for) that writes the traits of a view.

use super::IViewFor;
use std::rc::Rc;

/// A view that displays a view model of the type `VM` (`IViewFor<T>`).
///
/// Its accessors have the names of those of [`IViewFor`], as upstream's `new T? ViewModel` hides
/// the untyped one: where both traits are in scope, a call on a generic view names the trait
/// (`IViewForOf::<VM>::view_model(&*view)`). The macro [`view_for!`](crate::view_for) also gives
/// the view inherent accessors, which a call on the concrete type finds first.
pub trait IViewForOf<VM: 'static>: IViewFor {
    /// The view model displayed by the view (the getter of `ViewModel`).
    fn view_model(&self) -> Option<Rc<VM>>;

    /// Sets the view model displayed by the view (the setter of `ViewModel`).
    fn set_view_model(&self, view_model: Option<Rc<VM>>);
}

/// Implements [`IActivatableView`](crate::binding::interfaces::IActivatableView),
/// [`IViewFor`](crate::binding::interfaces::IViewFor) and
/// [`IViewForOf<VM>`](crate::binding::interfaces::IViewForOf) for a view.
///
/// Not an upstream item: upstream's views write the two `ViewModel` properties by hand (the
/// untyped one casting to the typed one).
///
/// - `view_for!(View, ViewModel)`: the view has inherent accessors
///   `view_model(&self) -> Option<Rc<ViewModel>>` and `set_view_model(&self, Option<Rc<ViewModel>>)`,
///   written by hand or by [`reactive_properties!`](crate::reactive_properties) with
///   `#[reactive] view_model: Option<Rc<ViewModel>>`.
/// - `view_for!(View, ViewModel, field)`: the view holds its view model in
///   `field: RefCell<Option<Rc<ViewModel>>>`, and the macro writes the inherent accessors too.
/// - `view_for!(View: INotifyPropertyChanged, ViewModel)`: as the first form, for a view that
///   raises `PropertyChanged` (a reactive object): the property observation of its `ViewModel`
///   (the bindings and the activation of the view) follows the changes of its view model through
///   [`IActivatableView::as_notify_property_changed`](crate::binding::interfaces::IActivatableView::as_notify_property_changed).
///
/// The untyped setter panics when it is given a view model of another type, as the cast of
/// upstream's views throws an `InvalidCastException`.
///
/// ```
/// use ferroui_reactiveui::view_for;
/// use ferroui_reactiveui::binding::interfaces::IViewFor;
/// use std::any::Any;
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// struct TodoViewModel;
///
/// #[derive(Default)]
/// struct TodoView {
///     view_model: RefCell<Option<Rc<TodoViewModel>>>,
/// }
/// view_for!(TodoView, TodoViewModel, view_model);
///
/// let view = Rc::new(TodoView::default());
/// IViewFor::set_view_model(&*view, Some(Rc::new(TodoViewModel) as Rc<dyn Any>));
/// assert!(view.view_model().is_some());
/// ```
#[macro_export]
macro_rules! view_for {
    (@view_for $view:ty, $view_model:ty) => {
        impl $crate::binding::interfaces::IViewFor for $view {
            fn view_model(&self) -> ::core::option::Option<::std::rc::Rc<dyn ::core::any::Any>> {
                <$view>::view_model(self).map(|view_model| view_model as ::std::rc::Rc<dyn ::core::any::Any>)
            }

            fn set_view_model(&self, view_model: ::core::option::Option<::std::rc::Rc<dyn ::core::any::Any>>) {
                let typed = view_model.map(|view_model| match view_model.downcast::<$view_model>() {
                    ::core::result::Result::Ok(view_model) => view_model,
                    ::core::result::Result::Err(_) => ::core::panic!(
                        "InvalidCastException: the view model of {} is a {}",
                        ::core::any::type_name::<$view>(),
                        ::core::any::type_name::<$view_model>()
                    ),
                });
                <$view>::set_view_model(self, typed);
            }

            fn as_view_for_of(self: ::std::rc::Rc<Self>) -> ::core::option::Option<::std::boxed::Box<dyn ::core::any::Any>> {
                $crate::binding::interfaces::typed_view_for::<$view_model, $view>(self)
            }
        }

        impl $crate::binding::interfaces::IViewForOf<$view_model> for $view {
            fn view_model(&self) -> ::core::option::Option<::std::rc::Rc<$view_model>> {
                <$view>::view_model(self)
            }

            fn set_view_model(&self, view_model: ::core::option::Option<::std::rc::Rc<$view_model>>) {
                <$view>::set_view_model(self, view_model);
            }
        }
    };
    ($view:ty : INotifyPropertyChanged, $view_model:ty) => {
        impl $crate::binding::interfaces::IActivatableView for $view {
            fn as_view_for(&self) -> ::core::option::Option<&dyn $crate::binding::interfaces::IViewFor> {
                ::core::option::Option::Some(self)
            }

            fn as_notify_property_changed(
                self: ::std::rc::Rc<Self>,
            ) -> ::core::option::Option<::std::rc::Rc<dyn $crate::__private::INotifyPropertyChanged>> {
                ::core::option::Option::Some(self)
            }
        }

        $crate::view_for!(@view_for $view, $view_model);
    };
    ($view:ty, $view_model:ty, $field:ident) => {
        impl $view {
            /// The view model displayed by the view (`ViewModel`).
            #[allow(dead_code)]
            pub fn view_model(&self) -> ::core::option::Option<::std::rc::Rc<$view_model>> {
                self.$field.borrow().clone()
            }

            /// Sets the view model displayed by the view (`ViewModel`).
            #[allow(dead_code)]
            pub fn set_view_model(&self, view_model: ::core::option::Option<::std::rc::Rc<$view_model>>) {
                *self.$field.borrow_mut() = view_model;
            }
        }

        $crate::view_for!($view, $view_model);
    };
    ($view:ty, $view_model:ty) => {
        impl $crate::binding::interfaces::IActivatableView for $view {
            fn as_view_for(&self) -> ::core::option::Option<&dyn $crate::binding::interfaces::IViewFor> {
                ::core::option::Option::Some(self)
            }
        }

        $crate::view_for!(@view_for $view, $view_model);
    };
}
