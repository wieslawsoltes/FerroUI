//! `ReactiveUserControlBase` (`ReactiveUserControlBase.cs`): a non-generic ReactiveUI
//! `UserControl` base that owns the view model property.

use crate::reactive_view::{IReactiveViewClass, ITypedReactiveViewClass, ReactiveViewFor, ViewModelFilter};
use crate::view_model_property_sync::ViewModelPropertySync;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, UserControl};
use ferroui_reactiveui::IViewFor;
use std::cell::Cell;
use std::rc::Rc;

/// A non-generic ReactiveUI `UserControl` base that owns the view model property
/// (`ReactiveUserControlBase`).
///
/// When the control is activated, the view model it shows is activated with it, provided the view
/// model implements `IActivatableViewModel`. The view model property and the data context follow
/// each other ([`ViewModelPropertySync`]).
///
/// The typed form is [`ReactiveUserControl<VM>`](crate::ReactiveUserControl); markup creates this
/// class.
#[repr(C)]
pub struct ReactiveUserControlBase {
    base: UserControl,
    /// The view models the control takes from its data context, when a typed handle restricted
    /// them (`IsValidViewModelValue` of `ReactiveUserControl<TViewModel>`).
    view_model_filter: Cell<Option<ViewModelFilter>>,
}

ferro_class!(ReactiveUserControlBase: UserControl);
ferro_class_info!(ReactiveUserControlBase { new: ReactiveUserControlBase::new });
ferro_impl_classes!(
    ReactiveUserControlBase: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for ReactiveUserControlBase {
    /// The constructor body: the view's activation is forwarded to its view model
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
    impl ReactiveUserControlBase {
        /// Identifies the `ViewModel` property (`ViewModelProperty`).
        pub fn view_model_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ReactiveUserControlBase, _>("ViewModel", None)
        }
    }
}

impl ReactiveUserControlBase {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), view_model_filter: Cell::new(None) }
    }

    /// Creates the control.
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

    /// The control as a view of ReactiveUI (`(IViewFor)control`), which keeps it alive.
    pub fn view_for(this: &Ref<Self>) -> Rc<dyn IViewFor> {
        ReactiveViewFor::new(this.clone())
    }
}

impl IReactiveViewClass for ReactiveUserControlBase {
    fn view_model_property() -> &'static StyledProperty<Option<BoxedValue>> {
        ReactiveUserControlBase::view_model_property()
    }

    /// `IsValidViewModelValue`: any value, unless a typed handle restricted the view models.
    fn is_valid_view_model_value(&self, value: Option<&BoxedValue>) -> bool {
        match (value, self.view_model_filter.get()) {
            (Some(value), Some(filter)) => filter(value),
            _ => true,
        }
    }
}

impl ITypedReactiveViewClass for ReactiveUserControlBase {
    fn create() -> Ref<Self> {
        Self::new()
    }

    fn set_view_model_filter(&self, filter: Option<ViewModelFilter>) {
        self.view_model_filter.set(filter);
    }
}
