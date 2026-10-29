//! Port of `Control/HideOnKeyPressedBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl, FerroProperty, Nullable,
    Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::Cell;

/// Hides the target control when the key is pressed in the associated control.
#[repr(C)]
pub struct HideOnKeyPressedBehavior {
    base: StyledElementBehaviorOfT,
    // The handler added to the associated control (the reference removes its delegate).
    key_down_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(HideOnKeyPressedBehavior: StyledElementBehaviorOfT);
ferro_class_info!(HideOnKeyPressedBehavior {
    new: HideOnKeyPressedBehavior::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(HideOnKeyPressedBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl HideOnKeyPressedBehavior {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the control, so the
        /// value is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<HideOnKeyPressedBehavior, _>("TargetControl", None)
        }

        /// Identifies the `Key` property.
        pub fn key_property() -> StyledProperty<Key> {
            FerroProperty::register::<HideOnKeyPressedBehavior, _>("Key", Key::Escape)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for HideOnKeyPressedBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::key_down_event(),
                move |_sender: &Interactive, e: &KeyEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_key_down(e);
                    }
                },
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
                false,
            );
            this.key_down_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            if let Some(handler) = this.key_down_handler.take() {
                associated_object.remove_handler(InputElement::key_down_event(), handler);
            }
        }
    }
}

impl HideOnKeyPressedBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            key_down_handler: Cell::new(None),
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

    /// Gets or sets the key that hides the target control. This is a property of the object
    /// model.
    pub fn key(&self) -> Key {
        self.get_value(Self::key_property())
    }

    pub fn set_key(&self, value: Key) {
        self.set_value(Self::key_property(), value)
    }

    fn associated_object_key_down(&self, e: &KeyEventArgs) {
        if e.key == self.key() {
            if let Some(target_control) = self.target_control() {
                target_control.set_is_visible(false);
            }
        }
    }
}
