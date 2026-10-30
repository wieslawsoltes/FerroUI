//! Port of `Focus/FocusBehavior.cs`.

use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroProperty, Ref, StaticType, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Focuses the associated control when [`is_focused`](Self::is_focused) becomes true, and sets it
/// back to false when the control loses the focus.
#[repr(C)]
pub struct FocusBehavior {
    base: DisposingBehavior,
}

ferro_class!(FocusBehavior: DisposingBehavior);
ferro_class_info!(FocusBehavior { new: FocusBehavior::new });
ferroui_base::ferro_impl_classes!(FocusBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl FocusBehavior {
        /// Identifies the `IsFocused` property.
        pub fn is_focused_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<FocusBehavior, _>(
                "IsFocused",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for FocusBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            let source: &FerroObject = &associated_object;
            let associated_object_is_focused_observable_dispose =
                FerroObjectExtensions::get_observable(source, InputElement::is_focused_property()).subscribe_fn(
                    move |focused: bool| {
                        if !focused {
                            if let Some(behavior) = behavior.upgrade() {
                                behavior.set_current_value(FocusBehavior::is_focused_property(), false);
                            }
                        }
                    },
                );

            let behavior = this.to_ref().downgrade();
            let this_object: &FerroObject = this;
            let is_focused_observable_dispose =
                FerroObjectExtensions::get_observable(this_object, Self::is_focused_property()).subscribe_fn(
                    move |focused: bool| {
                        if focused {
                            let behavior = behavior.clone();
                            Dispatcher::ui_thread().post_local(
                                move || {
                                    if let Some(associated_object) = behavior
                                        .upgrade()
                                        .and_then(|behavior| behavior.associated_object_as::<Control>())
                                    {
                                        associated_object.focus();
                                    }
                                },
                                DispatcherPriority::DEFAULT,
                            );
                        }
                    },
                );

            DisposableAction::create(move || {
                associated_object_is_focused_observable_dispose.dispose();
                is_focused_observable_dispose.dispose();
            })
        }
    }
}

impl FocusBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets whether the associated control has the focus. This is a property of the
    /// object model, bound in both directions by default.
    pub fn is_focused(&self) -> bool {
        self.get_value(Self::is_focused_property())
    }

    pub fn set_is_focused(&self, value: bool) {
        self.set_value(Self::is_focused_property(), value)
    }
}
