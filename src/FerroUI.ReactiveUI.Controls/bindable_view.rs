//! A view of markup as the view that the bindings of ReactiveUI take, and the paths from it to the
//! properties of its elements.
//!
//! Not an upstream file. Upstream's view is the control itself: `this.Bind(ViewModel, vm =>
//! vm.WorkItemText, v => v.WorkItemTextBox.Text)` binds the control, which implements `IViewFor<T>`
//! and raises `PropertyChanged` for its `ViewModel`. A control of the port is a handle of the object
//! model (`Ref<T>`), and the bindings of `ferroui-reactiveui` take a view shared as an `Rc` that
//! implements `IViewFor` and `INotifyPropertyChanged`.
//!
//! Deviation (DEVIATIONS.md, ReactiveUI controls): a view binds through a [`BindableView`] of its
//! control: the view of ReactiveUI that follows the `ViewModel` property of the control and raises
//! its change. The step from the view to a named element of its document is a step that holds the
//! element ([`element_property`], [`element_control`]), where upstream reads the generated field.

use ferroui_base::{BoxedValue, FerroPropertyChangedEventArgs, ObjectType, Ref, Upcast, WeakRef};
use ferroui_controls::Control;
use ferroui_reactiveui::{typed_view_for, IActivatableView, IViewFor, IViewForOf, SettableProperty};
use crate::{register_view_control, value_of_optional_view_model, view_model_of_value, IReactiveViewClass, IViewControl};
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyPath};
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

/// The name of the property of a view that holds its view model.
const VIEW_MODEL_PROPERTY: &str = "ViewModel";

/// A control of a reactive view class (a document whose root is `ReactiveUserControlBase` or
/// `ReactiveWindowBase`) as the view that the bindings of ReactiveUI take: an `IViewFor<VM>` over the
/// control's `ViewModel` property, which raises `PropertyChanged` for `ViewModel` when the property
/// changes.
///
/// It holds the control weakly, so that the activation of the control, which holds the view, does
/// not keep the control alive.
pub struct BindableView<C: IReactiveViewClass, VM: PartialEq + 'static> {
    control: WeakRef<C>,
    property_changed: Event<str>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    _view_model: PhantomData<fn() -> VM>,
}

impl<C: IReactiveViewClass, VM: PartialEq + 'static> BindableView<C, VM> {
    /// The view of `control`.
    pub fn new(control: &Ref<C>) -> Rc<Self> {
        register_view_control::<Self>();
        let view = Rc::new(Self {
            control: control.downgrade(),
            property_changed: Event::new(),
            subscription: RefCell::new(None),
            _view_model: PhantomData,
        });
        let weak = Rc::downgrade(&view);
        let element: &Control = Upcast::<Control>::upcast(&**control);
        let subscription = element.property_changed(move |args: &FerroPropertyChangedEventArgs<'_>| {
            if args.property() != C::view_model_property().as_property() {
                return;
            }
            if let Some(view) = weak.upgrade() {
                view.property_changed.raise(VIEW_MODEL_PROPERTY);
            }
        });
        *view.subscription.borrow_mut() = Some(subscription);
        view
    }

    /// The control, while it is alive.
    pub fn control(&self) -> Option<Ref<C>> {
        self.control.upgrade()
    }

    /// The view model the control holds, when it is a `VM` (`ViewModel`).
    pub fn view_model(&self) -> Option<Rc<VM>> {
        let control = self.control.upgrade()?;
        let element: &Control = Upcast::<Control>::upcast(&*control);
        let value = element.get_value(C::view_model_property())?;
        view_model_of_value(&value).downcast::<VM>().ok()
    }

    /// Sets the view model of the control (`ViewModel = value`).
    pub fn set_view_model(&self, view_model: Option<Rc<VM>>) {
        if let Some(control) = self.control.upgrade() {
            let element: &Control = Upcast::<Control>::upcast(&*control);
            element.set_value(C::view_model_property(), view_model.map(|view_model| view_model as BoxedValue));
        }
    }
}

