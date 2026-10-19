//! The reactive views as views of ReactiveUI: the untyped view of a control with a `ViewModel`
//! property ([`ReactiveViewFor`], upstream's `IViewFor` on `ReactiveUserControlBase`,
//! `ReactiveWindowBase` and `ViewModelViewHost`) and the typed reactive view ([`ReactiveView`],
//! upstream's generic `ReactiveUserControl<TViewModel>` and `ReactiveWindow<TViewModel>`).
//!
//! Not an upstream file.
//!
//! Deviation (DEVIATIONS.md, ReactiveUI controls): a class of the object model is not generic
//! (`ferro_class!` gives each class one type descriptor), so upstream's generic controls are a
//! non-generic class (`ReactiveUserControlBase`, `ReactiveWindowBase`, which markup creates) and a
//! typed handle over an instance of it, [`ReactiveView`], aliased as
//! [`ReactiveUserControl<VM>`](crate::ReactiveUserControl) and
//! [`ReactiveWindow<VM>`](crate::ReactiveWindow). The handle restricts the view models the control
//! takes from its `DataContext` to `VM` (upstream's override of `IsValidViewModelValue`) by setting
//! a filter on the instance.

use crate::view_control::{register_view_control, IViewControl, ViewLink};
use crate::view_model_value::{is_view_model_of, register_view_model, value_of_optional_view_model, view_model_of_value};
use ferroui_base::{BoxedValue, IntoRef, ObjectType, Ref, StyledProperty, Upcast};
use ferroui_controls::Control;
use ferroui_reactiveui::{typed_view_for, IActivatableView, IViewFor, IViewForOf};
use std::any::Any;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;

/// The filter of the view models a reactive view takes from its `DataContext`
/// (`IsValidViewModelValue`).
pub type ViewModelFilter = fn(&BoxedValue) -> bool;

/// A control class with a `ViewModel` property of the untyped value of the property system.
pub trait IReactiveViewClass: ObjectType + Upcast<Control> {
    /// The `ViewModel` property of the class (`ViewModelProperty`).
    fn view_model_property() -> &'static StyledProperty<Option<BoxedValue>>;

    /// Whether `value` may become the view model of the control (`IsValidViewModelValue`).
    fn is_valid_view_model_value(&self, value: Option<&BoxedValue>) -> bool {
        let _ = value;
        true
    }
}

/// A reactive view class that a typed handle can restrict to one type of view model.
pub trait ITypedReactiveViewClass: IReactiveViewClass {
    /// A new instance of the class (`new ReactiveUserControl<TViewModel>()`).
    fn create() -> Ref<Self>;

    /// Restricts the view models of the instance to those `filter` accepts; `None` accepts any.
    fn set_view_model_filter(&self, filter: Option<ViewModelFilter>);
}

/// A control with a `ViewModel` property as an untyped view of ReactiveUI (`IViewFor`).
pub struct ReactiveViewFor<C: IReactiveViewClass> {
    view: ViewLink<C>,
}

impl<C: IReactiveViewClass> ReactiveViewFor<C> {
    /// The view of `control`, which it keeps alive.
    pub fn new(control: Ref<C>) -> Rc<Self> {
        Self::with_link(ViewLink::Strong(control))
    }

    /// The view of `control`, which it does not keep alive (the view a control activates itself
    /// with).
    pub fn weak(control: &Ref<C>) -> Rc<Self> {
        Self::with_link(ViewLink::Weak(control.downgrade()))
    }

    fn with_link(view: ViewLink<C>) -> Rc<Self> {
        register_view_control::<Self>();
        Rc::new(Self { view })
    }

    /// The control, while it is alive.
    pub fn control(&self) -> Option<Ref<C>> {
        self.view.get()
    }

    /// The view as `IViewFor`.
    pub fn into_view_for(self: Rc<Self>) -> Rc<dyn IViewFor> {
        self
    }
}

impl<C: IReactiveViewClass> IViewControl for ReactiveViewFor<C> {
    fn view_control(&self) -> Option<Ref<Control>> {
        self.view.get().map(|control| control.upcast())
    }
}

impl<C: IReactiveViewClass> IActivatableView for ReactiveViewFor<C> {
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        Some(self)
    }
}

impl<C: IReactiveViewClass> IViewFor for ReactiveViewFor<C> {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        let control = self.view.get()?;
        as_control(&*control).get_value(C::view_model_property()).as_ref().map(view_model_of_value)
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        if let Some(control) = self.view.get() {
            as_control(&*control).set_value(C::view_model_property(), value_of_optional_view_model(view_model));
        }
    }
}

/// The control of an instance of a control class (the class as its base `Control`).
pub(crate) fn as_control<C: Upcast<Control>>(control: &C) -> &Control {
    Upcast::<Control>::upcast(control)
}

