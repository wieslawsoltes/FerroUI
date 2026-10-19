//! The class of a document as the view ReactiveUI binds: [`CodeBehindView`].
//!
//! Not an upstream file. Upstream's `MainWindow` and `NugetDetailsView` derive from
//! `ReactiveWindow<TViewModel>` and `ReactiveUserControl<TViewModel>`, so each is its own
//! `IViewFor<TViewModel>`: its code-behind calls `this.WhenActivated(..)`, `this.Bind(..)`,
//! `this.OneWayBind(..)` and `this.BindCommand(..)`, and the expressions of the view start at the
//! class (`view => view.searchTextBox.Text`).
//!
//! Deviation (DEVIATIONS.md, samples): a class of FerroUI is a handle (`Ref<T>`) of the object
//! model, while the bindings of ReactiveUI take the view as an `Rc` of a type that implements
//! `IViewFor<TViewModel>` and raises `PropertyChanged` for its `ViewModel`. A
//! [`CodeBehindView`] is that view for one class: it reads and writes the `ViewModel` property of
//! the class (that of `ReactiveWindowBase` or `ReactiveUserControlBase`), raises `PropertyChanged`
//! when it changes, finds the class for the activation of the integration
//! ([`IViewControl`]), and holds the class weakly; the class holds its view. The named elements of
//! the class are reached through getters of the view, which the property paths of the bindings
//! start with (`prop!(MainWindowView => search_text_box)`).

use ferroui_base::{BoxedValue, FerroPropertyChangedEventArgs, ObjectType, Ref, StyledProperty, Upcast, WeakRef};
use ferroui_controls::Control;
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactiveui::{typed_view_for, IActivatableView, IReactiveBinding, IViewFor, IViewForOf};
use ferroui_reactiveui_controls::{register_view_control, register_view_model, view_model_of_value, IViewControl};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;

/// The view of the class `C` of a document for the view model type `VM`, as ReactiveUI binds it
/// (`IViewFor<TViewModel>` on the class upstream).
pub struct CodeBehindView<C: ObjectType + Upcast<Control>, VM: PartialEq + 'static> {
    control: WeakRef<C>,
    view_model_property: &'static StyledProperty<Option<BoxedValue>>,
    property_changed: Event<str>,
    _view_model: PhantomData<fn() -> VM>,
}

impl<C: ObjectType + Upcast<Control>, VM: PartialEq + 'static> CodeBehindView<C, VM> {
    /// The view of `control`, whose view model is the value of `view_model_property` (the
    /// `ViewModel` property of the reactive base class of `C`).
    ///
    /// The view raises `PropertyChanged` for `ViewModel` each time the property changes, so that
    /// the bindings follow the view model of the class, as upstream's `ViewModel` dependency
    /// property notifies them.
    pub fn new(control: &Ref<C>, view_model_property: &'static StyledProperty<Option<BoxedValue>>) -> Rc<Self> {
        register_view_control::<Self>();
        register_view_model::<VM>();
        let view = Rc::new(Self { control: control.downgrade(), view_model_property, property_changed: Event::new(), _view_model: PhantomData });

        let weak = Rc::downgrade(&view);
        // The class holds the handler for as long as it lives; the handler holds the view weakly.
        let _ = as_control(control).property_changed(move |change: &FerroPropertyChangedEventArgs<'_>| {
            if change.property() != view_model_property.as_property() {
                return;
            }
            if let Some(view) = weak.upgrade() {
                view.property_changed.raise("ViewModel");
            }
        });
        view
    }

    /// The class, while it is alive.
    pub fn try_control(&self) -> Option<Ref<C>> {
        self.control.upgrade()
    }

    /// The class.
    ///
    /// # Panics
    /// Panics when the class is gone: a view is used only while its class lives (the class holds
    /// it).
    pub fn control(&self) -> Ref<C> {
        self.control.upgrade().expect("the class of a view is alive while the view is used")
    }

    /// The view model of the class (`ViewModel { get; }`): `None` when it has none or one of
    /// another type.
    pub fn view_model(&self) -> Option<Rc<VM>> {
        let control = self.control.upgrade()?;
        let value = as_control(&control).get_value(self.view_model_property)?;
        view_model_of_value(&value).downcast::<VM>().ok()
    }

    /// Sets the view model of the class (`ViewModel { set; }`).
    pub fn set_view_model(&self, view_model: Option<Rc<VM>>) {
        if let Some(control) = self.control.upgrade() {
            as_control(&control).set_value(self.view_model_property, view_model.map(|view_model| view_model as BoxedValue));
        }
    }
}

/// The class as its base `Control`.
fn as_control<C: ObjectType + Upcast<Control>>(control: &Ref<C>) -> &Control {
    Upcast::<Control>::upcast(&**control)
}

impl<C: ObjectType + Upcast<Control>, VM: PartialEq + 'static> INotifyPropertyChanged for CodeBehindView<C, VM> {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl<C: ObjectType + Upcast<Control>, VM: PartialEq + 'static> IViewControl for CodeBehindView<C, VM> {
    fn view_control(&self) -> Option<Ref<Control>> {
        self.control.upgrade().map(|control| control.upcast())
    }
}

impl<C: ObjectType + Upcast<Control>, VM: PartialEq + 'static> IActivatableView for CodeBehindView<C, VM> {
    fn as_view_for(&self) -> Option<&dyn IViewFor> {
        Some(self)
    }

    fn as_notify_property_changed(self: Rc<Self>) -> Option<Rc<dyn INotifyPropertyChanged>> {
        Some(self)
    }
}

impl<C: ObjectType + Upcast<Control>, VM: PartialEq + 'static> IViewFor for CodeBehindView<C, VM> {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        Self::view_model(self).map(|view_model| view_model as Rc<dyn Any>)
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
        Self::set_view_model(self, typed);
    }

    fn as_view_for_of(self: Rc<Self>) -> Option<Box<dyn Any>> {
        typed_view_for::<VM, Self>(self)
    }
}

impl<C: ObjectType + Upcast<Control>, VM: PartialEq + 'static> IViewForOf<VM> for CodeBehindView<C, VM> {
    fn view_model(&self) -> Option<Rc<VM>> {
        Self::view_model(self)
    }

    fn set_view_model(&self, view_model: Option<Rc<VM>>) {
        Self::set_view_model(self, view_model);
    }
}

/// Adds a binding to the disposables of an activation (`.DisposeWith(disposableRegistration)`); a
/// binding that a hook refused is `None` and adds nothing.
pub fn dispose_with<V: IViewFor + ?Sized + 'static, T: 'static>(binding: Option<Rc<dyn IReactiveBinding<V, T>>>, disposables: &CompositeDisposable) {
    if let Some(binding) = binding {
        disposables.add(binding);
    }
}
