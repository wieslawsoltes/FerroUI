//! `ViewModelPropertySync` (`ViewModelPropertySync.cs`): keeps a view's view model property and
//! its data context in step.
//!
//! A view model reaches a view either through the view model property or through the data context,
//! and each has to follow the other. The two reactive bases cannot share this by inheritance
//! because one extends `UserControl` and the other `Window`.

use crate::reactive_view::{as_control, IReactiveViewClass, ReactiveViewFor};
use crate::view_model_value::{same_view_model, view_model_of_optional};
use ferroui_base::{BoxedValue, FerroObject, FerroObjectExtensions, FerroPropertyChangedEventArgs, Ref, StyledElement, StyledProperty};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{ViewForMixins, ViewModelValue};

/// Keeps a view's view model property and its data context in step (`ViewModelPropertySync`).
pub struct ViewModelPropertySync;

impl ViewModelPropertySync {
    /// Forwards the view's activation to its view model when the view model takes part in
    /// activation (`ForwardActivation`).
    ///
    /// The observable of the view model property tells ReactiveUI when the view model changes.
    /// The view activates itself through a view that refers to it weakly, so that the handlers of
    /// its activation do not keep it alive.
    pub fn forward_activation<C: IReactiveViewClass>(view: &Ref<C>) {
        let object: &FerroObject = as_control(&**view);
        let view_model_changed = FerroObjectExtensions::get_observable(object, C::view_model_property())
            .select(|value: Option<BoxedValue>| -> ViewModelValue { view_model_of_optional(value.as_ref()) });
        let activatable = ReactiveViewFor::weak(view);
        let _ = activatable.when_activated_empty_with_view_model_changed(view_model_changed);
    }

    /// Propagates a change between the view model property and the data context (`Apply`).
    ///
    /// `is_valid_view_model_value` decides whether an incoming data context may become the view
    /// model.
    pub fn apply(
        view: &StyledElement,
        change: &FerroPropertyChangedEventArgs<'_>,
        view_model_property: &'static StyledProperty<Option<BoxedValue>>,
        is_valid_view_model_value: impl Fn(Option<&BoxedValue>) -> bool,
    ) {
        if change.property() == StyledElement::data_context_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<BoxedValue>>();
            let view_model = view.get_value(view_model_property);
            if same_view_model(old_value.as_ref(), view_model.as_ref()) && is_valid_view_model_value(new_value.as_ref()) {
                view.set_current_value(view_model_property, new_value);
            }
        } else if change.property() == view_model_property.as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<BoxedValue>>();
            let data_context = view.data_context();
            if same_view_model(old_value.as_ref(), data_context.as_ref()) {
                view.set_current_value(StyledElement::data_context_property(), new_value);
            }
        }
    }
}

