//! Port of `Control/HideOnLostFocusBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl, FerroProperty, Nullable,
    Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::Cell;

/// Hides the target control when the associated control loses the focus.
#[repr(C)]
pub struct HideOnLostFocusBehavior {
    base: StyledElementBehaviorOfT,
    // The handler added to the associated control (the reference removes its delegate).
    lost_focus_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(HideOnLostFocusBehavior: StyledElementBehaviorOfT);
ferro_class_info!(HideOnLostFocusBehavior {
    new: HideOnLostFocusBehavior::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(HideOnLostFocusBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl HideOnLostFocusBehavior {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the control, so the
        /// value is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<HideOnLostFocusBehavior, _>("TargetControl", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for HideOnLostFocusBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::lost_focus_event(),
                move |_sender: &Interactive, _e: &FocusChangedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_lost_focus();
                    }
                },
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
                false,
            );
            this.lost_focus_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            if let Some(handler) = this.lost_focus_handler.take() {
                associated_object.remove_handler(InputElement::lost_focus_event(), handler);
            }
        }
    }
}

impl HideOnLostFocusBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            lost_focus_handler: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the control that is hidden. Markup resolves a name given for it to the named
    /// element. This is a property of the object model.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    fn associated_object_lost_focus(&self) {
        if let Some(target_control) = self.target_control() {
            target_control.set_is_visible(false);
        }
    }
}