impl<C: IReactiveViewClass, VM: PartialEq + 'static> Drop for BindableView<C, VM> {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.borrow_mut().take() {
            subscription.dispose();
        }
    }
}

impl<C: IReactiveViewClass, VM: PartialEq + 'static> INotifyPropertyChanged for BindableView<C, VM> {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl<C: IReactiveViewClass, VM: PartialEq + 'static> IViewControl for BindableView<C, VM> {
    fn view_control(&self) -> Option<Ref<Control>> {
        self.control.upgrade().map(|control| control.upcast())
    }
}

impl<C: IReactiveViewClass, VM: PartialEq + 'static> IActivatableView for BindableView<C, VM> {
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        Some(self)
    }

    fn as_notify_property_changed(self: Rc<Self>) -> Option<Rc<dyn INotifyPropertyChanged>> {
        Some(self)
    }
}

impl<C: IReactiveViewClass, VM: PartialEq + 'static> IViewFor for BindableView<C, VM> {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        let control = self.control.upgrade()?;
        let element: &Control = Upcast::<Control>::upcast(&*control);
        element.get_value(C::view_model_property()).as_ref().map(view_model_of_value)
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        if let Some(control) = self.control.upgrade() {
            let element: &Control = Upcast::<Control>::upcast(&*control);
            element.set_value(C::view_model_property(), value_of_optional_view_model(view_model));
        }
    }

    fn as_view_for_of(self: Rc<Self>) -> Option<Box<dyn Any>> {
        typed_view_for::<VM, Self>(self)
    }
}

impl<C: IReactiveViewClass, VM: PartialEq + 'static> IViewForOf<VM> for BindableView<C, VM> {
    fn view_model(&self) -> Option<Rc<VM>> {
        BindableView::view_model(self)
    }

    fn set_view_model(&self, view_model: Option<Rc<VM>>) {
        BindableView::set_view_model(self, view_model);
    }
}

/// The property `property` of the element `element` (named `name` in the document of the view), as
/// the side of a binding the binding writes with `set` (`v => v.Element.Property`).
pub fn element_property<V: 'static, T: ObjectType, TV: 'static>(
    name: &str,
    element: &Ref<T>,
    property: PropertyPath<Ref<T>, TV>,
    set: impl Fn(&Ref<T>, TV) + 'static,
) -> SettableProperty<Rc<V>, TV> {
    let (read, write) = (element.clone(), element.clone());
    let path = PropertyPath::<Rc<V>, Ref<T>>::value(name, move |_: &Rc<V>| read.clone()).then(property);
    SettableProperty::new(path, move |_: &Rc<V>, value: TV| set(&write, value))
}

/// [`element_property`] whose step to the element names the view as the owner of the element, so
/// that the binding hooks see the element and its property (the default item template of an items
/// control, `AutoDataTemplateBindingHook`).
pub fn owned_element_property<V: 'static, T: ObjectType, TV: 'static>(
    name: &str,
    element: &Ref<T>,
    property: PropertyPath<Ref<T>, TV>,
    set: impl Fn(&Ref<T>, TV) + 'static,
) -> SettableProperty<Rc<V>, TV> {
    let (read, write) = (element.clone(), element.clone());
    let path = PropertyPath::<Rc<V>, Ref<T>>::object_property(name, move |_: &Rc<V>| read.clone()).then(property);
    SettableProperty::new(path, move |_: &Rc<V>, value: TV| set(&write, value))
}

/// The element `element` (named `name` in the document of the view) as the control a command binds
/// to (`v => v.Element`).
pub fn element_control<V: 'static, T: ObjectType>(name: &str, element: &Ref<T>) -> PropertyPath<Rc<V>, Rc<Ref<T>>> {
    let element = element.clone();
    PropertyPath::<Rc<V>, Rc<Ref<T>>>::value(name, move |_: &Rc<V>| Rc::new(element.clone()))
}
