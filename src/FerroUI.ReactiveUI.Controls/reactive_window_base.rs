//! `ReactiveWindowBase` (`ReactiveWindowBase.cs`): a non-generic ReactiveUI `Window` base that
//! owns the view model property.

use crate::reactive_view::{IReactiveViewClass, ITypedReactiveViewClass, ReactiveViewFor, ViewModelFilter};
use crate::view_model_property_sync::ViewModelPropertySync;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl};
use ferroui_reactiveui::IViewFor;
use std::cell::Cell;
use std::rc::Rc;

/// A non-generic ReactiveUI `Window` base that owns the view model property
/// (`ReactiveWindowBase`).
///
/// When the window is activated, the view model it shows is activated with it, provided the view
/// model implements `IActivatableViewModel`. The view model property and the data context follow
/// each other ([`ViewModelPropertySync`]).
///
/// The typed form is [`ReactiveWindow<VM>`](crate::ReactiveWindow); markup creates this class.
#[repr(C)]
pub struct ReactiveWindowBase {
    base: Window,
    /// The view models the window takes from its data context, when a typed handle restricted
    /// them (`IsValidViewModelValue` of `ReactiveWindow<TViewModel>`).
    view_model_filter: Cell<Option<ViewModelFilter>>,
}

ferro_class!(ReactiveWindowBase: Window);
ferro_class_info!(ReactiveWindowBase { new: ReactiveWindowBase::new });
ferro_impl_classes!(
    ReactiveWindowBase: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);

impl FerroObjectImpl for ReactiveWindowBase {
    /// The constructor body: the window's activation is forwarded to its view model
    /// (`ViewModelPropertySync.ForwardActivation`).
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        ViewModelPropertySync::forward_activation(&this.to_ref());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        ViewModelPropertySync::apply(this, change, Self::view_model_property(), |value| {
            IReactiveViewClass::is_valid_view_model_value(this, value)
        });
    }
}

ferro_properties! {
    impl ReactiveWindowBase {
        /// Identifies the `ViewModel` property (`ViewModelProperty`).
        pub fn view_model_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ReactiveWindowBase, _>("ViewModel", None)
        }
    }
}

impl ReactiveWindowBase {
    /// Field initialisation, over a window of the windowing platform.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered.
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()), view_model_filter: Cell::new(None) }
    }

    /// Creates the window.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The view model, as the untyped value of the property (`IViewFor.ViewModel { get; }`).
    pub fn view_model_value(&self) -> Option<BoxedValue> {
        self.get_value(Self::view_model_property())
    }

    /// Sets the view model, as the untyped value of the property (`IViewFor.ViewModel { set; }`).
    pub fn set_view_model_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::view_model_property(), value)
    }

    /// The window as a view of ReactiveUI (`(IViewFor)window`), which keeps it alive.
    pub fn view_for(this: &Ref<Self>) -> Rc<dyn IViewFor> {
        ReactiveViewFor::new(this.clone())
    }
}

impl IReactiveViewClass for ReactiveWindowBase {
    fn view_model_property() -> &'static StyledProperty<Option<BoxedValue>> {
        ReactiveWindowBase::view_model_property()
    }

    /// `IsValidViewModelValue`: any value, unless a typed handle restricted the view models.
    fn is_valid_view_model_value(&self, value: Option<&BoxedValue>) -> bool {
        match (value, self.view_model_filter.get()) {
            (Some(value), Some(filter)) => filter(value),
            _ => true,
        }
    }
}

impl ITypedReactiveViewClass for ReactiveWindowBase {
    fn create() -> Ref<Self> {
        Self::new()
    }

    fn set_view_model_filter(&self, filter: Option<ViewModelFilter>) {
        self.view_model_filter.set(filter);
    }
}
