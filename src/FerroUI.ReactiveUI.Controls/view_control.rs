//! The control behind a view of ReactiveUI.
//!
//! Not an upstream file. Upstream's views are controls that implement `IViewFor`; a host puts the
//! view it resolves into its `Content`, and the activation fetcher watches the view as a visual.
//!
//! Deviation (DEVIATIONS.md, ReactiveUI controls): a control is an object of the class model
//! (`Ref<T>`), and ReactiveUI's views are shared trait objects (`Rc<dyn IViewFor>`). A control is
//! seen as a view through a small adapter: [`ControlViewFor`] for any control class that states its
//! view model ([`IControlViewFor`]), and the adapters of the reactive views
//! ([`ReactiveViewFor`](crate::ReactiveViewFor), [`ReactiveView`](crate::ReactiveView)). An
//! adapter type registers itself on this thread ([`register_view_control`]), so that untyped code
//! finds the control behind any view ([`view_control`]), as upstream's `view as Control`.

use ferroui_base::{ObjectType, Ref, Upcast, WeakRef};
use ferroui_controls::Control;
use ferroui_reactiveui::{typed_view_for, IActivatableView, IViewFor, IViewForOf};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Finds the control behind a view of a registered adapter type.
type ControlOfView = fn(&dyn Any) -> Option<Ref<Control>>;

ferroui_base::ferro_thread_local! {
    /// The registered adapter types, by type.
    static VIEW_CONTROLS: RefCell<HashMap<TypeId, ControlOfView>> = RefCell::new(HashMap::new());
}

/// An adapter that shows a control as a view of ReactiveUI.
pub trait IViewControl: Any {
    /// The control behind the view, while it is alive.
    fn view_control(&self) -> Option<Ref<Control>>;
}

fn control_of_view<T: IViewControl>(view: &dyn Any) -> Option<Ref<Control>> {
    view.downcast_ref::<T>()?.view_control()
}

/// Registers the adapter type `T` on this thread, so that [`view_control`] finds the control of
/// its views. Idempotent; the adapters of this crate register themselves when they are created.
pub fn register_view_control<T: IViewControl>() {
    VIEW_CONTROLS.with(|types| {
        types.borrow_mut().entry(TypeId::of::<T>()).or_insert(control_of_view::<T>);
    });
}

/// The control behind a view (`view as Control`): the control of an adapter of a registered type,
/// `None` for any other view.
pub fn view_control(view: &dyn IActivatableView) -> Option<Ref<Control>> {
    let view: &dyn Any = view;
    let find = VIEW_CONTROLS.with(|types| types.borrow().get(&view.type_id()).copied())?;
    find(view)
}

/// How an adapter holds its control: strongly, for a view that is handed to others (a view a
/// locator creates), or weakly, for the view a control uses for itself (its own activation), so
/// that the control and its handlers do not keep each other alive.
pub enum ViewLink<C: ObjectType> {
    /// A strong reference.
    Strong(Ref<C>),
    /// A weak reference.
    Weak(WeakRef<C>),
}

impl<C: ObjectType> ViewLink<C> {
    /// The control, while it is alive.
    pub fn get(&self) -> Option<Ref<C>> {
        match self {
            ViewLink::Strong(control) => Some(control.clone()),
            ViewLink::Weak(control) => control.upgrade(),
        }
    }
}

impl<C: ObjectType> Clone for ViewLink<C> {
    fn clone(&self) -> Self {
        match self {
            ViewLink::Strong(control) => ViewLink::Strong(control.clone()),
            ViewLink::Weak(control) => ViewLink::Weak(control.clone()),
        }
    }
}

/// A control class that is a view of the view model type `ViewModel` (a control that implements
/// `IViewFor<TViewModel>` upstream, with a plain `ViewModel` property).
pub trait IControlViewFor: ObjectType + Upcast<Control> {
    /// The type of the view model.
    type ViewModel: PartialEq + 'static;

    /// The view model (`ViewModel { get; }`).
    fn view_model_of(&self) -> Option<Rc<Self::ViewModel>>;

    /// Sets the view model (`ViewModel { set; }`).
    fn set_view_model_of(&self, view_model: Option<Rc<Self::ViewModel>>);
}

/// A control of a class that implements [`IControlViewFor`], as a view of ReactiveUI
/// (`IViewFor<TViewModel>` on the control).
pub struct ControlViewFor<V: IControlViewFor> {
    view: ViewLink<V>,
}

impl<V: IControlViewFor> ControlViewFor<V> {
    /// The view of `control`, which it keeps alive.
    pub fn new(control: Ref<V>) -> Rc<Self> {
        Self::with_link(ViewLink::Strong(control))
    }

    /// The view of `control`, which it does not keep alive.
    pub fn weak(control: &Ref<V>) -> Rc<Self> {
        Self::with_link(ViewLink::Weak(control.downgrade()))
    }

    fn with_link(view: ViewLink<V>) -> Rc<Self> {
        register_view_control::<Self>();
        crate::register_view_model::<V::ViewModel>();
        Rc::new(Self { view })
    }

    /// The control, while it is alive.
    pub fn control(&self) -> Option<Ref<V>> {
        self.view.get()
    }

    /// The view as `IViewFor<TViewModel>`.
    pub fn into_view_for_of(self: Rc<Self>) -> Rc<dyn IViewForOf<V::ViewModel>> {
        self
    }
}

impl<V: IControlViewFor> IViewControl for ControlViewFor<V> {
    fn view_control(&self) -> Option<Ref<Control>> {
        self.view.get().map(|control| control.upcast())
    }
}

impl<V: IControlViewFor> IActivatableView for ControlViewFor<V> {
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        Some(self)
    }
}

impl<V: IControlViewFor> IViewFor for ControlViewFor<V> {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        let control = self.view.get()?;
        control.view_model_of().map(|view_model| view_model as Rc<dyn Any>)
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        let typed = view_model.map(|view_model| match view_model.downcast::<V::ViewModel>() {
            Ok(view_model) => view_model,
            Err(_) => panic!(
                "InvalidCastException: the view model of {} is a {}",
                std::any::type_name::<V>(),
                std::any::type_name::<V::ViewModel>()
            ),
        });
        if let Some(control) = self.view.get() {
            control.set_view_model_of(typed);
        }
    }

    fn as_view_for_of(self: Rc<Self>) -> Option<Box<dyn Any>> {
        typed_view_for::<V::ViewModel, Self>(self)
    }
}

impl<V: IControlViewFor> IViewForOf<V::ViewModel> for ControlViewFor<V> {
    fn view_model(&self) -> Option<Rc<V::ViewModel>> {
        self.view.get().and_then(|control| control.view_model_of())
    }

    fn set_view_model(&self, view_model: Option<Rc<V::ViewModel>>) {
        if let Some(control) = self.view.get() {
            control.set_view_model_of(view_model);
        }
    }
}