/// The filter of a typed view: a null view model, or one of type `VM` (`value is null or
/// TViewModel`).
fn accepts_view_model_of<VM: 'static>(value: &BoxedValue) -> bool {
    is_view_model_of::<VM>(value)
}

/// A reactive view class instance typed by its view model (`ReactiveUserControl<TViewModel>`,
/// `ReactiveWindow<TViewModel>`): a handle that dereferences to the control and, shared as an
/// `Rc`, is its `IViewFor<TViewModel>`.
pub struct ReactiveView<C: ITypedReactiveViewClass, VM: PartialEq + 'static> {
    control: Ref<C>,
    _view_model: PhantomData<fn() -> VM>,
}

impl<C: ITypedReactiveViewClass, VM: PartialEq + 'static> Clone for ReactiveView<C, VM> {
    fn clone(&self) -> Self {
        Self { control: self.control.clone(), _view_model: PhantomData }
    }
}

impl<C: ITypedReactiveViewClass, VM: PartialEq + 'static> ReactiveView<C, VM> {
    /// A new control of the class, typed by `VM` (`new ReactiveUserControl<TViewModel>()`).
    pub fn new() -> Self {
        Self::from_ref(C::create())
    }

    /// `control` (an instance of the class or of a class derived from it, as one of markup), typed
    /// by `VM`: from now on it takes only view models of type `VM` from its `DataContext`.
    pub fn from_control(control: impl IntoRef<C>) -> Self {
        Self::from_ref(control.into_ref())
    }

    fn from_ref(control: Ref<C>) -> Self {
        register_view_control::<Self>();
        register_view_model::<VM>();
        control.set_view_model_filter(Some(accepts_view_model_of::<VM>));
        Self { control, _view_model: PhantomData }
    }

    /// The control.
    pub fn control(&self) -> &Ref<C> {
        &self.control
    }

    /// The view model (`ViewModel { get; }`): `None` when the control holds none, or holds a view
    /// model of another type (which only the untyped `IViewFor` can set).
    pub fn view_model(&self) -> Option<Rc<VM>> {
        let value = as_control(&*self.control).get_value(C::view_model_property())?;
        view_model_of_value(&value).downcast::<VM>().ok()
    }

    /// Sets the view model (`ViewModel { set; }`).
    pub fn set_view_model(&self, view_model: Option<Rc<VM>>) {
        as_control(&*self.control).set_value(C::view_model_property(), view_model.map(|view_model| view_model as BoxedValue));
    }

    /// The view as `IViewFor<TViewModel>`.
    pub fn into_view_for_of(self) -> Rc<dyn IViewForOf<VM>> {
        Rc::new(self)
    }

    /// The view as `IViewFor`.
    pub fn into_view_for(self) -> Rc<dyn IViewFor> {
        Rc::new(self)
    }
}

impl<C: ITypedReactiveViewClass, VM: PartialEq + 'static> Default for ReactiveView<C, VM> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ITypedReactiveViewClass, VM: PartialEq + 'static> Deref for ReactiveView<C, VM> {
    type Target = C;

    fn deref(&self) -> &C {
        &self.control
    }
}

impl<C: ITypedReactiveViewClass, VM: PartialEq + 'static> IViewControl for ReactiveView<C, VM> {
    fn view_control(&self) -> Option<Ref<Control>> {
        Some(self.control.clone().upcast())
    }
}

impl<C: ITypedReactiveViewClass, VM: PartialEq + 'static> IActivatableView for ReactiveView<C, VM> {
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        Some(self)
    }
}

impl<C: ITypedReactiveViewClass, VM: PartialEq + 'static> IViewFor for ReactiveView<C, VM> {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        ReactiveView::view_model(self).map(|view_model| view_model as Rc<dyn Any>)
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        let typed = view_model.map(|view_model| match view_model.downcast::<VM>() {
            Ok(view_model) => view_model,
            Err(_) => panic!(
                "InvalidCastException: the view model of {} is a {}",
                std::any::type_name::<C>(),
                std::any::type_name::<VM>()
            ),
        });
        ReactiveView::set_view_model(self, typed);
    }

    fn as_view_for_of(self: Rc<Self>) -> Option<Box<dyn Any>> {
        typed_view_for::<VM, Self>(self)
    }
}

impl<C: ITypedReactiveViewClass, VM: PartialEq + 'static> IViewForOf<VM> for ReactiveView<C, VM> {
    fn view_model(&self) -> Option<Rc<VM>> {
        ReactiveView::view_model(self)
    }

    fn set_view_model(&self, view_model: Option<Rc<VM>>) {
        ReactiveView::set_view_model(self, view_model);
    }
}
